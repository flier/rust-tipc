#![cfg(any(target_os = "linux", feature = "doc"))]

mod addr;
mod sock;
mod topo;

#[allow(non_camel_case_types, dead_code, non_snake_case)]
mod raw;

pub use addr::{
    AddrParseError, Instance, Scope, ServiceAddr, ServiceRange, SocketAddr, ToInstanceRange, Type,
    Visibility,
};
pub use sock::{
    dgram, rdm, seq_packet, stream, Builder, Datagram, Importance, Listener, Recv, Stream,
};
pub use topo::Server;
