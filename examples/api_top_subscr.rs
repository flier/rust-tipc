use std::time::Duration;

use failure::{Fallible, ResultExt};

use mio::{
    unix::{EventedFd, UnixReady},
    Events, Poll, PollOpt, Ready, Token,
};
use tipc::{topo, Scope, Type};

const RDM_SRV_TYPE: Type = 18888;
const STREAM_SRV_TYPE: Type = 17777;
const SEQPKT_SRV_TYPE: Type = 16666;

const TOP_SERVER: Token = Token(0);
const NEIGHBOR_NODES: Token = Token(1);
const NEIGHBOR_LINKS: Token = Token(2);
const NEIGHBOR_NODE: Token = Token(4);

fn main() -> Fallible<()> {
    println!("****** TIPC C API Topology Service Client Started ******\n");

    let poll = Poll::new()?;

    // Subscribe for service events
    let top_srv = topo::connect(Scope::Global)?;
    poll.register(
        &EventedFd(&top_srv),
        TOP_SERVER,
        Ready::readable() | UnixReady::hup(),
        PollOpt::empty(),
    )?;
    top_srv
        .subscribe(RDM_SRV_TYPE)
        .context("subscribe for RDM server")?;
    top_srv
        .subscribe(STREAM_SRV_TYPE)
        .context("subscribe for STREAM server")?;
    top_srv
        .subscribe(SEQPKT_SRV_TYPE)
        .context("subscribe for SEQPACKET server")?;

    // Subscribe for neighbor nodes
    let nodes = topo::neighbor_nodes(Scope::Global).context("subscribe for neighbor nodes")?;
    poll.register(
        &EventedFd(&nodes),
        NEIGHBOR_NODES,
        Ready::readable() | UnixReady::hup(),
        PollOpt::empty(),
    )?;

    // Subscribe for neighbor links
    let links = topo::neighbor_links(Scope::Global).context("subscribe for neigbor links")?;
    poll.register(
        &EventedFd(&links),
        NEIGHBOR_LINKS,
        Ready::readable() | UnixReady::hup(),
        PollOpt::empty(),
    )?;

    let mut events = Events::with_capacity(16);
    let mut neigh_topsrvnode = None;

    while let Ok(_) = poll.poll_interruptible(&mut events, Some(Duration::from_secs(1))) {
        for event in &events {
            let ready = UnixReady::from(event.readiness());

            match event.token() {
                TOP_SERVER if ready.is_readable() => {
                    let evt = top_srv.recv().context("reception of service event")?;

                    match evt.service().ty() {
                        RDM_SRV_TYPE => {
                            println!(
                                "Service {} on SOCK_RDM is {}",
                                evt.service(),
                                if evt.available() { "UP" } else { "DOWN" }
                            );
                        }
                        STREAM_SRV_TYPE => {
                            println!(
                                "Service {} on SOCK_STREAM is {}",
                                evt.service(),
                                if evt.available() { "UP" } else { "DOWN" }
                            );
                        }
                        SEQPKT_SRV_TYPE => {
                            println!(
                                "Service {} on SOCK_SEQPACKET is {}",
                                evt.service(),
                                if evt.available() { "UP" } else { "DOWN" }
                            );
                        }
                        _ => panic!("unexpected {:?}", evt),
                    }
                }
                NEIGHBOR_NODES if ready.is_readable() => {
                    let node = nodes.recv().context("reception of service event")?;

                    match node {
                        topo::Node::Up(instance) => {
                            println!("Found neighbor node {}", instance);

                            // Allow only one "neighbor's neighbor's" subsription
                            if neigh_topsrvnode.is_none() && !Scope::new(instance).is_own_node() {
                                let neigh_node = topo::neighbor_nodes(Scope::new(instance))?;

                                poll.register(
                                    &EventedFd(&neigh_node),
                                    NEIGHBOR_NODE,
                                    Ready::readable() | UnixReady::hup(),
                                    PollOpt::empty(),
                                )?;

                                neigh_topsrvnode = Some((neigh_node, instance));
                            }
                        }
                        topo::Node::Down(instance) => {
                            println!("Lost contact with neighbor node {}", instance);

                            match neigh_topsrvnode {
                                Some((ref neigh_node, id)) if id == instance => {
                                    poll.deregister(&EventedFd(&neigh_node))?;
                                    neigh_topsrvnode.take();
                                }
                                _ => {}
                            }
                        }
                    }
                }
                NEIGHBOR_LINKS if ready.is_readable() => {
                    let link = links.recv().context("reception of service event")?;

                    println!(
                        "{} link {}",
                        if link.available() { "Found" } else { "Lost" },
                        link.local_link_name()?
                    );
                }
                NEIGHBOR_NODE if ready.is_hup() => {
                    if let Some((neigh_node, _)) = neigh_topsrvnode.take() {
                        poll.deregister(&EventedFd(&neigh_node))?;
                    }
                }
                NEIGHBOR_NODE if ready.is_readable() => {
                    if let Some((ref neigh_node, id)) = neigh_topsrvnode.take() {
                        let node = neigh_node.recv().context("reception of service event")?;

                        println!(
                            "Neighbor node {} {} {}",
                            id,
                            if node.available() {
                                "found node"
                            } else {
                                "lost contact with node"
                            },
                            node.instance()
                        )
                    }
                }
                _ => panic!("unexpected {:?}", event),
            }
        }
    }

    Ok(())
}
