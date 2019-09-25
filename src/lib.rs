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
}
pub use addr::{
    own_node, AddrParseError, Instance, Scope, ServiceAddr, ServiceRange, SocketAddr,
    ToInstanceRange, Type, Visibility,
};
pub use sock::{
    bind, connect, connect_timeout, datagram, rdm, seq_packet, stream, Bindable, Bound, Buildable,
    Builder, Connectable, Connected, Datagram, Group, Importance, Incoming, Join, Listener, Recv,
    RecvMsg, Rejected, Send, SeqPacket, Socket, Stream, ToBindAddr, ToConnectAddr, ToSendToAddr,
};
