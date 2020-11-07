use crate::{
    commit::Commits,
    config::Config,
    destination::Destination,
    error::{ErrorKind, GsyncError},
    Opt,
};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{
    io,
    io::{Read, Write},
};

pub struct Gsync {
    config: Config,
    source: PathBuf,
    destination: Destination,
    commits: Commits,
}

impl Gsync {
    pub fn from_options(opts: Opt) -> Result<Self, GsyncError> {
        let valid = validate_source(&opts.source)?;
        if !valid {
            return Err(GsyncError::Custom(ErrorKind::SourceNotExist));
        }
        let destination = Destination::parse_destination(&opts.destination)?;
        let config = Config::parse_config(&opts.config)?;
        let commits = Commits::new(opts.commits, &opts.source);
        Ok(Gsync {
            source: opts.source,
            destination,
            commits: commits,
            config,
        })
    }

    pub fn start(&self) {
        let file2sync = self.commits.changes();
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
                return;
            }
            _ => {
                let result: Result<Vec<usize>, _> =
                    decision.split(" ").map(|d| d.parse()).collect();
                if result.is_ok() {
                    choices = result.unwrap();
                } else {
                    println!("Invalid line numbers");
                    return;
                }
            }
        }

        let mut buffer = [0; 1024];
        let mut size: usize;
        let ssh: ssh2::Session;
        match self.destination.connect() {
            Err(_) => {
                eprintln!("Error while connecting remote machine");
                return;
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
    }
}

fn validate_source<P: AsRef<Path>>(source: P) -> io::Result<bool> {
    if !source.as_ref().exists() {
        eprintln!(
            "Source {} does not exists",
            source.as_ref().to_string_lossy()
        );
        return Ok(false);
    }

    match is_git_repo(&source) {
        Err(e) => {
            eprintln!("Error while validating source: {:?}", e);
            Err(e)
        }
        Ok(ret) => {
            if !ret {
                eprintln!(
                    "{} is not a git repository",
                    source.as_ref().to_string_lossy()
                );
            }
            Ok(ret)
        }
    }
}

fn is_git_repo<P: AsRef<Path>>(path: P) -> io::Result<bool> {
    Command::new("git")
        .arg("-C")
        .arg(path.as_ref())
        .arg("rev-parse")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
}

#[test]
fn test_is_git_repo() {
    assert_eq!(is_git_repo(".").unwrap(), true);
    assert_eq!(is_git_repo("/").unwrap(), false);
}
