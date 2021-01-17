use log::{debug, LevelFilter};
use std::path::PathBuf;
use std::process::exit;
use structopt::StructOpt;

mod commit;
mod config;
mod destination;
mod error;
mod gsync;
mod logger;

#[derive(Debug, StructOpt)]
#[structopt(name = "gsync", about = "A tool to sync file from a git repository")]
pub struct Opt {
    #[structopt(short, long, required = true)]
    config: PathBuf,
    #[structopt(short, long, required = true)]
    source: PathBuf,
    #[structopt(short, long, required = true)]
    destination: String,
    #[structopt(short, parse(from_occurrences))]
    verbose: u8,
    #[structopt()]
    commits: Vec<String>,
}

fn main() {
    let opts = Opt::from_args();
    let log_level = match opts.verbose {
        0 => LevelFilter::Error, // the default
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        // 3 => LevelFilter::Trace,
        _ => LevelFilter::Off,
    };
    logger::init(log_level);
    debug!("Cmdline options: {:?}", opts);

    match gsync::Gsync::from_options(opts) {
        Err(_) => exit(1),
        Ok(sync) => {
            let success = sync.start();
            if success {
                println!("Done!");
            }
        }
    }
}
