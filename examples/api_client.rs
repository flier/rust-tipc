use std::io::{self, prelude::*};
use std::str;
use std::time::Duration;

use failure::{Fallible, ResultExt};
use mio::{
    unix::{EventedFd, UnixReady},
    Events, Poll, PollOpt, Ready, Token,
};
use tipc::{topo, Builder, Datagram, Instance, Scope, SeqPacket, ServiceAddr, Stream, Type};

const RDM_SRV_TYPE: Type = 18888;
const STREAM_SRV_TYPE: Type = 17777;
const SEQPKT_SRV_TYPE: Type = 16666;

const SRV_INST: Instance = 17;

const RDM: Token = Token(0);
const STREAM: Token = Token(1);
const SEQ_PACKET: Token = Token(2);
const TOP_SERVER: Token = Token(3);

const BUF_SZ: usize = 40;

fn rdm_service_demo(datagram: &Datagram) -> Fallible<Scope> {
    println!("\n\n-------------------------------------");
    println!("Service on SOCK_RDM came up");

    let msg = "Hello World";
    let srv = ServiceAddr::new(RDM_SRV_TYPE, SRV_INST);

    println!("Sending msg: `{}` on SOCK_RDM", msg);
    println!("               -->{}", srv);

    datagram.set_rejectable(false)?;
    datagram.send_to(msg, srv)?;

    let mut buf = [0; BUF_SZ];

    match datagram.recv_from(&mut buf[..]) {
        Ok((len, srv)) => {
            let msg = str::from_utf8(&buf[..len])?;

            println!("Received response: {}", msg);
            println!("               <-- {:?}", srv);

            Ok(Scope::new(srv.node()))
        }
        res => panic!("unexpected response {:?}", res),
    }
}

fn rdm_reject_demo(datagram: &Datagram, scope: Scope) -> Fallible<()> {
    let msg = "Hello World";
    let invalid = ServiceAddr::new(42, 1);

    println!("\nSending msg: `{}` on SOCK_RDM ", msg);
    println!("               --> {} (non-existing)", invalid);

    match datagram.send_to(msg, (invalid, scope)) {
        Ok(len) => {
            if len != msg.len() {
                println!("Client sendto() failed: incomplete send, {}", len);
                return Ok(());
            }
        }
        Err(err) => {
            println!("Client sendto() failed: {}", err);
            return Ok(());
        }
    }

    let mut buf = [0; BUF_SZ];

    match datagram.recv_from(&mut buf[..]) {
        Err(ref err) if err.kind() == io::ErrorKind::Other => {
            println!("Received rejected msg: {}", err)
        }
        res => println!("unexpected response {:?}", res),
    }

    Ok(())
}

fn stream_service_demo(builder: &Builder<Stream>) -> Fallible<()> {
    println!("\n\n-------------------------------------");
    println!("Service on SOCK_STREAM came up");

    let srv = ServiceAddr::new(STREAM_SRV_TYPE, SRV_INST);

    println!("Connecting to:              -->{}", srv);

    let mut stream = builder.try_clone()?.connect(srv)?;

    let msg = "Hello World";

    println!("Sending msg: `{}` on connection", msg);

    stream.write_all(msg.as_bytes())?;

    let mut buf = [0; BUF_SZ];
    let len = stream.read(&mut buf)?;
    let msg = str::from_utf8(&buf[..len])?;

    println!("Received response: `{}` on SOCK_STREAM connection", msg);
    println!("-------------------------------------");

    Ok(())
}

fn seqpacket_service_demo(builder: &Builder<SeqPacket>) -> Fallible<()> {
    println!("\n\n-------------------------------------");
    println!("Service on SOCK_SEQPACKET came up");

    let srv = ServiceAddr::new(SEQPKT_SRV_TYPE, SRV_INST);

    println!("Connecting to:              -->{}", srv);

    let seq_packet = builder.try_clone()?.connect(srv)?;

    let msg = "Hello World";

    println!("Sending msg: `{}` on connection", msg);

    seq_packet.send(msg)?;

    let mut buf = [0; BUF_SZ];
    let len = seq_packet.recv(&mut buf[..])?;
    let msg = str::from_utf8(&buf[..len])?;

    println!("Received response: `{}` on SOCK_SEQPACKET connection", msg);
    println!("-------------------------------------");

    Ok(())
}

fn main() -> Fallible<()> {
    println!("****** TIPC C API Demo Client Started ******\n");

    let srv = ServiceAddr::new(RDM_SRV_TYPE, SRV_INST);

    println!("Waiting for Service {}", srv);

    topo::wait(srv, Scope::Global, None)?;

    let poll = Poll::new()?;

    // Create traffic sockets
    let rdm = tipc::rdm()?;
    poll.register(&EventedFd(&rdm), RDM, Ready::readable(), PollOpt::empty())?;

    let mut stream = Builder::stream()?;
    poll.register(
        &EventedFd(&stream),
        STREAM,
        Ready::readable() | UnixReady::hup(),
        PollOpt::empty(),
    )?;

    let mut seq_packet = Builder::seq_packet()?;
    poll.register(
        &EventedFd(&seq_packet),
        SEQ_PACKET,
        Ready::readable() | UnixReady::hup(),
        PollOpt::empty(),
    )?;

    // Subscribe for service events
    let top_srv = topo::connect(Scope::Global)?;
    poll.register(
        &EventedFd(&top_srv),
        TOP_SERVER,
        Ready::readable() | UnixReady::hup(),
        PollOpt::empty(),
    )?;
    top_srv
        .subscribe(RDM_SRV_TYPE, false, None, 0)
        .context("subscribe for RDM server")?;
    top_srv
        .subscribe(STREAM_SRV_TYPE, false, None, 0)
        .context("subscribe for STREAM server")?;
    top_srv
        .subscribe(SEQPKT_SRV_TYPE, false, None, 0)
        .context("subscribe for SEQPACKET server")?;

    let mut events = Events::with_capacity(16);

    while let Ok(_) = poll.poll_interruptible(&mut events, Some(Duration::from_secs(1))) {
        for event in &events {
            let ready = UnixReady::from(event.readiness());

            match event.token() {
                STREAM if ready.is_hup() => {
                    println!("SOCK_STREAM connection hangup");

                    poll.deregister(&EventedFd(&stream))?;

                    stream = Builder::stream()?;

                    poll.register(
                        &EventedFd(&stream),
                        STREAM,
                        Ready::readable() | UnixReady::hup(),
                        PollOpt::empty(),
                    )?;
                }
                SEQ_PACKET if ready.is_hup() => {
                    println!("SOCK_SEQPACKET connection hangup");

                    poll.deregister(&EventedFd(&seq_packet))?;

                    seq_packet = Builder::seq_packet()?;

                    poll.register(
                        &EventedFd(&seq_packet),
                        SEQ_PACKET,
                        Ready::readable() | UnixReady::hup(),
                        PollOpt::empty(),
                    )?;
                }
                TOP_SERVER if ready.is_readable() => {
                    let evt = top_srv.recv().context("reception of service event")?;

                    match evt.service().ty() {
                        RDM_SRV_TYPE => {
                            if !evt.available() {
                                println!("Service on SOCK_RDM went down");
                            } else {
                                let scope = rdm_service_demo(&rdm)?;

                                rdm_reject_demo(&rdm, scope)?;
                            }
                        }
                        STREAM_SRV_TYPE => {
                            if !evt.available() {
                                println!("Service on SOCK_STREAM went down");
                            } else {
                                stream_service_demo(&stream)?;
                            }
                        }
                        SEQPKT_SRV_TYPE => {
                            if !evt.available() {
                                println!("Service on SOCK_SEQPACKET went down");
                            } else {
                                seqpacket_service_demo(&seq_packet)?;
                            }
                        }
                        _ => panic!("unexpected {:?}", evt),
                    }
                }
                _ => panic!("unexpected {:?}", event),
            }
        }
    }

    println!("\n****** TIPC C API Demo Finished ******\n");

    Ok(())
}
