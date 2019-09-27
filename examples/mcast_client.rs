//! TIPC multicast demo (client side)

use failure::Fallible;
use structopt::StructOpt;

use tipc::{ServiceRange, Type};

const SERVICE_TYPE: Type = 4711;

#[derive(Debug, StructOpt)]
#[structopt(name = "mcast_client", about = "TIPC multicast demo (client side)")]
struct Opt {
    /// lower bound
    #[structopt()]
    lower: Option<u32>,

    /// upper bound
    #[structopt()]
    upper: Option<u32>,

    /// sends the server termination message
    #[structopt()]
    kill: Option<bool>,
}

fn client_mcast(service_range: ServiceRange, kill: bool) -> Fallible<()> {
    let rdm = tipc::rdm()?;

    let msg = if kill {
        println!("Client: sending termination message to {}", service_range);

        "\0".to_string()
    } else {
        let msg = format!("message to {}", service_range);

        println!("Client: sending {}", msg);

        msg
    };

    rdm.send_to(&msg, service_range)?;

    Ok(())
}

// Mainline for client side of multicast demo.
//
// Usage: client_tipc [lower [upper [kill]]]
//
// If no arguments supplied, sends a predetermined set of multicast messages.
// If optional "lower" and "upper" arguments are specified, client sends a
// single message to the specified instance range; if "upper" is omitted,
// it defaults to the same value as "lower".
// If optional "kill" argument is specified (it can have any value), the client
// sends the server termination message rather than the normal server data
// message.
fn main() -> Fallible<()> {
    tipc::wait((SERVICE_TYPE, 399), None)?;

    let opt = Opt::from_args();

    if let Some(service_range) = match (opt.lower, opt.upper) {
        (Some(lower), None) => Some(lower..lower),
        (Some(lower), Some(upper)) => Some(lower..upper),
        _ => None,
    } {
        // multicast to instance range specified by user

        client_mcast(
            (SERVICE_TYPE, service_range).into(),
            opt.kill.unwrap_or_default(),
        )?;
    } else {
        // run standard demo if no arguments supplied by user

        println!("****** TIPC client multicast demo started ******\n");

        client_mcast((SERVICE_TYPE, 99..100).into(), false)?;
        client_mcast((SERVICE_TYPE, 150..250).into(), false)?;
        client_mcast((SERVICE_TYPE, 200..399).into(), false)?;
        client_mcast((SERVICE_TYPE, 0..399).into(), false)?;
        client_mcast((SERVICE_TYPE, 0..399).into(), true)?;

        println!("\n****** TIPC client multicast demo finished ******");
    }

    Ok(())
}
