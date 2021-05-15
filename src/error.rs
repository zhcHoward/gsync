use serde_json;
use std::convert::From;
use std::error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum GsyncError {
    Io(io::Error),
    Custom(ErrorKind),
    Serde(serde_json::Error),
}

impl GsyncError {
    pub fn custom(kind: ErrorKind) -> Self {
        GsyncError::Custom(kind)
    }

    pub fn io(error: io::Error) -> Self {
        GsyncError::Io(error)
    }
}

impl fmt::Display for GsyncError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GsyncError::Io(ref err) => write!(f, "IO error: {}", err),
            GsyncError::Custom(ref err) => write!(f, "{}", err.as_str()),
            GsyncError::Serde(ref err) => err.fmt(f),
        }
    }
}

impl error::Error for GsyncError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            GsyncError::Io(ref err) => Some(err),
            GsyncError::Serde(ref err) => Some(err),
            GsyncError::Custom(..) => None,
        }
    }
}

impl From<io::Error> for GsyncError {
    fn from(err: io::Error) -> Self {
        GsyncError::Io(err)
    }
}

impl From<ErrorKind> for GsyncError {
    fn from(kind: ErrorKind) -> Self {
        GsyncError::Custom(kind)
    }
}

impl From<serde_json::Error> for GsyncError {
    fn from(error: serde_json::Error) -> Self {
        GsyncError::Serde(error)
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    SourceNotExist,
    DestinationInvalid,
    ConfigNotExist,
    SourceNotGitRepo,
}

impl ErrorKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match *self {
            ErrorKind::SourceNotExist => "Source folder does not exist",
            ErrorKind::DestinationInvalid => {
                "The format of destination is invalid, cannot parse it"
            }
            ErrorKind::ConfigNotExist => "Config file does not exist",
            ErrorKind::SourceNotGitRepo => "Source is not a git repository",
        }
    }
}
