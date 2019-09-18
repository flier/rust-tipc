//! Topology Server

use core::mem::{self, MaybeUninit};
use core::ops::Deref;
use core::time::Duration;

use std::io;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

use crate::{
    addr::{Scope, ServiceAddr, ServiceRange, SocketAddr},
    forward_raw_fd_traits, raw as ffi,
    sock::{self, IntoResult, Socket},
};

#[repr(transparent)]
#[derive(Debug)]
pub struct Server(Socket);

forward_raw_fd_traits!(Server => Socket);

impl Server {
    pub fn connect(scope: Scope) -> io::Result<Self> {
        let sock = sock::new(libc::SOCK_SEQPACKET)?;
        let addr = ServiceAddr::new(ffi::TIPC_TOP_SRV, ffi::TIPC_TOP_SRV);

        sock.connect((addr, scope))?;

        Ok(Server(sock))
    }

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
                socket: SocketAddr::new(evt.port.ref_, evt.port.node),
                available: evt.event == ffi::TIPC_PUBLISHED,
            })
        }
    }

    pub fn wait(service: ServiceAddr, scope: Scope, expire: Option<Duration>) -> io::Result<bool> {
        let s = Self::connect(scope)?;

        s.subscribe(service, false, expire)?;

        loop {
            let evt = s.recv()?;

            if let Scope::Node(node) = scope {
                if node.get() != evt.socket.node() {
                    continue;
                }
            }

            return Ok(evt.available);
        }
    }
}

#[derive(Debug)]
pub struct Event {
    pub service: ServiceAddr,
    pub socket: SocketAddr,
    pub available: bool,
}
