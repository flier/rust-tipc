use std::io;

use tipc::{Instance, Scope, Server, ServiceAddr, Type};

const RDM_SRV_TYPE: Type = 18888;
const STREAM_SRV_TYPE: Type = 17777;
const SEQPKT_SRV_TYPE: Type = 16666;
const SRV_INST: Instance = 17;

fn main() -> io::Result<()> {
    println!("****** TIPC C API Demo Client Started ******\n");

    let srv = ServiceAddr::new(RDM_SRV_TYPE, SRV_INST);

    println!("Waiting for Service {}", srv);

    Server::wait(srv, Scope::Global, None)?;

    Ok(())
}
