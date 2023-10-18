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
    CacheMissError,
    CASConflictError,
    NotStoredError,
    ServerError,
    ClientError(String),
    NoStatsError,
    MalformedKeyError,
    NoServersError,
    CorruptResponseError(String),
    IoError(WriteReadLineError),
}

impl std::fmt::Display for OperationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationError::CacheMissError => {
                write!(f, "memcache: cache miss error")
            }
            OperationError::CASConflictError => {
                write!(f, "memcache: CAS conflict error")
            }
            OperationError::NotStoredError => {
                write!(f, "memcache: not stored error")
            }
            OperationError::ServerError => {
                write!(f, "memcache: server error")
            }
            OperationError::ClientError(error_msg) => {
                write!(f, "memcache: client error: {}", error_msg)
            }
            OperationError::NoStatsError => {
                write!(f, "memcache: no stats error")
            }
            OperationError::MalformedKeyError => {
                write!(f, "memcache: malformed key error")
            }
            OperationError::NoServersError => {
                write!(f, "memcache: no servers error")
            }
            OperationError::CorruptResponseError(error_msg) => {
                write!(f, "memcache: corrupt response error: {}", error_msg)
            }
            OperationError::IoError(error) => {
                write!(f, "memcache: IO error: {}", error)
            }
        }
    }
}

impl std::error::Error for OperationError {}

#[derive(Debug)]
pub enum WriteReadLineError {
    WriteError(io::Error),
    FlushError(io::Error),
    ReadError(io::Error),
}

impl std::fmt::Display for WriteReadLineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteReadLineError::WriteError(error) => {
                write!(f, "Could not write to buffer: {}", error)
            }
            WriteReadLineError::FlushError(error) => {
                write!(f, "Could not flush the buffer to server: {}", error)
            }
            WriteReadLineError::ReadError(error) => {
                write!(f, "Could not read from server: {}", error)
            }
        }
    }
}
