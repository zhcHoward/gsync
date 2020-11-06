use crate::{
    commit,
    config::Config,
    destination::Destination,
    error::{ErrorKind, GsyncError},
    Opt,
};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct Gsync {
    config: PathBuf,
    source: PathBuf,
    destination: Destination,
    commits: Vec<String>,
}

impl Gsync {
    pub fn from_options(opts: Opt) -> Result<Self, GsyncError> {
        let valid = validate_source(&opts.source)?;
        if !valid {
            return Err(GsyncError::Custom(ErrorKind::SourceNotExist));
        }
        let destination = Destination::parse_destination(&opts.destination)?;
        let config = Config::parse_config(&opts.config)?;
        let commits = &opts
            .commits
            .iter()
            .map(|raw_commit| commit::parse_commit(raw_commit, &opts.source));
        unimplemented!();
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
