use regex::Regex;
use ssh2::Session;
use std::env;
use std::io;
use std::net::TcpStream;
use std::path::Path;
use std::process::{Command, Stdio};

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
