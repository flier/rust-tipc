//! TIPC multicast demo (server side)

use std::str;

use failure::Fallible;
use structopt::StructOpt;

use tipc::{ServiceRange, Type};

const SERVICE_TYPE: Type = 4711;

const BUF_SIZE: usize = 100;

#[derive(Debug, StructOpt)]
#[structopt(name = "mcast_server", about = "TIPC multicast demo (server side)")]
struct Opt {
    /// lower bound
    #[structopt()]
    lower: Option<u32>,

    /// upper bound
    #[structopt()]
    upper: Option<u32>,
}

fn server_mcast(service_range: ServiceRange) -> Fallible<()> {
    let rdm = tipc::rdm()?.bind(service_range)?;

    println!(
        "Server: port {} created at {}",
        service_range,
        rdm.local_addr()?
    );

    let mut buf = [0; BUF_SIZE];

    while let Ok((len, addr)) = rdm.recv_from(&mut buf[..]) {
        if len == 0 || buf[0] == 0 {
            println!("Server: port {} terminated", service_range);
            break;
        }

        let msg = str::from_utf8(&buf[..len])?;

        println!(
            "Server: port {} received from {}: {}",
            service_range, addr, msg
        );
    }

    Ok(())
}

// Mainline for server side of multicast demo.
//
// Usage: mcast_server [lower [upper]]
//
// If no arguments supplied, creates a predetermined set of multicast servers.
// If optional "lower" and "upper" arguments are specified, creates a single
// server for the specified instance range; if "upper" is omitted, it defaults
// to the same value as "lower".
fn main() -> Fallible<()> {
    let opt = Opt::from_args();

    let _ = crossbeam::scope(|s| {
        for range in match (opt.lower, opt.upper) {
            (Some(lower), None) => vec![lower..lower],
            (Some(lower), Some(upper)) => vec![lower..upper],
            _ => vec![0..99, 100..199, 200..299, 300..399],
        } {
            s.spawn(move |_| server_mcast((SERVICE_TYPE, range).into()).expect("server mcast"));
        }
    });

    Ok(())
}
