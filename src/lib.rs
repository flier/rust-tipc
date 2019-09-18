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

pub use addr::{
    AddrParseError, Instance, Scope, ServiceAddr, ServiceRange, SocketAddr, ToInstanceRange, Type,
    Visibility,
};
pub use sock::{
    datagram, rdm, seq_packet, stream, Builder, Datagram, Importance, Incoming, IntoConnectAddr,
    IntoSendToAddr, Listener, Rejected, SeqPacket, Stream,
};
