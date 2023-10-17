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

impl std::error::Error for ConnError {}

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

#[derive(Debug)]
pub enum OperationError {
    CacheMissError(String),
    CASConflictError(String),
    NotStoredError(String),
    ServerError(String),
    NoStatsError(String),
    MalformedKeyError(String),
    NoServersError(String),
    IoError(WriteReadLineError),
}

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
