use std::fmt::format;
use std::io::{self, BufRead, Write};
use std::net::{AddrParseError, SocketAddr, TcpStream};
use std::str::FromStr;

#[allow(dead_code)]
#[derive(Debug)]
pub struct Client {
    // Server address
    server_addr: SocketAddr,
    // Server connection
    conn: Conn,
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
            conn: Conn::new(tcp_stream),
            timeout: 500,
        })
    }

    pub fn ping(&mut self) -> Result<(), &'static str> {
        match self.conn.write_read_line(b"version\r\n") {
            Ok(_) => Ok(()),
            Err(_) => Err("Failed to ping server"),
        }
    }
}

#[derive(Debug)]
struct Conn {
    stream: TcpStream,
    reader: io::BufReader<TcpStream>,
    writer: io::BufWriter<TcpStream>,
}

impl Conn {
    fn new(stream: TcpStream) -> Self {
        // FIXME: try_clone
        Self {
            stream: stream.try_clone().expect("Clone failed!"), // NOTE: Needed?
            reader: io::BufReader::new(stream.try_clone().expect("Clone failed!")),
            writer: io::BufWriter::new(stream.try_clone().expect("Clone failed!")),
        }
    }

    fn write_read_line(&mut self, write_buf: &[u8]) -> Result<Vec<u8>, &'static str> {
        if let Err(_) = self.writer.write_all(write_buf) {
            return Err("Could not write buffer to stream");
        }

        let mut read_buf: Vec<u8> = Vec::new();
        match self.reader.read_until(b'\n', &mut read_buf) {
            Ok(bytes_read) => {
                print!("Successfully read {} bytes", bytes_read);
                Ok(read_buf)
            }
            Err(_) => return Err("Could not read from stream"),
        }
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
