use failure::Fallible;

use tipc::{topo, Scope, ServiceRange, Type};

const SERVER_TYPE: Type = 18888;

fn main() -> Fallible<()> {
    println!("TIPC Topology subscriber started");

    // Connect to topology server

    let top_srv = topo::connect(Scope::Global)?;

    println!("Client: connected to topology server");

    // Subscribe to subscription server name events

    let service_range: ServiceRange = (SERVER_TYPE, 0..100).into();

    top_srv.subscribe(service_range)?;

    println!("Client: issued subscription to {}", service_range);

    // Subscribe to network node name events

    let service_range: ServiceRange = 0.into();

    top_srv.subscribe(topo::Subscription::from(service_range).all())?;

    println!("Client: issued subscription to {}", service_range);

    println!("Client: subscriptions remain active until client is killed");

    loop {
        match top_srv.recv() {
            Ok(event) => {
                println!(
                    "Client: received {} event {} port id {}",
                    match event {
                        topo::Event::Published { .. } => "published",
                        topo::Event::Withdrawn { .. } => "withdrawn",
                    },
                    event.service(),
                    event.sock()
                );
            }
            Err(err) => {
                println!("Client: failed to receive event: {}", err);
                break;
            }
        }
    }

    Ok(())
}
