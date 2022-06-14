use crate::{
    commit,
    config::Config,
    destination::Destination,
    error::{ErrorKind, GsyncError},
    Opt,
};
use indicatif::ProgressBar;
use log::error;
use std::{
    collections::HashSet,
    env,
    ffi::OsStr,
    fs, io,
    io::{Read, Write},
    os::unix::{ffi::OsStrExt, fs::PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
    str,
};

pub struct Gsync {
    config: Config,
    destination: Destination,
    commits: Vec<String>,
}

impl Gsync {
    pub fn from_options(opts: Opt) -> Result<Self, GsyncError> {
        validate_source(&opts.source)?;
        let repo_root = get_repo_root(opts.source)?;
        let config = Config::parse_config(opts.config)?;
        let destination = Destination::parse_destination(opts.destination)?;
        let commits = opts.commits;

        // change current working directory for following steps so that
        // 1. `-C` parameter can be omitted for the following git command
        // 2. handling files' relative paths will be easier and clearer
        env::set_current_dir(&repo_root)?;
        Ok(Gsync {
            destination,
            commits,
            config,
        })
    }

    fn find_changes(&self) -> HashSet<String> {
        match self.commits.len() {
            0 => {
                let output = Command::new("git").arg("status").arg("--short").output();
                let bytes = output.unwrap().stdout;
                let changes = str::from_utf8(&bytes).unwrap();
                commit::parse_changes(&changes)
            }
            _ => self
                .commits
                .iter()
                .map(|raw_commit| commit::find_changes(raw_commit))
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
        let mut ignored = Vec::new();
        'outer: for fpath in file2sync.iter() {
            for ignore in self.config.ignored.iter() {
                if ignore.is_match(fpath) {
                    ignored.push(fpath);
                    continue 'outer;
                }
            }

            for (s, d) in self.config.dir_map.iter() {
                if s.is_match(fpath) {
                    let source_path = Path::new(fpath);
                    let relative_dest_path = source_path.strip_prefix(s.as_str()).unwrap();
                    let full_dest_path = d.join(relative_dest_path);
                    matched.push((source_path, full_dest_path));
                    continue 'outer;
                }
            }

            not_matched.push(fpath);
        }

        if !ignored.is_empty() {
            println!("Following files are ignored:");
            for p in ignored {
                println!("{:?}", p);
            }
        }

        if matched.is_empty() {
            if !not_matched.is_empty() {
                println!("Following files has no configured remote dir:");
                for p in not_matched {
                    println!("{}", p);
                }
            }
            println!("No file will be updated, exit.");
            return true;
        }

        println!("Following files will be updated:");
        let offset = matched.len().to_string().len();
        matched.sort_unstable_by(|a, b| a.cmp(b));
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
                println!("{}", p);
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

        let bar = ProgressBar::new(choices.len() as u64);
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
            bar.inc(1);
        }
        bar.finish();
        true
    }
}

fn validate_source<P: AsRef<Path>>(source: P) -> Result<(), GsyncError> {
    match source.as_ref().exists() {
        true => Ok(()),
        false => {
            error!(
                "Source {} does not exists",
                source.as_ref().to_string_lossy()
            );
            Err(ErrorKind::SourceNotExist.into())
        }
    }
}

fn get_repo_root<P: AsRef<Path>>(path: P) -> Result<PathBuf, GsyncError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path.as_ref())
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .unwrap();

    let path_len = output.stdout.len();
    if path_len == 0 {
        return Err(ErrorKind::SourceNotGitRepo.into());
    } else {
        let bytes = &output.stdout[..path_len - 1]; // remove trailing b'\n'
        return Ok(PathBuf::from(OsStr::from_bytes(bytes)));
    }
}

#[test]
fn test_get_root_repo() {
    let result = get_repo_root(".");
    assert_eq!(result.is_ok(), true);
    assert_eq!(result.unwrap(), PathBuf::from(env!("CARGO_MANIFEST_DIR")));

    let result = get_repo_root("/");
    assert_eq!(result.is_err(), true);
}
