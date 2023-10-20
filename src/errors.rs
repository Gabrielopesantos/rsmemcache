#[allow(dead_code)]
use std::io::{self};
use std::net::AddrParseError;

#[derive(Debug)]
pub enum ConnError {
    AddrParseError(AddrParseError),
    TcpConnectError(io::Error),
}

impl From<AddrParseError> for ConnError {
    fn from(error: AddrParseError) -> Self {
        Self::AddrParseError(error)
    }
}

impl From<io::Error> for ConnError {
    fn from(error: io::Error) -> Self {
        Self::TcpConnectError(error)
    }
}

impl std::fmt::Display for ConnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnError::AddrParseError(error) => {
                write!(f, "could not parse the server address: {}", error)
            }
            ConnError::TcpConnectError(error) => {
                write!(f, "could reach the server: {}", error)
            }
        }
    }
}

impl std::error::Error for ConnError {}

#[derive(Debug)]
pub enum OperationError {
    CacheMiss,
    CASConflict,
    NotStored,
    Server,
    Client(String),
    NoStats,
    MalformedKey,
    NoServers,
    CorruptResponse(String),
    Io(WriteReadLineError),
}

impl std::fmt::Display for OperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationError::CacheMiss => {
                write!(f, "memcache: cache miss error")
            }
            OperationError::CASConflict => {
                write!(f, "memcache: CAS conflict error")
            }
            OperationError::NotStored => {
                write!(f, "memcache: not stored error")
            }
            OperationError::Server => {
                write!(f, "memcache: server error")
            }
            OperationError::Client(error_msg) => {
                write!(f, "memcache: client error: {}", error_msg)
            }
            OperationError::NoStats => {
                write!(f, "memcache: no stats error")
            }
            OperationError::MalformedKey => {
                write!(f, "memcache: malformed key error")
            }
            OperationError::NoServers => {
                write!(f, "memcache: no servers error")
            }
            OperationError::CorruptResponse(error_msg) => {
                write!(f, "memcache: corrupt response error: {}", error_msg)
            }
            OperationError::Io(error) => {
                write!(f, "memcache: IO error: {}", error)
            }
        }
    }
}

impl std::error::Error for OperationError {}

#[derive(Debug)]
pub enum WriteReadLineError {
    Write(io::Error),
    Flush(io::Error),
    Read(io::Error),
}

impl std::fmt::Display for WriteReadLineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteReadLineError::Write(error) => {
                write!(f, "Could not write to buffer: {}", error)
            }
            WriteReadLineError::Flush(error) => {
                write!(f, "Could not flush the buffer to server: {}", error)
            }
            WriteReadLineError::Read(error) => {
                write!(f, "Could not read from server: {}", error)
            }
        }
    }
}
