//! A demo showing how unicast/anycast/multicast/broadcast can be sent loss free from a client to multiple servers,
//! demonstrating the group communication feature's flow control and sequence guarantee.

use std::io;
use std::mem;
use std::time::{Duration, Instant};

use failure::{Fallible, ResultExt};
use mio::{
    unix::{EventedFd, UnixReady},
    Events, Poll, PollOpt, Ready, Token,
};
use structopt::StructOpt;

use tipc::{Join, Recv, RecvMsg, ServiceAddr, SocketAddr, Type, Visibility::Cluster};

const SERVICE_TYPE: Type = 4711;
const BUF_LEN: usize = 66000;
const INTV_SZ: usize = 10000;

const RDM: Token = Token(0);

#[derive(Debug, StructOpt)]
#[structopt(name = "group_cast", about = "TIPC multicast demo.")]
struct Opt {
    /// send group broadcast
    #[structopt(short, long)]
    broadcast: bool,

    /// send group multicast
    #[structopt(short, long)]
    multicast: bool,

    /// send group anycast
    #[structopt(short, long)]
    anycast: bool,

    /// request response on sent Xcast
    #[structopt(short = "r", long = "reply")]
    ucast_reply: bool,

    /// destination member instance
    #[structopt(short, long)]
    destination: Option<u32>,

    /// member instance
    #[structopt(short, long)]
    instance: Option<u32>,

    /// message size
    #[structopt(short = "l", long = "msg-size", default_value = "66000")]
    msg_size: usize,

    /// loopback broadcast/multicast/anycast
    #[structopt(short = "L", long)]
    loopback: bool,
}

impl Opt {
    fn msg_type(&self) -> Option<MsgType> {
        if self.broadcast {
            Some(MsgType::Broadcast)
        } else if self.multicast {
            Some(MsgType::Multicast)
        } else if self.anycast {
            Some(MsgType::Anycast)
        } else {
            None
        }
    }

    fn dst_addr(&self) -> ServiceAddr {
        ServiceAddr::new(SERVICE_TYPE, self.instance.unwrap_or_default())
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum MsgType {
    Broadcast,
    Multicast,
    Unicast,
    Anycast,
}

#[repr(packed)]
#[derive(Debug)]
struct MsgHeader {
    ty: MsgType,
    reply: u8,
}

impl MsgHeader {
    fn reply(&self) -> bool {
        self.reply != 0
    }
}

#[derive(Debug)]
struct Client {
    opt: Opt,
    rdm: tipc::Group<tipc::Datagram>,
    member_cnt: usize,
    dst_member_cnt: usize,
    snt_bc: usize,
    snt_mc: usize,
    snt_ac: usize,
    snt_uc: usize,
    rcv_bc: usize,
    rcv_mc: usize,
    rcv_ac: usize,
    rcv_uc: usize,
}

impl Client {
    fn group_transceive(&mut self, events: &Events) -> Fallible<()> {
        let mut buf = [0; BUF_LEN];

        for event in events {
            println!("{:?}", event);

            let ready = UnixReady::from(event.readiness());

            if ready.is_error() {
                return Err(self.rdm.last_error().into());
            }

            if ready.is_readable() {
                self.read_msgs(&mut buf[..]).context("read msgs")?;
            }

            if let Some(msg_type) = self.opt.msg_type() {
                if self.dst_member_cnt > 0 && ready.is_writable() {
                    self.send_msgs(msg_type, &mut buf[..])
                        .context("send msgs")?;
                }
            }
        }

        Ok(())
    }

    fn read_msgs(&mut self, buf: &mut [u8]) -> Fallible<()> {
        while let Ok((msg, addr)) = self.rdm.as_ref().recv_msg(&mut buf[..], Recv::DONT_WAIT) {
            match msg {
                RecvMsg::MemberJoin(service) => {
                    println!("Member {} discovered at {}", service, addr);

                    if service.instance() == self.opt.instance.unwrap_or_default() {
                        self.dst_member_cnt += 1;
                    }
                }
                RecvMsg::MemberLeave(service) => {
                    println!("Member {} at {} lost", service, addr);

                    if service.instance() == self.opt.instance.unwrap_or_default() {
                        self.dst_member_cnt -= 1;
                    }
                }
                RecvMsg::Message { len, .. } if len == BUF_LEN => {
                    let hdr = unsafe { &*(buf.as_ptr() as *const MsgHeader) };

                    match hdr.ty {
                        MsgType::Broadcast => {
                            self.rcv_bc += 1;
                        }
                        MsgType::Multicast => {
                            self.rcv_mc += 1;
                        }
                        MsgType::Unicast => {
                            self.rcv_uc += 1;
                        }
                        MsgType::Anycast => {
                            self.rcv_ac += 1;
                        }
                    }

                    println!(
                        "Recv {} broadcasts, {} multicasts, {} anycasts, {} unicasts;",
                        self.rcv_bc, self.rcv_mc, self.rcv_ac, self.rcv_uc
                    );

                    if hdr.reply() {
                        println!("Sent {} unicasts", self.snt_uc);

                        self.send_reply(&mut buf[..len], addr)?;
                        self.snt_uc += 1;
                    }
                }
                _ => panic!("unexpected msg on member socket: {:?}", msg),
            }
        }

        Ok(())
    }

    fn send_reply(&self, buf: &mut [u8], addr: SocketAddr) -> Fallible<()> {
        buf[0] = MsgType::Unicast as u8;
        buf[1] = 0;

        loop {
            if self
                .rdm
                .send_to(&buf[..], addr)
                .or_else(should_try_again)
                .context("send reply")?
                == buf.len()
            {
                break;
            }
        }

        Ok(())
    }

    fn send_msgs(&mut self, msg_type: MsgType, msg: &mut [u8]) -> Fallible<()> {
        let len = msg.len();
        msg[0] = msg_type as u8;
        msg[1] = if self.opt.ucast_reply { 1 } else { 0 };

        loop {
            match msg_type {
                MsgType::Broadcast => {
                    if self
                        .rdm
                        .broadcast(&msg[..])
                        .or_else(should_try_again)
                        .context("send broadcast message")?
                        == len
                    {
                        self.snt_bc += 1;
                        break;
                    }
                }
                MsgType::Anycast => {
                    let dst = self.opt.dst_addr();
                    if self
                        .rdm
                        .anycast(&msg[..], dst)
                        .or_else(should_try_again)
                        .context("send anycast message")?
                        == len
                    {
                        self.snt_ac += 1;
                        break;
                    }
                }
                MsgType::Multicast => {
                    let dst = self.opt.dst_addr();
                    if self
                        .rdm
                        .multicast(&msg[..], dst)
                        .or_else(should_try_again)
                        .context("send multicast message")?
                        == len
                    {
                        self.snt_mc += 1;
                        break;
                    }
                }
                _ => panic!("unexpected msg type: {:?}", msg_type),
            }
        }

        Ok(())
    }
}

fn should_try_again(err: io::Error) -> io::Result<usize> {
    if err.kind() == io::ErrorKind::WouldBlock {
        Ok(0)
    } else {
        Err(err)
    }
}

fn main() -> Fallible<()> {
    let opt = Opt::from_args();

    // Prepared epolled socket
    let poll = Poll::new().context("create epoll object")?;

    let rdm = tipc::rdm().context("create member socket")?;
    rdm.set_nonblocking(true)?;
    poll.register(
        &EventedFd(&rdm),
        RDM,
        Ready::readable() | Ready::writable() | UnixReady::error(),
        PollOpt::edge(),
    )?;

    let group = ServiceAddr::new(SERVICE_TYPE, opt.instance.unwrap_or_default());
    let flags = Join::MEMBER_EVTS
        | if opt.loopback {
            Join::LOOPBACK
        } else {
            Join::empty()
        };

    let rdm = rdm.join(group, Cluster, flags).context("join group")?;

    println!("Joined as member {} at socket {}", group, rdm.local_addr()?);

    let mut client = Client {
        opt,
        rdm,
        ..unsafe { mem::zeroed() }
    };

    // Start receiving/transmitting
    let start_time = Instant::now();
    let mut start_intv = Instant::now();

    let mut events = Events::with_capacity(16);

    let mut prev = 0;

    while let Ok(_) = poll.poll_interruptible(&mut events, Some(Duration::from_secs(1))) {
        client.group_transceive(&events)?;

        if client.opt.msg_type().is_none() {
            continue;
        }

        let snt = client.snt_bc + client.snt_mc + client.snt_ac + client.snt_uc;
        if (snt - prev) < INTV_SZ {
            continue;
        }
        prev = snt;

        // Print out send statistics at regular intervals
        let msg_per_sec_tot = (snt * 1000) / start_time.elapsed().as_millis() as usize;
        let thruput_tot = (msg_per_sec_tot * BUF_LEN * 8) / 1000_000;
        let msg_per_sec_intv = (INTV_SZ * 1000) / start_intv.elapsed().as_millis() as usize;
        let thruput_intv = (msg_per_sec_intv * BUF_LEN * 8) / 1000_000;

        println!(
            "Sent {} broadcast, {} multicast, {} anycast, throughput {} MB/s, last intv {} MB/s",
            client.snt_bc, client.snt_mc, client.snt_ac, thruput_tot, thruput_intv
        );

        start_intv = Instant::now();
    }

    Ok(())
}
