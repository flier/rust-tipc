use failure::{Fallible, ResultExt};

use tipc::{topo, Instance, NetworkAddr, Scope};

/// Connect to the TIPC topology server
fn topology_connect() -> Fallible<topo::Server> {
    Ok(topo::connect(Scope::Global).context("connect to the TIPC topology server")?)
}

/// subscribe for node and link state events
fn subscribe_evt(topsrv: &topo::Server) -> Fallible<()> {
    topsrv
        .subscribe(topo::NEIGHBOR_NODES)
        .context("subscribe to TIPC node events")?;
    topsrv
        .subscribe(topo::NEIGHBOR_LINKS)
        .context("subscribe to TIPC link state events")?;

    Ok(())
}

fn log_event(event: topo::Event, own_node: Instance) -> Fallible<()> {
    match *event.subscription() {
        topo::NEIGHBOR_NODES => {
            log_node(event.into(), own_node)?;
        }
        topo::NEIGHBOR_LINKS => {
            log_link(event.into())?;
        }
        _ => panic!("Unknown event received: {:?}", event),
    }

    Ok(())
}

fn log_node(node: topo::Node, own_node: Instance) -> Fallible<()> {
    use topo::Node::*;

    if node.instance() != own_node {
        match node {
            Up(instance) => {
                println!(
                    "Established contact with node {}",
                    NetworkAddr::from(instance)
                );
            }
            Down(instance) => {
                println!("Lost contact with node {}", NetworkAddr::from(instance));
            }
        }
    }

    Ok(())
}

fn log_link(link: topo::Link) -> Fallible<()> {
    println!(
        "{} link <{}> on network plane {}",
        if link.available() {
            "Established"
        } else {
            "Lost"
        },
        link.local_link_name()?,
        link.local_bearer_id()
    );

    Ok(())
}

fn main() -> Fallible<()> {
    let topsrv = topology_connect()?;
    let own_node = topsrv
        .local_addr()
        .context("fetch the local TIPC address")?
        .node();

    subscribe_evt(&topsrv)?;

    println!("TIPC network event logger started");

    for event in &topsrv {
        log_event(event, own_node)?;
    }

    Ok(())
}
