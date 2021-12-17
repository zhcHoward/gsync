use crate::error::{ErrorKind, GsyncError};
use log::debug;
use regex::Regex;
use ssh2::Session;
use std::env;
use std::io;
use std::net::TcpStream;
use std::path::Path;

const PRI_KEY_FILES: [&'static str; 4] = ["id_ed25519", "id_rsa", "id_ecdsa", "id_dsa"];

pub struct Destination {
    username: String,
    host: String,
}

impl Destination {
    pub fn new(username: String, host: String) -> Self {
        Destination { username, host }
    }

    pub fn parse_destination<S: AsRef<str>>(destination: S) -> Result<Self, GsyncError> {
        let pattern = Regex::new(r"^(?:(?P<username>[^@]+)@)*(?P<host>.+)$").unwrap();
        match pattern.captures(destination.as_ref()) {
            None => {
                eprintln!("Failed to parse destination {}", destination.as_ref());
                Err(ErrorKind::DestinationInvalid.into())
            }
            Some(caps) => {
                let username = match caps.name("username") {
                    Some(m) => m.as_str().to_string(),
                    None => env::var("USER").unwrap(),
                };
                let host = caps.name("host").unwrap().as_str().to_string();
                Ok(Self::new(username, host))
            }
        }
    }

    pub fn connect(&self) -> io::Result<ssh2::Session> {
        let address = format!("{}:22", self.host);
        let stream = TcpStream::connect(address)?;
        let mut session = Session::new()?;
        session.set_tcp_stream(stream);
        session.handshake()?;

        let result = session.userauth_agent(self.username.as_str());
        if let Ok(_) = result {
            return Ok(session);
        };

        debug!("userauth_agent failed: {:?}", result);
        let home = env::var("HOME").unwrap();
        for fname in PRI_KEY_FILES.iter() {
            let private_key_file = Path::new(&home).join(".ssh").join(fname);
            if !private_key_file.exists() {
                continue;
            }
            let public_key_file = Path::new(&home).join(".ssh").join(format!("{}.pub", fname));
            if !public_key_file.exists() {
                continue;
            }
            let result = session.userauth_pubkey_file(
                self.username.as_str(),
                Some(public_key_file.as_path()),
                private_key_file.as_path(),
                None,
            );
            if let Ok(_) = result {
                return Ok(session);
            };
        }

        debug!("userauth pubkey file failed: {:?}", result);
        let message = format!("{}@{}'s password: ", self.username, self.host);
        let pass = rpassword::read_password_from_tty(Some(message.as_str())).unwrap();
        match session.userauth_password(self.username.as_str(), pass.as_str()) {
            Ok(_) => Ok(session),
            Err(e) => {
                eprintln!("{}", e.message());
                Err(io::Error::new(io::ErrorKind::Other, "Invalid password"))
            }
        }
    }
}
