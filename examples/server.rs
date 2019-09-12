use std::convert::{TryFrom, TryInto};
use std::io;

use tipc::{self, Builder, Datagram, ServiceRange, Stream, Type, Visibility};

const RDM_SRV_TYPE: Type = 18888;
const STREAM_SRV_TYPE: Type = 17777;
const SEQPKT_SRV_TYPE: Type = 16666;

fn bind_service<T>(builder: Builder, ty: Type, name: &str) -> io::Result<T>
where
    T: TryFrom<Builder, Error = io::Error>,
{
    let addr = ServiceRange::with_range(ty, ..);
    let s = builder.bind(addr, Visibility::Cluster)?;

    println!(
        "Bound {} socket {} to {}",
        name,
        s.local_addr().unwrap(),
        addr
    );

    builder.try_into()
}

fn main() -> io::Result<()> {
    println!("****** TIPC C API Demo Server Started ******\n");

    let rdm: Datagram = bind_service(tipc::rdm()?, RDM_SRV_TYPE, "RDM")?;
    let stream: Stream = bind_service(tipc::stream()?, STREAM_SRV_TYPE, "STREAM")?;
    let pkt: Datagram = bind_service(tipc::seq_packet()?, SEQPKT_SRV_TYPE, "SEQPACKET")?;

    Ok(())
}
