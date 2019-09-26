#![cfg(any(target_os = "linux", feature = "doc"))]

mod addr;
mod sock;
pub mod topo;

#[allow(
    non_camel_case_types,
    dead_code,
    non_snake_case,
    clippy::unreadable_literal
)]
mod raw;

#[doc(hidden)]
pub mod ffi {
    pub use crate::raw::*;

    pub const TIPC_NODEID_LEN: usize = 16;

    pub const SIOCGETNODEID: u32 = SIOCGETLINKNAME + 1;

    #[repr(C)]
    #[derive(Debug, Default, Copy, Clone, Hash, PartialOrd, Ord, PartialEq, Eq)]
    pub struct tipc_sioc_nodeid_req {
        pub peer: u32,
        pub node_id: [u8; TIPC_NODEID_LEN],
    }
}

pub use addr::{
    AddrParseError, Instance, NetworkAddr, Scope, ServiceAddr, ServiceRange, SocketAddr,
    ToInstanceRange, Type, Visibility,
};
pub use sock::{
    bind, connect, connect_timeout, datagram, rdm, seq_packet, stream, Bindable, Bound, Buildable,
    Builder, Connectable, Connected, Datagram, Group, Importance, Incoming, Join, Listener, Recv,
    RecvMsg, Rejected, Send, SeqPacket, Socket, Stream, ToServiceAddrs, ToServiceRanges,
    ToSocketAddrs, Wrapped,
};
