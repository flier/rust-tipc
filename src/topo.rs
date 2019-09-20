//! Topology Server

use core::mem::{self, MaybeUninit};
use core::ops::Deref;
use core::time::Duration;

use std::ffi::CStr;
use std::io;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

use failure::Fallible;

use crate::{
    addr::{Scope, ServiceAddr, ServiceRange, SocketAddr},
    forward_raw_fd_traits, raw as ffi,
    sock::{self, IntoResult, Socket},
    Instance,
};

pub type BearerId = u32;

pub fn connect(scope: Scope) -> io::Result<Server> {
    let sock = sock::new(libc::SOCK_SEQPACKET)?;
    let addr = ServiceAddr::new(ffi::TIPC_TOP_SRV, ffi::TIPC_TOP_SRV);

    sock.connect((addr, scope))?;

    Ok(Server(sock))
}

pub fn wait(service: ServiceAddr, scope: Scope, expire: Option<Duration>) -> io::Result<bool> {
    let srv = connect(scope)?;

    srv.subscribe(service, false, expire)?;

    loop {
        let evt = srv.recv()?;

        if let Scope::Node(node) = scope {
            if node.get() != evt.sock.node() {
                continue;
            }
        }

        return Ok(evt.available);
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct Server(Socket);

forward_raw_fd_traits!(Server => Socket);

impl Server {
    pub fn subscribe<T: Into<ServiceRange>>(
        &self,
        service_range: T,
        all: bool,
        expire: Option<Duration>,
    ) -> io::Result<()> {
        let subscr = ffi::tipc_subscr {
            seq: service_range.into().into(),
            timeout: expire.map_or(ffi::TIPC_WAIT_FOREVER as u32, |d| d.as_millis() as u32),
            filter: if all {
                ffi::TIPC_SUB_PORTS
            } else {
                ffi::TIPC_SUB_SERVICE
            },
            ..Default::default()
        };

        unsafe {
            libc::send(
                self.0.as_raw_fd(),
                &subscr as *const _ as *const _,
                mem::size_of::<ffi::tipc_subscr>(),
                0,
            )
        }
        .into_result()
        .and_then(|size| {
            if size == mem::size_of::<ffi::tipc_subscr>() {
                Ok(())
            } else {
                Err(io::Error::new(io::ErrorKind::Other, "subscribe failed"))
            }
        })
    }

    pub fn recv(&self) -> io::Result<Event> {
        let mut evt = MaybeUninit::<ffi::tipc_event>::zeroed();

        unsafe {
            libc::recv(
                self.0.as_raw_fd(),
                evt.as_mut_ptr() as *mut _,
                mem::size_of::<ffi::tipc_event>(),
                0,
            )
        }
        .into_result()
        .and_then(|size| {
            if size == mem::size_of::<ffi::tipc_event>() {
                Ok(())
            } else {
                Err(io::Error::new(io::ErrorKind::Other, "receive event failed"))
            }
        })?;

        let evt = unsafe { evt.assume_init() };

        if evt.event == ffi::TIPC_SUBSCR_TIMEOUT {
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "receive event timed out",
            ))
        } else {
            Ok(Event {
                service: ServiceAddr::new(evt.s.seq.type_, evt.found_lower),
                node: evt.port.node,
                sock: SocketAddr::new(evt.port.ref_, evt.port.node),
                available: evt.event == ffi::TIPC_PUBLISHED,
            })
        }
    }
}

#[derive(Debug)]
pub struct Event {
    pub service: ServiceAddr,
    pub node: Instance,
    pub sock: SocketAddr,
    pub available: bool,
}

pub struct Events<'a>(&'a Server);

impl<'a> Iterator for Events<'a> {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        self.0.recv().ok()
    }
}

impl<'a> IntoIterator for &'a Server {
    type Item = Event;
    type IntoIter = Events<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Events(self)
    }
}

pub fn neighbor_nodes(scope: Scope) -> io::Result<Nodes> {
    let srv = connect(scope)?;

    srv.subscribe(ServiceRange::with_range(ffi::TIPC_CFG_SRV, ..), true, None)?;

    Ok(Nodes(srv))
}

#[derive(Debug)]
pub struct Node {
    pub id: Instance,
    pub available: bool,
}

impl From<Event> for Node {
    fn from(event: Event) -> Self {
        Node {
            id: event.node,
            available: event.available,
        }
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct Nodes(Server);

forward_raw_fd_traits!(Nodes => Server);

impl Iterator for Nodes {
    type Item = Node;

    fn next(&mut self) -> Option<Node> {
        self.recv().ok()
    }
}

impl Nodes {
    pub fn recv(&self) -> io::Result<Node> {
        self.0.recv().map(Node::from)
    }
}

pub fn neighbor_links(scope: Scope) -> io::Result<Links> {
    let srv = connect(scope)?;

    srv.subscribe(
        ServiceRange::with_range(ffi::TIPC_LINK_STATE, ..),
        true,
        None,
    )?;

    Ok(Links(srv))
}

#[derive(Debug)]
pub struct Link {
    pub local: BearerId,
    pub remote: BearerId,
    pub neighbor: Instance,
    pub available: bool,
}

impl From<Event> for Link {
    fn from(event: Event) -> Self {
        Link {
            local: event.sock.port() & 0xFFFF,
            remote: (event.sock.port() >> 16) & 0xFFFF,
            neighbor: event.service.instance(),
            available: event.available,
        }
    }
}

impl Link {
    pub fn local_link(&self) -> Fallible<String> {
        link_name(self.neighbor, self.local)
    }

    pub fn remote_link(&self) -> Fallible<String> {
        link_name(self.neighbor, self.remote)
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct Links(Server);

forward_raw_fd_traits!(Links => Server);

impl Iterator for Links {
    type Item = Link;

    fn next(&mut self) -> Option<Link> {
        self.recv().ok()
    }
}

impl Links {
    pub fn recv(&self) -> io::Result<Link> {
        self.0.recv().map(Link::from)
    }
}

pub fn link_name(peer: Instance, bearer_id: BearerId) -> Fallible<String> {
    let srv = sock::rdm()?;
    let mut req = ffi::tipc_sioc_ln_req {
        peer,
        bearer_id,
        ..Default::default()
    };

    unsafe { libc::ioctl(srv.as_raw_fd(), u64::from(ffi::SIOCGETLINKNAME), &mut req) }
        .into_result()?;

    Ok(unsafe { CStr::from_ptr(req.linkname.as_ptr() as *const _) }
        .to_str()?
        .to_owned())
}
