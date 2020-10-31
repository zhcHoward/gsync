use regex::Regex;
use serde_json;
use ssh2::Session;
use std::char;
use std::env;
use std::ffi::OsStr;
use std::io;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output, Stdio};
use std::str;

pub fn validate_source<P: AsRef<Path>>(source: P) -> bool {
    if !source.as_ref().exists() {
        eprintln!(
            "Source {} does not exists",
            source.as_ref().to_string_lossy()
        );
        return false;
    }

    match is_git_repo(&source) {
        Err(e) => {
            eprintln!("Error while validating source: {:?}", e);
            false
        }
        Ok(ret) => {
            if !ret {
                eprintln!(
                    "{} is not a git repository",
                    source.as_ref().to_string_lossy()
                );
                false
            } else {
                true
            }
        }
    }
}

pub struct Destination {
    username: String,
    host: String,
    private_key_file: PathBuf,
    public_key_file: PathBuf,
}

impl Destination {
    pub fn new(username: String, host: String) -> Self {
        let home = env::var("HOME").unwrap();
        let private_key_file = Path::new(&home).join(".ssh/id_rsa");
        let public_key_file = Path::new(&home).join(".ssh/id_rsa.pub");
        return Destination {
            username,
            host,
            private_key_file,
            public_key_file,
        };
    }

    pub fn connect(&self) -> io::Result<ssh2::Session> {
        let address = format!("{}:22", self.host);
        let stream = TcpStream::connect(address)?;
        let mut session = Session::new()?;
        session.set_tcp_stream(stream);
        session.handshake()?;

        // Try to authenticate with the first identity in the agent.
        match session.userauth_agent(self.username.as_str()) {
            Ok(_) => Ok(session),
            Err(e) => {
                println!("userauth_agent failed: {:?}", e);
            }
        };

        match session.userauth_pubkey_file(
            self.username.as_str(),
            Some(self.public_key_file.as_path()),
            self.private_key_file.as_path(),
            None,
        ) {
            Ok(_) => Ok(session),
            Err(e) => println!("userauth pubkey file failed: {:?}", e),
        };

        let message = format!("{}@{}'s password: ", self.username, self.host);
        let pass = rpassword::read_password_from_tty(Some(message.as_str())).unwrap();
        match session.userauth_password(self.username.as_str(), pass.as_str()) {
            Ok(_) => Ok(session),
            Err(e) => {
                println!("userauth password failed: {:?}", e);
                Err(io::Error::new(io::ErrorKind::Other, "Invalid password"))
            }
        }

    }
}

pub fn parse_destination<S: AsRef<str>>(destination: S) -> Option<Destination> {
    let pattern = Regex::new(r"^(?:(?P<username>[^@]+)@)*(?P<host>.+)$").unwrap();
    match pattern.captures(destination.as_ref()) {
        None => {
            eprintln!("Failed to parse destination {}", destination.as_ref());
            None
        }
        Some(caps) => {
            let username = match caps.name("username") {
                Some(m) => m.as_str().to_string(),
                None => env::var("USER").unwrap(),
            };
            let host = caps.name("host").unwrap().as_str().to_string();
            Some(Destination::new(username, host))
        }
    }
}

fn sort_commits<'a, P: AsRef<Path>>(c1: &'a str, c2: &'a str, repo: P) -> (&'a str, &'a str) {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo.as_ref())
        .args(&["merge-base", "--is-ancestor", c1, c2])
        .status()
        .unwrap();
    match status.success() {
        true => (c1, c2),
        false => (c2, c1),
    }
}

pub fn find_changes_between_commits<P: AsRef<Path>>(c1: &str, c2: &str, repo: P) -> Vec<String> {
    let (c1, c2) = sort_commits(c1, c2, &repo);
    let result = Command::new("git")
        .arg("-C")
        .arg(repo.as_ref())
        .args(&["diff", "--name-status", c1, c2])
        .output();
    match result {
        Err(e) => {
            eprintln!("Error while find changes between commits: {:?}", e);
            vec![]
        }
        Ok(output) => {
            if output.status.success() {
                let output = str::from_utf8(&output.stdout).unwrap();
                output
                    .lines()
                    .filter_map(|line| {
                        let l: Vec<_> = line.split(char::is_whitespace).collect();
                        match l[0] {
                            "D" => None,
                            _ => Some(l[1].to_owned()),
                        }
                    })
                    .collect()
            } else {
                eprintln!(
                    "git command failed, stdout:\n{}stderr:\n{}",
                    str::from_utf8(&output.stdout).unwrap(),
                    str::from_utf8(&output.stderr).unwrap(),
                );
                vec![]
            }
        }
    }
}

#[test]
fn test_find_changes_between_commits() {
    let result = find_changes_between_commits(
        "15cfed3f",
        "764c3656",
        "/Users/howard/Workspaces/Rust/gsync",
    );
    assert_eq!(vec!["LICENSE"], result);
}

// fn find_remote_path<P: AsRef<Path>>(local_path: P, config: &serde_json::Value) -> PathBuf {}

fn scp<P: AsRef<Path>>(fpath: &str, repo: P, config: &serde_json::Value, session: ssh2::Session) {
    let local_path = repo.as_ref().join(&fpath);
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
