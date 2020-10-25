use regex::Regex;
use ssh2::Session;
use std::char;
use std::env;
use std::io;
use std::net::TcpStream;
use std::path::Path;
use std::process::{Command, Stdio};
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
}

impl Destination {
    pub fn new(username: String, host: String) -> Self {
        return Destination { username, host };
    }

    pub fn connect(&self) -> io::Result<ssh2::Session> {
        let address = format!("{}:22", self.host);
        let stream = TcpStream::connect(address)?;
        let mut session = Session::new()?;
        session.set_tcp_stream(stream);
        session.handshake()?;

        // Try to authenticate with the first identity in the agent.
        match session.userauth_agent(&self.username) {
            Ok(_) => Ok(session),
            Err(_) => {
                // println!("userauth_agent error: {:?}", e);
                let message = format!("{}@{}'s password: ", self.username, self.host);
                let pass = rpassword::read_password_from_tty(Some(&message))?;
                session.userauth_password(&self.username, &pass)?;
                Ok(session)
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

fn sort_commits<'a>(c1: &'a str, c2: &'a str, repo: &str) -> (&'a str, &'a str) {
    let status = Command::new("git")
        .args(&["-C", repo, "merge-base", "--is-ancestor", c1, c2])
        .status()
        .unwrap();
    match status.success() {
        true => (c1, c2),
        false => (c2, c1),
    }
}

pub fn find_changes_between_commits(c1: &str, c2: &str, repo: &str) -> Vec<String> {
    let (c1, c2) = sort_commits(c1, c2, repo);
    let result = Command::new("git")
        .args(&["-C", repo, "diff", "--name-status", c1, c2])
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
