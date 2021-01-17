use crate::{
    commit,
    config::Config,
    destination::Destination,
    error::{ErrorKind, GsyncError},
    Opt,
};
use log::error;
use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{
    io,
    io::{Read, Write},
    str,
};

pub struct Gsync {
    config: Config,
    source: PathBuf,
    destination: Destination,
    commits: Vec<String>,
}

impl Gsync {
    pub fn from_options(opts: Opt) -> Result<Self, GsyncError> {
        if !validate_source(&opts.source) {
            return Err(ErrorKind::SourceNotExist.error());
        }
        let destination = Destination::parse_destination(opts.destination)?;
        let config = Config::parse_config(opts.config)?;
        let commits = opts.commits;
        Ok(Gsync {
            source: opts.source,
            destination,
            commits,
            config,
        })
    }

    fn find_changes(&self) -> HashSet<String> {
        match self.commits.len() {
            0 => {
                let output = Command::new("git")
                    .arg("-C")
                    .arg(&self.source)
                    .arg("diff")
                    .arg("--name-status")
                    .output();
                let bytes = output.unwrap().stdout;
                let changes = str::from_utf8(&bytes).unwrap();
                commit::parse_changes(&changes)
            }
            _ => self
                .commits
                .iter()
                .map(|raw_commit| commit::find_changes(raw_commit, &self.source))
                .fold(HashSet::new(), |mut acc, changes| {
                    acc.extend(changes);
                    acc
                }),
        }
    }

    pub fn start(&self) -> bool {
        let file2sync = self.find_changes();
        let mut matched = Vec::new();
        let mut not_matched = Vec::new();
        let mut is_matched: bool;
        for fpath in file2sync.iter() {
            is_matched = false;
            for (s, d) in self.config.dir_map.iter() {
                if s.is_match(fpath) {
                    let full_source_path = self.source.join(fpath);
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
            println!(
                "{0:>3$}. {1} --> {2}",
                idx + 1,
                source.to_string_lossy(),
                dest.to_string_lossy(),
                offset
            );
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
                println!("Update cancelled.");
                return false;
            }
            decision => {
                let result: Result<Vec<usize>, _> =
                    decision.split(" ").map(|d| d.parse()).collect();
                if result.is_ok() {
                    choices = result.unwrap().iter().map(|c| c - 1).collect();
                } else {
                    println!("Invalid line numbers!");
                    return false;
                }
            }
        }

        let mut buffer = [0; 1024];
        let mut size: usize;
        let ssh: ssh2::Session;
        match self.destination.connect() {
            Err(_) => {
                eprintln!("Failed to connect remote machine!");
                return false;
            }
            Ok(session) => ssh = session,
        };

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
        true
    }
}

fn validate_source<P: AsRef<Path>>(source: P) -> bool {
    if !source.as_ref().exists() {
        error!(
            "Source {} does not exists",
            source.as_ref().to_string_lossy()
        );
        return false;
    }

    is_git_repo(source.as_ref())
}

fn is_git_repo<P: AsRef<Path>>(path: P) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(path.as_ref())
        .arg("rev-parse")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success()
}

#[test]
fn test_is_git_repo() {
    assert_eq!(is_git_repo("."), true);
    assert_eq!(is_git_repo("/"), false);
}
