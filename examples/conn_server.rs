use std::str;

use failure::Fallible;

use tipc::{Instance, SeqPacket, Type, Visibility::*};

const SERVER_TYPE: Type = 18888;
const SERVER_INST: Instance = 17;

const BUF_SZ: usize = 40;

fn main() -> Fallible<()> {
    println!("****** TIPC connection demo server started ******");

    let listener = tipc::bind::<SeqPacket, _>(((SERVER_TYPE, SERVER_INST), Zone))?.listen()?;

    let _ = crossbeam::scope(|s| {
        for (id, peer) in listener.incoming().enumerate() {
            let peer = peer.expect("Server: accept failed");

            println!("Server: accept() returned");

            s.spawn(move |_| -> Fallible<()> {
                let mut buf = [0; BUF_SZ];

                println!("Server process {} created", id);

                while let Ok(len) = peer.recv(&mut buf[..]) {
                    let msg = str::from_utf8(&buf[..len])?;

                    println!("Server {}: received msg {:?}", id, msg);

                    let res = format!("Response for test {}", id);

                    println!("Server {}: responded msg {:?}", id, res);

                    peer.send(res)?;
                }

                Ok(())
            });
        }
    });

    Ok(())
}
