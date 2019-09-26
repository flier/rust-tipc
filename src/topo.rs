//! The TIPC internal topology service.

use core::mem::{self, MaybeUninit};
use core::ops::Deref;
use core::time::Duration;

use std::io;
use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};

use crate::{
    addr::{Scope, ServiceAddr, ServiceRange, SocketAddr},
    ffi, impl_raw_fd_traits,
    sock::{self, BearerId, IntoResult, Socket},
    Instance,
};

/// Connects to the TIPC internal topology service.
pub fn connect(scope: Scope) -> io::Result<Server> {
    let sock = sock::new(libc::SOCK_SEQPACKET)?;
    let addr = ServiceAddr::new(ffi::TIPC_TOP_SRV, ffi::TIPC_TOP_SRV);

    sock.connect((addr, scope))?;

    Ok(Server(sock))
}

/// Waits the service ready.
pub fn wait<A: Into<ServiceAddr>>(
    service: A,
    scope: Scope,
    timeout: Option<Duration>,
) -> io::Result<bool> {
    let srv = connect(scope)?;

    srv.subscribe(Subscription {
        service: service.into().into(),
        filter: Filter::Edge,
        timeout,
        userdata: 0,
    })?;

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

/// specifying how the topology service should act on the subscription.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Filter {
    /// The subscriber wants an event for each matching update of the binding table.
    ///
    /// This way, it can keep track of each individual matching service binding that exists in the cluster.
    All = ffi::TIPC_SUB_PORTS,
    /// The subscriber only wants 'edge' events, i.e., a TIPC_PUBLISHED event
    /// when the first matching binding is encountered,
    /// and a TIPC_WITHDRAWN event when the last matching binding is removed from the binding table.
    /// Hence, this event type only informs if there exists any matching binding in the cluster at the moment.
    Edge = ffi::TIPC_SUB_SERVICE,
}

/// The topology service subscription.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

/// Subscription for neighbor nodes.
pub const NEIGHBOR_NODES: Subscription = Subscription {
    service: ServiceRange::with_type(ffi::TIPC_CFG_SRV),
    filter: Filter::All,
    timeout: None,
    userdata: 0,
};

/// Subscription for neighbor links.
pub const NEIGHBOR_LINKS: Subscription = Subscription {
    service: ServiceRange::with_type(ffi::TIPC_LINK_STATE),
    filter: Filter::All,
    timeout: None,
    userdata: 0,
};

impl Subscription {
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn all(mut self) -> Self {
        self.filter = Filter::All;
        self
    }

    pub fn edge(mut self) -> Self {
        self.filter = Filter::Edge;
        self
    }

    pub fn userdata(mut self, userdata: u64) -> Self {
        self.userdata = userdata;
        self
    }
}

impl<T> From<T> for Subscription
where
    T: Into<ServiceRange>,
{
    fn from(service: T) -> Self {
        Subscription {
            service: service.into(),
            timeout: None,
            filter: Filter::Edge,
            userdata: 0,
        }
    }
}

impl From<Subscription> for ffi::tipc_subscr {
    fn from(sub: Subscription) -> Self {
        ffi::tipc_subscr {
            seq: sub.service.into(),
            timeout: sub
                .timeout
                .map_or(ffi::TIPC_WAIT_FOREVER as u32, |t| t.as_millis() as u32),
            filter: sub.filter as u32,
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
            filter: if (sub.filter & ffi::TIPC_SUB_PORTS) == ffi::TIPC_SUB_PORTS {
                Filter::All
            } else {
                Filter::Edge
            },
            userdata: u64::from_ne_bytes(unsafe { mem::transmute(sub.usr_handle) }),
        }
    }
}

/// The node topology server,
#[repr(transparent)]
#[derive(Debug)]
pub struct Server(Socket);

impl_raw_fd_traits!(Server);

impl Server {
    /// Returns the address of the local half of this TIPC socket.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }

    /// The subscriber wants `All` or `Edge` event for each matching update of the binding table.
    pub fn subscribe<T: Into<Subscription>>(&self, sub: T) -> io::Result<Subscription> {
        let sub = sub.into();
        let subscr: ffi::tipc_subscr = sub.into();

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
                Ok(sub)
            } else {
                Err(io::Error::new(io::ErrorKind::Other, "subscribe failed"))
            }
        })
    }

    /// The subscriber doesn't want any more events for this service range.
    pub fn unsubscribe<T: Into<Subscription>>(&self, sub: T) -> io::Result<()> {
        let mut subscr: ffi::tipc_subscr = sub.into().into();

        subscr.filter = ffi::TIPC_SUB_CANCEL;

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
                Err(io::Error::new(io::ErrorKind::Other, "unsubscribe failed"))
            }
        })
    }

    /// Receives events for this service range.
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

/// The service event.
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

impl Deref for Event {
    type Target = Subscription;

    fn deref(&self) -> &Self::Target {
        match self {
            Event::Published { subscription, .. } | Event::Withdrawn { subscription, .. } => {
                subscription
            }
        }
    }
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

/// An iterator over the events.
#[repr(transparent)]
#[derive(Debug)]
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

/// Subscribe events for neighbor nodes.
pub fn neighbor_nodes(scope: Scope) -> io::Result<Nodes> {
    let srv = connect(scope)?;

    srv.subscribe(NEIGHBOR_NODES)?;

    Ok(Nodes(srv))
}

/// The node event.
#[derive(Debug)]
pub enum Node {
    Up(Instance),
    Down(Instance),
}

impl From<Event> for Node {
    fn from(event: Event) -> Self {
        let node = event.sock().node();

        if event.available() {
            Node::Up(node)
        } else {
            Node::Down(node)
        }
    }
}

impl Node {
    /// The node is available.
    pub fn available(&self) -> bool {
        match self {
            Node::Up(_) => true,
            Node::Down(_) => false,
        }
    }

    /// The node instance.
    pub fn instance(&self) -> Instance {
        match *self {
            Node::Up(instance) | Node::Down(instance) => instance,
        }
    }
}

/// An iterator over the node events.
#[repr(transparent)]
#[derive(Debug)]
pub struct Nodes(Server);

impl_raw_fd_traits! { Nodes(Server) }

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

/// Subscribe events for neighbor links.
pub fn neighbor_links(scope: Scope) -> io::Result<Links> {
    let srv = connect(scope)?;

    srv.subscribe(NEIGHBOR_LINKS)?;

    Ok(Links(srv))
}

/// The link event.
#[derive(Debug)]
pub enum Link {
    Up {
        local: BearerId,
        peer: BearerId,
        neighbor: Instance,
    },
    Down {
        local: BearerId,
        peer: BearerId,
        neighbor: Instance,
    },
}

impl From<Event> for Link {
    fn from(event: Event) -> Self {
        let port = event.sock().port();
        let local = port & 0xFFFF;
        let peer = (port >> 16) & 0xFFFF;
        let neighbor = event.service().lower();

        if event.available() {
            Link::Up {
                local,
                peer,
                neighbor,
            }
        } else {
            Link::Down {
                local,
                peer,
                neighbor,
            }
        }
    }
}

impl Link {
    /// The link is available.
    pub fn available(&self) -> bool {
        match self {
            Link::Up { .. } => true,
            Link::Down { .. } => false,
        }
    }

    /// The local link bearer id.
    pub fn local_bearer_id(&self) -> BearerId {
        match *self {
            Link::Up { local, .. } | Link::Down { local, .. } => local,
        }
    }

    /// The peer link bearer id.
    pub fn peer_bearer_id(&self) -> BearerId {
        match *self {
            Link::Up { peer, .. } | Link::Down { peer, .. } => peer,
        }
    }

    /// The local link name.
    pub fn local_link_name(&self) -> io::Result<String> {
        match *self {
            Link::Up {
                local, neighbor, ..
            }
            | Link::Down {
                local, neighbor, ..
            } => link_name(neighbor, local),
        }
    }

    /// The peer link name.
    pub fn peer_link_name(&self) -> io::Result<String> {
        match *self {
            Link::Up { peer, neighbor, .. } | Link::Down { peer, neighbor, .. } => {
                link_name(neighbor, peer)
            }
        }
    }
}

/// An iterator over the link events.
#[repr(transparent)]
#[derive(Debug)]
pub struct Links(Server);

impl_raw_fd_traits! { Links(Server) }

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

/// Retrieve a link name
pub fn link_name(peer: Instance, bearer_id: BearerId) -> io::Result<String> {
    sock::rdm()?.as_ref().link_name(peer, bearer_id)
}
