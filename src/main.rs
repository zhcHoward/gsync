use serde_json;
use ssh2;
use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::{
    io,
    io::{Read, Write},
};
use structopt::StructOpt;

mod commit;
mod config;
mod destination;
mod error;
mod gsync;

#[derive(Debug, StructOpt)]
#[structopt(name = "gsync", about = "A tool to sync file from a git repository")]
struct Opt {
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

    // validate source
    if !utils::validate_source(&opts.source) {
        exit(1);
    }

    // validate remote and establish ssh connection
    let destination: utils::Destination;
    match utils::parse_destination(&opts.destination) {
        Some(dest) => destination = dest,
        None => exit(1),
    }
    let ssh: ssh2::Session;
    match destination.connect() {
        Err(_) => {
            eprintln!("Error while connecting remote machine");
            exit(1);
        }
        Ok(session) => ssh = session,
    }
    // println!("session authenticated: {:?}", ssh.authenticated());

    // read config file
    let config_file = opts.config.as_path();
    if !config_file.exists() {
        eprintln!(
            "Config file {} does not exist",
            config_file.to_string_lossy()
        )
    }
    let contents = fs::read_to_string(config_file).unwrap();
    let rules: config::Config = serde_json::from_str(&contents).unwrap();
    // println!("rules: {:?}", rules);

    // find changes
    let changes: HashSet<_> = opts
        .commits
        .iter()
        .map(|raw_commit| utils::find_changes(raw_commit, &opts.source))
        .fold(HashSet::new(), |acc, c| {
            acc.union(&c).into_iter().map(|f| f.clone()).collect()
        });
    // println!("{:?}", changes);

    // generate source and destination file path
    let mut matched = Vec::new();
    let mut not_matched = Vec::new();
    let mut is_matched: bool;
    for fpath in changes.iter() {
        is_matched = false;
        for (s, d) in rules.dir_map.iter() {
            if s.is_match(fpath) {
                let full_source_path = opts.source.join(fpath);
                let relative_dest_path = Path::new(fpath).strip_prefix(s.as_str()).unwrap();
                let full_dest_path = d.join(relative_dest_path);
                matched.push((full_source_path, full_dest_path));
                is_matched = true;
                break;
            }
        }
        if !is_matched {
            not_matched.push(fpath);
        }
    }

    println!("Following files will be updated:");
    let offset = matched.len().to_string().len();
    for (idx, (source, dest)) in matched.iter().enumerate() {
        println!("{0:>3$}. {1:?} --> {2:?}", idx + 1, source, dest, offset);
    }
    if !not_matched.is_empty() {
        println!("Following files has no configured remote dir:");
        for p in not_matched {
            println!("{:?}", p);
        }
    }

    println!("Update remote files? ('y' or 'n' or 'line number of files you want to update')");
    let choices: Vec<usize>;
    let mut decision = String::new();
    io::stdin().read_line(&mut decision).unwrap();
    match decision.as_str().trim() {
        "y" | "Y" => choices = matched.iter().enumerate().map(|m| m.0).collect(),
        "n" | "N" => {
            println!("Update cancelled");
            exit(0);
        }
        _ => {
            let result: Result<Vec<usize>, _> = decision.split(" ").map(|d| d.parse()).collect();
            if result.is_ok() {
                choices = result.unwrap();
            } else {
                println!("Invalid line numbers");
                exit(1);
            }
        }
    }

    let mut buffer = [0; 1024];
    let mut size: usize;
    for choice in choices {
        let (src, dst) = &matched[choice];
        let mut file = fs::File::open(src).unwrap();
        let metadata = file.metadata().unwrap();
        let mut remote = ssh
            .scp_send(
                dst,
                (metadata.permissions().mode() & 0o777) as i32,
                metadata.len(),
                None,
            )
            .unwrap();
        size = file.read(&mut buffer).unwrap();
        while size > 0 {
            remote.write(&buffer[..size]).unwrap();
            size = file.read(&mut buffer).unwrap();
        }
    }
    println!("Done!");
}
