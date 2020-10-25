use serde_json;
use ssh2;
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use structopt::StructOpt;

mod utils;

#[derive(Debug, StructOpt)]
#[structopt(name = "gsync", about = "A tool to sync file from a git repository")]
struct Opt {
    #[structopt(short, long, required = true)]
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
        }
        Ok(session) => ssh = session,
    }
    println!("session authenticated: {:?}", ssh.authenticated());

    let config_file = opts.config.as_path();
    if !config_file.exists() {
        eprintln!(
            "Config file {} does not exist",
            config_file.to_string_lossy()
        )
    }
    let contents = fs::read_to_string(config_file).unwrap();
    let rules: serde_json::Value = serde_json::from_str(&contents).unwrap();
    println!("rules: {:?}", rules);
}
