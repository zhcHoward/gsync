use std::path::PathBuf;
use std::process::exit;
use structopt::StructOpt;

mod commit;
mod config;
mod destination;
mod error;
mod gsync;

#[derive(Debug, StructOpt)]
#[structopt(name = "gsync", about = "A tool to sync file from a git repository")]
pub struct Opt {
    #[structopt(short, long, required = true)]
    config: PathBuf,
    #[structopt(short, long, required = true)]
    source: PathBuf,
    #[structopt(short, long, required = true)]
    destination: String,
    #[structopt()]
    commits: Vec<String>,
}

fn main() {
    let opts = Opt::from_args();
    // println!("{:?}", opts);

    match gsync::Gsync::from_options(opts) {
        Err(_) => exit(1),
        Ok(sync) => {
            sync.start();
            println!("Done!");
        }
    }
}
