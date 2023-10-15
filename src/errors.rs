#[allow(dead_code)]
use std::io::{self};
use std::net::AddrParseError;

#[derive(Debug)]
pub enum ClientConnError {
    AddrParseError(AddrParseError),
    TcpConnectError(io::Error),
}

impl From<AddrParseError> for ClientConnError {
    fn from(error: AddrParseError) -> Self {
        Self::AddrParseError(error)
    }
}

impl From<io::Error> for ClientConnError {
    fn from(error: io::Error) -> Self {
        Self::TcpConnectError(error)
    }
}

pub enum ClientOperationError {
    CacheMissError(String),
    CASConflictError(String),
    NotStoredError(String),
    ServerError(String),
    NoStatsError(String),
    MalformedKeyError(String),
    NoServersError(String),
}