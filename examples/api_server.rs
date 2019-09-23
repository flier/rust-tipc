use std::io::prelude::*;
use std::str;
use std::time::Duration;

use failure::{Fallible, ResultExt};
use mio::{
    unix::{EventedFd, UnixReady},
    Events, Poll, PollOpt, Ready, Token,
};
use tipc::{
    self, Bindable, Bound, Builder, Connected, Datagram, Listener, SeqPacket, ServiceRange, Stream,
    Type,
};

const RDM_SRV_TYPE: Type = 18888;
const STREAM_SRV_TYPE: Type = 17777;
const SEQPKT_SRV_TYPE: Type = 16666;

const BUF_SZ: usize = 40;

const STREAM: Token = Token(0);
const SEQ_PACKET: Token = Token(1);

fn bind_service<T>(builder: Builder<T>, ty: Type, name: &str) -> Fallible<Bound<T>>
where
    T: Bindable,
{
    let addr = ServiceRange::from(ty);
    let s = builder.bind(addr)?;

    println!(
        "Bound {} socket {} to {}",
        name,
        s.local_addr().unwrap(),
        addr
    );

    Ok(s)
}

fn recv_rdm_msg(rdm: &Datagram) -> Fallible<()> {
    let mut buf = [0; BUF_SZ];

    println!("\n-------------------------------------");

    let cli = match rdm.recv_from(&mut buf[..]) {
        Ok((len, cli)) => {
            println!(
                "Received msg: `{}` on SOCK_RDM",
                str::from_utf8(&buf[..len])?
            );
            println!("                        <-- {}", cli);
            cli
        }
        msg => panic!("unexpected message on RDM socket: {:?}", msg),
    };

    let msg = "Huh?";
    println!("Responding with: {}", msg);
    println!("                        --> {}", cli);

    rdm.send_to(msg, cli)?;

    println!("-------------------------------------");

    Ok(())
}

fn recv_stream_setup(listener: &Listener<Stream>) -> Fallible<Connected<Stream>> {
    let mut buf = [0; BUF_SZ];

    println!("\n-------------------------------------");

    let (mut stream, cli) = listener.accept().context("accept on SOCK_STREAM")?;

    println!("SOCK_STREAM connection established");
    println!("                        --> {}", cli);

    let len = stream.read(&mut buf)?;
    let msg = str::from_utf8(&buf[..len])?;

    println!("Received msg: {} on STREAM connection", msg);

    let msg = "Huh?";
    println!("Responding with: {}", msg);

    stream.write(msg.as_bytes())?;

    println!("-------------------------------------");

    Ok(stream)
}

fn recv_seqpacket_setup(listener: &Listener<SeqPacket>) -> Fallible<Connected<SeqPacket>> {
    let mut buf = [0; BUF_SZ];

    println!("\n-------------------------------------");

    let (seq_packet, cli) = listener.accept().context("accept on SOCK_SEQPACKET")?;

    println!("SOCK_SEQPACKET connection established");
    println!("                        --> {}", cli);

    let len = seq_packet.recv(&mut buf[..])?;
    let msg = str::from_utf8(&buf[..len])?;

    println!("Received msg: {} on SOCK_SEQPACKET connection", msg);

    let msg = "Huh?";
    println!("Responding with: {}", msg);

    seq_packet.send(msg.as_bytes())?;

    println!("-------------------------------------");

    Ok(seq_packet)
}

fn main() -> Fallible<()> {
    println!("****** TIPC C API Demo Server Started ******\n");

    let rdm: Datagram = bind_service(Builder::rdm()?, RDM_SRV_TYPE, "RDM")?.into();
    let stream_lisener: Listener<Stream> =
        bind_service(Builder::stream()?, STREAM_SRV_TYPE, "STREAM")?.listen()?;
    let seq_packet_lisener: Listener<SeqPacket> =
        bind_service(Builder::seq_packet()?, SEQPKT_SRV_TYPE, "SEQPACKET")?.listen()?;

    loop {
        recv_rdm_msg(&rdm)?;

        let poll = Poll::new()?;
        let mut events = Events::with_capacity(16);

        let stream = recv_stream_setup(&stream_lisener)?;
        poll.register(
            &EventedFd(&stream),
            STREAM,
            Ready::readable() | UnixReady::hup(),
            PollOpt::empty(),
        )?;

        let seq_packet = recv_seqpacket_setup(&seq_packet_lisener)?;
        poll.register(
            &EventedFd(&seq_packet),
            SEQ_PACKET,
            Ready::readable() | UnixReady::hup(),
            PollOpt::empty(),
        )?;

        while let Ok(_) = poll.poll_interruptible(&mut events, Some(Duration::from_secs(1))) {
            for event in &events {
                let ready = UnixReady::from(event.readiness());

                if !ready.is_hup() {
                    continue;
                }

                println!("\n-------------------------------------");
                println!(
                    "{} Connection hangup",
                    match event.token() {
                        STREAM => "SOCK_STREAM",
                        SEQ_PACKET => "SOCK_SEQPACKET",
                        token => panic!("unexpected {:?}", token),
                    }
                );
            }

            if !events.is_empty() {
                break;
            }
        }
    }

    println!("\n****** TIPC C API Demo Server Finished ******\n");

    Ok(())
}
