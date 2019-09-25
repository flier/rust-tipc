//! Topology Server

use core::mem::{self, MaybeUninit};
use core::ops::Deref;
use core::time::Duration;

use std::ffi::CStr;
use std::io;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

use bitflags::bitflags;
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

pub fn wait<A: Into<ServiceAddr>>(
    service: A,
    scope: Scope,
    expire: Option<Duration>,
) -> io::Result<bool> {
    let srv = connect(scope)?;

    srv.subscribe(service.into(), false, expire, 0)?;

    loop {
        let evt = srv.recv()?;

        if let Scope::Node(node) = scope {
            if node.get() != evt.sock().node() {
                continue;
            }
        }

        return Ok(evt.available());
    }
}

bitflags! {
    /// A bit field specifying how the topology service should act on the subscription.
    pub struct Filter: u32 {
        /// The subscriber only wants 'edge' events, i.e., a TIPC_PUBLISHED event
        /// when the first matching binding is encountered,
        /// and a TIPC_WITHDRAWN event when the last matching binding is removed from the binding table.
        /// Hence, this event type only informs if there exists any matching binding in the cluster at the moment.
        const SERVICE = ffi::TIPC_SUB_SERVICE;
        /// The subscriber wants an event for each matching update of the binding table.
        ///
        /// This way, it can keep track of each individual matching service binding that exists in the cluster.
        const PORTS = ffi::TIPC_SUB_PORTS;
        /// The subscriber doesn't want any more events for this service range, i.e., the subscription is canceled.
        ///
        /// Apart from the 'cancel' bit, this subscription must be a copy of the original subscription request.
        const CANCEL = ffi::TIPC_SUB_CANCEL;
    }
}

#[derive(Debug)]
pub struct Subscription {
    /// The service address or range of interest.
    ///
    /// If only a single service address is tracked the upper and lower fields are set to be equal.
    pub service: ServiceRange,
    /// A value specifying, in milliseconds, the life time of the subscription.
    ///
    /// If this field is set to `None` the subscription will never expire (TIPC_WAIT_FOREVER).
    pub timeout: Option<Duration>,
    /// A bit field specifying how the topology service should act on the subscription.
    pub filter: Filter,
    /// A 64 bit field which is set and used at the subscriber's discretion.
    pub userdata: u64,
}

impl From<Subscription> for ffi::tipc_subscr {
    fn from(sub: Subscription) -> Self {
        ffi::tipc_subscr {
            seq: sub.service.into(),
            timeout: sub
                .timeout
                .map_or(ffi::TIPC_WAIT_FOREVER as u32, |t| t.as_millis() as u32),
            filter: sub.filter.bits(),
            usr_handle: unsafe { mem::transmute(sub.userdata.to_ne_bytes()) },
        }
    }
}

impl From<ffi::tipc_subscr> for Subscription {
    fn from(sub: ffi::tipc_subscr) -> Self {
        Subscription {
            service: sub.seq.into(),
            timeout: if sub.timeout == 0 {
                None
            } else {
                Some(Duration::from_millis(u64::from(sub.timeout)))
            },
            filter: Filter::from_bits_truncate(sub.filter),
            userdata: u64::from_ne_bytes(unsafe { mem::transmute(sub.usr_handle) }),
        }
    }
}

#[repr(transparent)]
#[derive(Debug)]
pub struct Server(Socket);

forward_raw_fd_traits!(Server => Socket);

impl Server {
    pub fn subscribe<A: Into<ServiceRange>>(
        &self,
        service: A,
        all: bool,
        timeout: Option<Duration>,
        userdata: u64,
    ) -> io::Result<()> {
        let subscr: ffi::tipc_subscr = Subscription {
            service: service.into(),
            timeout,
            filter: if all { Filter::PORTS } else { Filter::SERVICE },
            userdata,
        }
        .into();

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

        match evt.event {
            ffi::TIPC_SUBSCR_TIMEOUT => {
                // The subscription expired, as specified by the given timeout value, and has been removed.
                Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "receive event timed out",
                ))
            }
            ffi::TIPC_PUBLISHED | ffi::TIPC_WITHDRAWN => {
                let service = ServiceRange::new(evt.s.seq.type_, evt.found_lower, evt.found_upper);
                let sock = evt.port.into();
                let subscription = evt.s.into();

                if evt.event == ffi::TIPC_PUBLISHED {
                    Ok(Event::Published {
                        service,
                        sock,
                        subscription,
                    })
                } else {
                    Ok(Event::Withdrawn {
                        service,
                        sock,
                        subscription,
                    })
                }
            }
            _ => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("unexpect event: {:?}", evt),
            )),
        }
    }
}

#[derive(Debug)]
pub enum Event {
    /// A matching binding was found in the binding table.
    Published {
        /// Describes the matching binding's range.
        service: ServiceRange,
        /// The socket address of the matching socket.
        sock: SocketAddr,
        /// The original subscription request.
        subscription: Subscription,
    },
    /// A matching binding was removed from the binding table.
    Withdrawn {
        /// Describes the matching binding's range.
        service: ServiceRange,
        /// The socket address of the matching socket.
        sock: SocketAddr,
        /// The original subscription request.
        subscription: Subscription,
    },
}

impl Event {
    pub fn available(&self) -> bool {
        if let Event::Published { .. } = self {
            true
        } else {
            false
        }
    }

    pub fn service(&self) -> ServiceRange {
        match self {
            Event::Published { service, .. } | Event::Withdrawn { service, .. } => *service,
        }
    }

    pub fn sock(&self) -> SocketAddr {
        match self {
            Event::Published { sock, .. } | Event::Withdrawn { sock, .. } => *sock,
        }
    }

    pub fn subscription(&self) -> &Subscription {
        match self {
            Event::Published { subscription, .. } | Event::Withdrawn { subscription, .. } => {
                subscription
            }
        }
    }
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

    srv.subscribe(ffi::TIPC_CFG_SRV, true, None, 0)?;

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
            id: event.sock().node(),
            available: event.available(),
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

    srv.subscribe(ffi::TIPC_LINK_STATE, true, None, 0)?;

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
            local: event.sock().port() & 0xFFFF,
            remote: (event.sock().port() >> 16) & 0xFFFF,
            neighbor: event.service().lower(),
            available: event.available(),
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
