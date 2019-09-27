use std::str;

use failure::Fallible;

use tipc::{seq_packet, Instance, SeqPacket, ServiceAddr, Type};

const SERVER_TYPE: Type = 18888;
const SERVER_INST: Instance = 17;

const BUF_SZ: usize = 40;

fn main() -> Fallible<()> {
    println!("****** TIPC connection demo client started ******\n");

    tipc::wait((SERVER_TYPE, SERVER_INST), None)?;

    let server_addr = ServiceAddr::new(SERVER_TYPE, SERVER_INST);

    println!("Client: connection setup 1 - standard (TCP style) connect");
    {
        let peer = tipc::connect::<SeqPacket, _>(server_addr)?;

        println!("Client: connection established");

        let msg = "Hello World";

        println!("Client: Sent msg: {:?}", msg);

        peer.send(msg)?;

        let mut buf = [0; BUF_SZ];
        let len = peer.recv(&mut buf[..])?;
        let msg = str::from_utf8(&buf[..len])?;

        println!("Client: received response {:?}", msg);

        peer.shutdown()?;

        println!("Client: shutting down connection");
    }

    println!("Client: connection setup 2 - optimized (TIPC style) connect");
    {
        let peer = seq_packet()?;
        let msg = "Hello Again";

        println!("Client: Sent msg: {:?}", msg);

        peer.send_to(msg, server_addr)?;

        let peer = peer.into_connected();

        let mut buf = [0; BUF_SZ];
        let len = peer.recv(&mut buf[..])?;
        let msg = str::from_utf8(&buf[..len])?;

        println!("Client: received response {:?}", msg);
        println!("Client: killing connection without shutdown");
    }

    println!("Client: connection setup 3 - optimized (TIPC style) connect");
    {
        let peer = seq_packet()?;
        let msg = "Hello Again";

        println!("Client: Sent msg: {:?}", msg);

        peer.send_to(msg, server_addr)?;

        println!("Client: will now exit without closing socket!!");
    }

    Ok(())
}
