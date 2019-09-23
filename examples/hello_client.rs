use std::str;

use failure::Fallible;

use tipc::{topo, Instance, Scope, ServiceAddr, Type};

const SERVER_TYPE: Type = 18888;
const SERVER_INST: Instance = 17;

const BUF_SIZE: usize = 40;

fn main() -> Fallible<()> {
    println!("****** TIPC hello world client started ******");

    topo::wait((SERVER_TYPE, SERVER_INST), Scope::Global, None)?;

    let srv = ServiceAddr::new(SERVER_TYPE, SERVER_INST);
    let rdm = tipc::rdm()?;

    let msg = "Hello World!!!";
    rdm.send_to(msg, srv)?;
    println!("Client: sent message: {}", msg);

    let rdm = rdm.into_connected();
    let mut buf = [0; BUF_SIZE];
    let len = rdm.recv(&mut buf[..])?;
    let msg = str::from_utf8(&buf[..len])?;
    println!("Client: received response: {}", msg);

    println!("****** TIPC hello client finished ******");

    Ok(())
}
