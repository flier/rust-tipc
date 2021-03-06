use core::cmp::PartialEq;
use core::fmt;
use core::num::NonZeroU32;
use core::num::ParseIntError;
use core::ops::{Range, RangeFrom, RangeFull, RangeTo};
use core::str::FromStr;

use std::sync::Once;

use failure::Fail;

use crate::{ffi, sock};

pub const TIPC_ADDR_MCAST: i32 = 1;
pub const TIPC_SERVICE_RANGE: u8 = 1;
pub const TIPC_SERVICE_ADDR: u8 = 2;
pub const TIPC_SOCKET_ADDR: u8 = 3;

/// A Service Type number
pub type Type = u32;

/// A Service Instance number
pub type Instance = u32;

/// A physical entity within a network is identified internally by a TIPC Network Address.
#[repr(transparent)]
#[derive(Clone, Copy, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct NetworkAddr(u32);

impl From<u32> for NetworkAddr {
    fn from(n: u32) -> Self {
        NetworkAddr(n)
    }
}

impl From<NetworkAddr> for u32 {
    fn from(addr: NetworkAddr) -> Self {
        addr.0
    }
}

impl PartialEq<u32> for NetworkAddr {
    fn eq(&self, other: &u32) -> bool {
        self.0 == *other
    }
}

impl NetworkAddr {
    pub fn new(zone: u32, cluster: u32, node: u32) -> Self {
        NetworkAddr((zone << ffi::TIPC_ZONE_OFFSET) | (cluster << ffi::TIPC_CLUSTER_OFFSET) | node)
    }

    pub fn zone(self) -> u32 {
        self.0 >> ffi::TIPC_ZONE_OFFSET
    }

    pub fn cluster(self) -> u32 {
        (self.0 & ffi::TIPC_CLUSTER_MASK) >> ffi::TIPC_CLUSTER_OFFSET
    }

    pub fn node(self) -> u32 {
        self.0 & ffi::TIPC_NODE_MASK
    }
}

impl fmt::Display for NetworkAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<{}.{}.{}>", self.zone(), self.cluster(), self.node())
    }
}

macro_rules! addr {
    (
        $(#[$outer:meta])*
        pub struct $name:ident($raw:ident) {
            $(
                $(#[$attr:meta])*
                pub $prop:ident : $ty:ty = $field:ident,
            )*
        }

        $($tt:tt)*
    ) => {
        $(#[$outer])*
        #[repr(transparent)]
        #[derive(Clone, Copy, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name(ffi::$raw);

        impl $name {
            /// Construct a new address
            pub const fn new( $( $prop : $ty ),* ) -> Self {
                $name( ffi::$raw { $( $field : $prop ),* })
            }

            $(
                $(#[$attr])*
                pub fn $prop(&self) -> $ty {
                    (self.0).$field
                }
            )*
        }

        impl fmt::Debug for $name {
            fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                fmt.debug_struct(stringify!($name))
                $(
                    .field(stringify!($prop), &(self.0).$field)
                )*
                    .finish()
            }
        }

        impl From<( $( $ty ),* )> for $name {
            fn from( ( $( $field ),* ) : ( $( $ty ),* ) ) -> Self {
                Self( ffi::$raw { $( $field ),* } )
            }
        }

        impl From<ffi::$raw> for $name {
            fn from(addr: ffi::$raw) -> Self {
                Self(addr)
            }
        }

        impl From<$name> for ffi::$raw {
            fn from(addr: $name) -> Self {
                addr.0
            }
        }

        impl AsRef<ffi::$raw> for $name {
            fn as_ref(&self) -> & ffi::$raw {
                &self.0
            }
        }

        impl AsMut<ffi::$raw> for $name {
            fn as_mut(&mut self) -> &mut ffi::$raw {
                &mut self.0
            }
        }

        addr!{ $($tt)* }
    };
    () => {};
}

addr! {
    /// The address is a reference to a specific socket in the cluster.
    ///
    /// The address of this type can be used for connecting or for sending messages
    /// in the same way as service addresses can be used,
    /// but is only valid as long as long as the referenced socket exists.
    pub struct SocketAddr(tipc_portid) {
        /// port number
        pub port: u32 = ref_,

        /// node hash number
        pub node: u32 = node,
    }

    /// This address type consists of a 32 bit service type identifier and a 32 bit service instance identifier.
    ///
    /// The type identifier is typically determined and hard coded by the user application programmer,
    /// but its value may have to be coordinated with other applications which might be present in the same cluster.
    /// The instance identifier is often calculated by the program, based on application specific criteria.
    pub struct ServiceAddr(tipc_name) {
        /// type identifier
        pub ty: Type = type_,

        /// instance identifier
        pub instance: Instance = instance,
    }

    /// This address type represents a range of service addresses of the same type
    /// and with instances between a lower and an upper range limit.
    ///
    /// By binding a socket to this address type one can make it represent many instances,
    /// something which has proved useful in many cases.
    /// This address type is also used as multicast address.
    pub struct ServiceRange(tipc_name_seq) {
        /// type identifier
        pub ty: Type = type_,

        /// lower instance
        pub lower: Instance = lower,

        /// upper instance
        pub upper: Instance = upper,
    }
}

impl From<Type> for ServiceRange {
    fn from(ty: Type) -> Self {
        ServiceRange::with_range(ty, ..)
    }
}

impl From<ServiceAddr> for ServiceRange {
    fn from(service: ServiceAddr) -> Self {
        ServiceRange::with_range(service.ty(), service.instance())
    }
}

impl<T: ToInstanceRange> From<(Type, T)> for ServiceRange {
    fn from((ty, range): (Type, T)) -> Self {
        ServiceRange::with_range(ty, range)
    }
}

impl ServiceRange {
    pub const fn with_type(ty: Type) -> Self {
        ServiceRange(ffi::tipc_name_seq {
            type_: ty,
            lower: u32::min_value(),
            upper: u32::max_value(),
        })
    }

    pub fn with_range<T: ToInstanceRange>(ty: Type, range: T) -> Self {
        ServiceRange(ffi::tipc_name_seq {
            type_: ty,
            lower: range.lower(),
            upper: range.upper(),
        })
    }
}

impl fmt::Display for SocketAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0:{:010}@{:x}", self.port(), self.node())
    }
}

impl fmt::Display for ServiceAddr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}@{:x}", self.ty(), self.instance(), 0)
    }
}

impl fmt::Display for ServiceRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}:{}@{:x}", self.ty(), self.lower(), self.upper(), 0)
    }
}

/// An error which can be returned when parsing a TIPC address.
#[derive(Debug, Fail)]
pub enum AddrParseError {
    #[fail(display = "missing reference")]
    MissingRef,

    #[fail(display = "invalid reference, {}", _0)]
    InvalidRef(#[cause] ParseIntError),

    #[fail(display = "missing type")]
    MissingType,

    #[fail(display = "invalid type, {}", _0)]
    InvalidType(#[cause] ParseIntError),

    #[fail(display = "missing instance")]
    MissingInstance,

    #[fail(display = "invalid instance, {}", _0)]
    InvalidInstance(#[cause] ParseIntError),

    #[fail(display = "missing node")]
    MissingNode,

    #[fail(display = "invalid node, {}", _0)]
    InvalidNode(#[cause] ParseIntError),
}

impl FromStr for SocketAddr {
    type Err = AddrParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use AddrParseError::*;

        let mut iter = s.splitn(2, ':');
        let _ty = iter.next().ok_or(MissingType)?;
        let mut iter = iter.next().ok_or(MissingRef)?.splitn(2, '@');
        let ref_ = iter
            .next()
            .ok_or(MissingRef)
            .and_then(|s| u32::from_str_radix(s, 10).map_err(InvalidRef))?;
        let node = iter
            .next()
            .ok_or(MissingNode)
            .and_then(|s| u32::from_str_radix(s, 16).map_err(InvalidNode))?;

        Ok(SocketAddr(ffi::tipc_portid { ref_, node }))
    }
}

impl FromStr for ServiceAddr {
    type Err = AddrParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use AddrParseError::*;

        let mut iter = s.splitn(2, ':');
        let type_ = iter
            .next()
            .ok_or(MissingType)
            .and_then(|s| u32::from_str_radix(s, 10).map_err(InvalidType))?;

        let mut iter = iter.next().ok_or(MissingInstance)?.splitn(2, '@');
        let instance = iter
            .next()
            .ok_or(MissingInstance)
            .and_then(|s| u32::from_str_radix(s, 10).map_err(InvalidInstance))?;

        Ok(ServiceAddr(ffi::tipc_name { type_, instance }))
    }
}

impl FromStr for ServiceRange {
    type Err = AddrParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use AddrParseError::*;

        let mut iter = s.splitn(3, ':');
        let type_ = iter
            .next()
            .ok_or(MissingType)
            .and_then(|s| u32::from_str_radix(s, 10).map_err(InvalidType))?;
        let lower = iter
            .next()
            .ok_or(MissingInstance)
            .and_then(|s| u32::from_str_radix(s, 10).map_err(InvalidInstance))?;

        let mut iter = iter.next().ok_or(MissingInstance)?.splitn(2, '@');
        let upper = iter
            .next()
            .ok_or(MissingInstance)
            .and_then(|s| u32::from_str_radix(s, 10).map_err(InvalidInstance))?;

        Ok(ServiceRange(ffi::tipc_name_seq {
            type_,
            lower,
            upper,
        }))
    }
}

impl SocketAddr {
    /// A Service Scope indicator
    pub fn scope(self) -> Scope {
        NonZeroU32::new(self.node()).map_or(Scope::Global, Scope::Node)
    }
}

pub fn own_node() -> u32 {
    static mut OWN_NODE: u32 = 0;
    static OWN_NODE_INIT: Once = Once::new();

    unsafe {
        OWN_NODE_INIT.call_once(|| {
            OWN_NODE = sock::new(libc::SOCK_RDM)
                .and_then(|sock| sock.local_addr())
                .map(|addr| addr.node())
                .unwrap_or(0);
        });
        OWN_NODE
    }
}

impl From<SocketAddr> for ffi::sockaddr_tipc {
    fn from(addr: SocketAddr) -> ffi::sockaddr_tipc {
        let mut sa = ffi::sockaddr_tipc {
            family: libc::AF_TIPC as u16,
            addrtype: TIPC_SOCKET_ADDR,
            ..Default::default()
        };

        sa.addr.id = addr.into();
        sa
    }
}

impl From<ServiceAddr> for ffi::sockaddr_tipc {
    fn from(addr: ServiceAddr) -> ffi::sockaddr_tipc {
        (addr, Scope::Global).into()
    }
}

impl From<(ServiceAddr, Scope)> for ffi::sockaddr_tipc {
    fn from((addr, scope): (ServiceAddr, Scope)) -> ffi::sockaddr_tipc {
        let mut sa = ffi::sockaddr_tipc {
            family: libc::AF_TIPC as u16,
            addrtype: TIPC_SERVICE_ADDR,
            ..Default::default()
        };

        sa.addr.name.name = addr.into();
        sa.addr.name.domain = scope.into();
        sa
    }
}

impl From<ServiceRange> for ffi::sockaddr_tipc {
    fn from(addr: ServiceRange) -> ffi::sockaddr_tipc {
        (addr, Visibility::default()).into()
    }
}

impl From<(ServiceRange, Visibility)> for ffi::sockaddr_tipc {
    fn from((addr, visibility): (ServiceRange, Visibility)) -> ffi::sockaddr_tipc {
        let mut sa = ffi::sockaddr_tipc {
            family: libc::AF_TIPC as u16,
            addrtype: TIPC_SERVICE_RANGE,
            scope: visibility as i8,
            ..Default::default()
        };

        sa.addr.nameseq = addr.into();
        sa
    }
}

/// A trait for objects which can be converted or resolved to one or more `Instance` values.
pub trait ToInstanceRange {
    fn lower(&self) -> Instance;

    fn upper(&self) -> Instance;
}

impl ToInstanceRange for Instance {
    fn lower(&self) -> Instance {
        *self
    }

    fn upper(&self) -> Instance {
        *self
    }
}

impl ToInstanceRange for Range<Instance> {
    fn lower(&self) -> Instance {
        self.start
    }

    fn upper(&self) -> Instance {
        self.end
    }
}

impl ToInstanceRange for RangeTo<Instance> {
    fn lower(&self) -> Instance {
        u32::min_value()
    }

    fn upper(&self) -> Instance {
        self.end
    }
}

impl ToInstanceRange for RangeFrom<Instance> {
    fn lower(&self) -> Instance {
        self.start
    }

    fn upper(&self) -> Instance {
        u32::max_value()
    }
}

impl ToInstanceRange for RangeFull {
    fn lower(&self) -> Instance {
        u32::min_value()
    }

    fn upper(&self) -> Instance {
        u32::max_value()
    }
}

/// A service scope indicator.
#[derive(Clone, Copy, Debug, PartialEq, Hash)]
pub enum Scope {
    /// cluster global
    Global,
    /// node local
    Node(NonZeroU32),
}

impl From<Scope> for u32 {
    fn from(scope: Scope) -> u32 {
        match scope {
            Scope::Global => 0,
            Scope::Node(node) => node.get(),
        }
    }
}

impl From<u32> for Scope {
    fn from(node: u32) -> Self {
        Self::new(node)
    }
}

impl PartialEq<u32> for Scope {
    fn eq(&self, other: &u32) -> bool {
        match self {
            Scope::Global => 0u32 == *other,
            Scope::Node(instance) => instance.get() == *other,
        }
    }
}

impl Scope {
    /// Creates a new `Scope` with the given node.
    pub fn new(node: u32) -> Self {
        NonZeroU32::new(node).map_or(Scope::Global, Scope::Node)
    }

    /// Creates a new `Scope` with the own node.
    pub fn own_node() -> Self {
        Self::new(own_node())
    }

    /// Checks if a node is the own node.
    pub fn is_own_node(self) -> bool {
        match self {
            Scope::Node(instance) if instance.get() == own_node() => true,
            _ => false,
        }
    }

    /// Returns the `Visibility` level for self.
    pub fn visibility(self) -> Visibility {
        match self {
            Scope::Node(node) if node.get() == own_node() => Visibility::Node,
            _ => Visibility::default(),
        }
    }
}

/// The visibility scope.
#[repr(i8)]
#[derive(Clone, Copy, Debug, PartialEq, Hash)]
pub enum Visibility {
    /// Visibility within whole own zone
    Zone = ffi::TIPC_ZONE_SCOPE as i8,
    /// Visibility within whole own cluster
    Cluster = ffi::TIPC_CLUSTER_SCOPE as i8,
    /// Visibility limited to own node
    Node = ffi::TIPC_NODE_SCOPE as i8,
}

impl Default for Visibility {
    fn default() -> Self {
        Visibility::Cluster
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_addr() {
        assert_eq!(
            SocketAddr(ffi::tipc_portid {
                ref_: 123,
                node: 456
            }),
            SocketAddr::new(123, 456)
        );

        assert_eq!(SocketAddr::new(123, 456).to_string(), "0:0000000123@1c8");

        assert_eq!(
            "0:123@1c8".parse::<SocketAddr>().unwrap(),
            SocketAddr::new(123, 456)
        );
    }
}
