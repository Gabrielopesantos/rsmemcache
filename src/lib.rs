use std::io;
use std::net::{AddrParseError, SocketAddr, TcpStream};
use std::str::FromStr;

#[allow(dead_code)]
#[derive(Debug)]
pub struct Client {
    // Server address
    server_addr: SocketAddr,
    // Tcp connection
    conn: TcpStream,
    // Socket read/write timeout.
    timeout: u32,
}

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

impl Client {
    pub fn new(server_addr: String) -> Result<Self, ClientConnError> {
        let socket_addr = SocketAddr::from_str(&server_addr)?;
        let tcp_stream = TcpStream::connect(socket_addr)?;

        Ok(Self {
            server_addr: socket_addr,
            conn: tcp_stream,
            timeout: 500,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Client;

    #[test]
    fn invalid_server_addr_returns_err() {
        let result = Client::new(String::from("alksdjasld"));
        match result {
            Ok(_) => panic!("Expected creation of new client to fail"),
            Err(_) => (),
        };
    }
}

