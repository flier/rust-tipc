//! A minimal program just showing how a server receives a message by using `recvmsg()`
//! while reading the source socket address and the used destination service address.

use std::io;
use std::str;
use std::thread;
use std::time::Duration;

use failure::Fallible;

use tipc::{ServiceAddr, Type, Visibility::Zone};

const MY_TYPE: Type = 666;

fn main() -> Fallible<()> {
    let _ = crossbeam::scope(|s| {
        s.spawn(move |_| -> Fallible<()> {
            let rdm = tipc::rdm()?.bind((MY_TYPE, Zone))?;

            let mut header = [0u8; 1];
            let mut data = [0u8; 7];
            let mut iov = [
                io::IoSliceMut::new(&mut header[..]),
                io::IoSliceMut::new(&mut data[..]),
            ];

            let (len, peer, service) = rdm.recv_vectored(&mut iov[..])?;

            println!("Server received {} bytes", len);
            if let Some(service) = service {
                println!("Message was sent from {} to {}", peer, service);
            }
            println!(
                "Header: {}, Data: {}",
                str::from_utf8(&header[..])?,
                str::from_utf8(&data[..])?
            );

            Ok(())
        });

        s.spawn(move |_| -> Fallible<()> {
            // Address a random instance in MY_TYPE
            let addr = ServiceAddr::new(MY_TYPE, rand::random());

            let rdm = tipc::rdm()?;

            // Give the server a chance to start
            thread::sleep(Duration::from_secs(1));

            println!("Client: Send packet to {}", addr);

            let header = b"H";
            let data = b"DDDDDDD";
            let iov = [io::IoSlice::new(header), io::IoSlice::new(data)];

            rdm.send_vectored(&iov[..], addr)?;

            Ok(())
        });
    });

    Ok(())
}
