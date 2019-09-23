use std::io::{prelude::*, Cursor};
use std::iter;
use std::mem;
use std::net::Shutdown;

use failure::Fallible;

use tipc::{topo, Instance, Scope, ServiceAddr, Stream, Type};

const SERVER_TYPE: Type = 18888;
const SERVER_INST: Instance = 17;

const BUF_SIZE: usize = 2000;
const MSG_SIZE: usize = 80;

fn main() -> Fallible<()> {
    println!("****** TIPC stream demo client started ******");

    let addr = ServiceAddr::new(SERVER_TYPE, SERVER_INST);

    topo::wait(addr, Scope::Global, None)?;

    let mut peer = tipc::connect::<Stream, _>(addr)?;

    // Create buffer containing numerous (size,data) records

    let mut buf = [0u8; BUF_SIZE];
    let mut cur = Cursor::new(&mut buf[..]);
    let mut rec_size = 1;
    let mut rec_num: u32 = 0;

    while cur.position() as usize + mem::size_of::<u32>() + rec_size as usize <= BUF_SIZE {
        rec_num += 1;

        cur.write_all(&(rec_size as u32).to_ne_bytes()[..])?;
        cur.write_all(
            iter::repeat(rec_size as u8)
                .take(rec_size)
                .collect::<Vec<_>>()
                .as_slice(),
        )?;

        println!(
            "Client: creating record {} of size {} bytes",
            rec_num, rec_size
        );

        rec_size = ((rec_size + 147) % 0xFF).max(1); // record size must be 1-255 bytes
    }

    // Now send records using messages that break record boundaries

    println!(
        "Client: sending {} records with {} bytes using {} byte messages",
        rec_num,
        cur.position(),
        MSG_SIZE
    );

    for msg in cur.get_ref()[..cur.position() as usize].chunks(MSG_SIZE) {
        peer.write_all(msg)?;
    }

    // Now grab set of one-byte client acknowledgements all at once

    println!("Client: waiting for server acknowledgements");

    peer.read_exact(&mut buf[..rec_num as usize])?;

    println!("Client: received {} acknowledgements", rec_num);

    peer.shutdown(Shutdown::Both)?;

    println!("****** TIPC stream demo client finished ******");

    Ok(())
}
