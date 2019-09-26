use std::thread::sleep;
use std::time::Duration;

use failure::Fallible;

use tipc::{Instance, ServiceRange, Type, Visibility::Zone};

const SERVER_TYPE: Type = 18888;
const SERVER_INST_LOWER: Instance = 6;
const SERVER_INST_UPPER: Instance = 53;

fn main() -> Fallible<()> {
    let server_addr = ServiceRange::with_range(SERVER_TYPE, SERVER_INST_LOWER..SERVER_INST_UPPER);
    let visibility = Zone;

    // Make server available
    let rdm = tipc::rdm()?.bind((server_addr, visibility))?;

    println!(
        "Server: bound port A to {} scope {:?}",
        server_addr, visibility
    );

    // Bind name a second time, to get a higher share of the calls
    rdm.bind(server_addr)?;

    println!(
        "Server: bound port A to name sequence {} scope {:?}",
        server_addr, visibility
    );

    // Bind a third time, with a different name sequence
    let server_addr =
        ServiceRange::with_range(SERVER_TYPE, SERVER_INST_UPPER + 1..SERVER_INST_UPPER + 2);

    rdm.bind(server_addr)?;

    println!(
        "Server: bound port A to name sequence {} scope {:?}",
        server_addr, visibility
    );

    // Bind a second port to the same sequence
    let _rdm2 = tipc::rdm()?.bind((server_addr, visibility))?;

    println!(
        "Server: bound port B to name sequence {} scope {:?}",
        server_addr, visibility
    );
    println!("Server: port names remain published until server is killed");

    loop {
        sleep(Duration::from_secs(1));
    }
}
