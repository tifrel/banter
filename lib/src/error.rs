use std::{convert::Into, fmt};

/// Error types that might be encountered while bantering
#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Error {
    /// Deserialization of a message from a peer (or attacker!) failed
    Deserialization,
    /// Parsing of a string failed
    Parsing { from: String, to: String },
    /// Connection to TCP socket failed
    ConnectionFailed { to: String },
    /// Writing to TCP socket failed
    SocketWriteFailed { to: String },
}

impl<T> Into<Result<T, Error>> for Error {
    #[inline]
    fn into(self) -> Result<T, Error> {
        Err(self)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        match self {
            Deserialization => write!(f, "Deserialization error"),
            Parsing { from, to } => {
                write!(f, "Error while trying to parse {} as {}", from, to)
            }
            ConnectionFailed { to } => write!(f, "Connection failed to: {}", to),
            SocketWriteFailed { to } => write!(f, "Writing to socket failed: {}", to),
        }
    }
}

impl std::error::Error for Error {}
