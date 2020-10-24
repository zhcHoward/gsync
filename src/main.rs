use structopt::StructOpt;
use std::path::PathBuf;
use std::process::exit;
use ssh2;

mod utils;

#[derive(Debug, StructOpt)]
#[structopt(name = "gsync", about = "A tool to sync file from a repository")]
struct Opt {
    #[structopt(short, long, required=true)]
    config: PathBuf,
    #[structopt()]
    source: PathBuf,
    #[structopt()]
    destination: String,
}

fn main() {
    let opts = Opt::from_args();
    println!("{:?}", opts);

    if !utils::validate_source(&opts.source) {
        exit(1);
    }

    let destination: utils::Destination;
    match utils::parse_destination(&opts.destination) {
        Some(dest) => destination = dest,
        None => exit(1),
    }
    let ssh: ssh2::Session;
    match destination.connect() {
        Err(e) => {
            eprintln!("Error while connecting remote machine: {:?}", e);
            exit(1);
        },
        Ok(session) => ssh = session,
    }
    println!("session authenticated: {:?}", ssh.authenticated());
}
