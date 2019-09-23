use std::convert::TryInto;
use std::io::prelude::*;
use std::mem;

use failure::Fallible;

use tipc::{Instance, Stream, Type, Visibility::Zone};

const SERVER_TYPE: Type = 18888;
const SERVER_INST: Instance = 17;

const MAX_REC_SIZE: usize = 256;

fn main() -> Fallible<()> {
    println!("****** TIPC stream demo server started ******");

    let listener = tipc::bind::<Stream, _>((SERVER_TYPE, SERVER_INST, Zone))?.listen()?;
    let (mut peer, _addr) = listener.accept()?;

    let mut buf = [0; MAX_REC_SIZE];
    let mut rec_num = 0;

    while let Ok(()) = peer.read_exact(&mut buf[..mem::size_of::<u32>()]) {
        rec_num += 1;

        dbg!(&buf[..4]);

        let rec_size = u32::from_ne_bytes(buf[..mem::size_of::<u32>()].try_into()?) as usize;

        println!("Server: receiving record {} of {} bytes", rec_num, rec_size);

        assert!(rec_size < MAX_REC_SIZE);

        peer.read_exact(&mut buf[..rec_size])?;

        assert_eq!(&buf[..rec_size], vec![rec_size as u8; rec_size].as_slice());

        println!("Server: record {} received", rec_num);

        // Send 1 byte acknowledgement (value is irrelevant)
        peer.write_all(b"X")?;

        println!("Server: record {} acknowledged", rec_num);
    }

    println!("****** TIPC stream demo server finished ******");

    Ok(())
}
