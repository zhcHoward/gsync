use std::convert::From;
use std::error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum GsyncError {
    Io(io::Error),
    Custom(ErrorKind),
}

impl fmt::Display for GsyncError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GsyncError::Io(ref err) => write!(f, "IO error: {}", err),
            GsyncError::Custom(ref err) => write!(f, "{}", err.as_str()),
        }
    }
}

impl error::Error for GsyncError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            GsyncError::Io(ref err) => Some(err),
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

#[derive(Debug)]
pub enum ErrorKind {
    SourceNotExist,
    DestinationInvalid,
    ConfigNotExist,
}

impl ErrorKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match *self {
            ErrorKind::SourceNotExist => "Source folder does not exist",
            ErrorKind::DestinationInvalid => {
                "The format of destination is invalid, cannot parse it"
            }
            ErrorKind::ConfigNotExist => "Config file does not exist",
        }
    }
}
