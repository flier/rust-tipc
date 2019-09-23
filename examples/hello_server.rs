use std::str;

use failure::Fallible;

use tipc::{Instance, ServiceAddr, Type};

const SERVER_TYPE: Type = 18888;
const SERVER_INST: Instance = 17;

const BUF_SIZE: usize = 40;

fn main() -> Fallible<()> {
    println!("****** TIPC hello world server started ******");

    let srv = ServiceAddr::new(SERVER_TYPE, SERVER_INST);
    let rdm = tipc::rdm()?.bind(srv)?;

    let mut buf = [0; BUF_SIZE];
    let (len, addr) = rdm.recv_from(&mut buf[..])?;
    let msg = str::from_utf8(&buf[..len])?;
    println!("Server: Message received: {}", msg);

    let msg = "Uh ?";
    rdm.send_to(msg, addr)?;
    println!("Server: Sent response: {}", msg);

    println!("****** TIPC hello world server finished ******");

    Ok(())
}
