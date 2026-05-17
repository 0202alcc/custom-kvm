use std::fmt;
use std::error::Error as StdError;
use std::io;

/// Custom error type for the KVM system
#[derive(Debug)]
pub enum Error {
    /// Serialization/deserialization errors
    SerializationError(String),
    /// Device-related errors (evdev operations)
    DeviceError(String),
    /// Socket/network-related errors
    SocketError(String),
    /// Configuration file errors
    ConfigError(String),
    /// macOS CGEvent-related errors
    CgEventError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Error::DeviceError(msg) => write!(f, "Device error: {}", msg),
            Error::SocketError(msg) => write!(f, "Socket error: {}", msg),
            Error::ConfigError(msg) => write!(f, "Config error: {}", msg),
            Error::CgEventError(msg) => write!(f, "CGEvent error: {}", msg),
        }
    }
}

impl StdError for Error {}

/// Convert from bincode errors to our Error type
impl From<bincode::Error> for Error {
    fn from(err: bincode::Error) -> Self {
        Error::SerializationError(err.to_string())
    }
}

/// Convert from std::io errors to our Error type
impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::SocketError(err.to_string())
    }
}

/// Custom Result type using our Error type
pub type Result<T> = std::result::Result<T, Error>;
