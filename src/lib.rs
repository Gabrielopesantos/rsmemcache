mod item;

use crate::item::Item;
use std::io::{self, BufRead, Write};
use std::net::{AddrParseError, SocketAddr, TcpStream};
use std::str::FromStr;

const DEFAULT_NET_TIMEOUT: u32 = 500;
const DEFAULT_MAX_IDLE_CONNS: u8 = 2;

#[allow(dead_code)]
#[derive(Debug)]
pub struct Client {
    // Server address
    server_addr: SocketAddr,
    // Server connections
    conns: Vec<Conn>,
    // Socket read/write timeout.
    timeout: u32,
    // Max idle connections
    max_idle_cons: u8,
}

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

impl Client {
    pub fn new(
        server_addr: String,
        timeout: u32,
        max_idle_conns: u8,
    ) -> Result<Self, ClientConnError> {
        let socket_addr = SocketAddr::from_str(&server_addr)?;
        let tcp_stream = TcpStream::connect(socket_addr)?;

        let mut server_conns: Vec<Conn> = Vec::new();
        // NOTE: Lazily create connections or start with one?
        server_conns.push(Conn::new(tcp_stream));

        Ok(Self {
            server_addr: socket_addr,
            conns: server_conns,
            timeout: Client::net_timout(timeout),
            max_idle_cons: Client::max_idle_conns(max_idle_conns),
        })
    }

    // TODO: Error
    pub fn ping(&mut self) -> Result<(), &'static str> {
        // TODO: Select server
        match self.conns[0].write_read_line(b"version\r\n") {
            Ok(_) => Ok(()),
            Err(_) => Err("Failed to ping server"),
        }
    }

    // TODO: Error
    // NOTE: Item reference?
    pub fn add(&mut self, item: Item) -> Result<(), &'static str> {
        Client::populate_one(&mut self.conns[0], "add", item)
    }

    // TODO: Error
    // TODO: returns?
    // NOTE: Populate one what?
    fn populate_one(conn: &mut Conn, verb: &str, item: Item) -> Result<(), &'static str> {
        if !legal_key(&item.key) {
            return Err("Invalid item key");
        }
        // NOTE: Include all in one write?
        if let Err(_) = conn.writer.write_fmt(format_args!(
            "{} {} {} {} {}\r\n",
            verb,
            item.key,
            item.flags,
            item.expiration,
            item.value.len(),
        )) {
            return Err("Could write set item command");
        }
        if let Err(_) = conn.writer.write_all(&item.value) {
            return Err("Could write item");
        }
        if let Err(_) = conn.writer.write_all(b"\r\n") {
            return Err("Could write limiter");
        }
        if let Err(_) = conn.writer.flush() {
            return Err("Could not send item to server");
        }
        let mut read_buf: Vec<u8> = Vec::new();
        if let Err(_) = conn.reader.read_until(b'\n', &mut read_buf) {
            return Err("Could not read server message");
        }
        if let Ok(line) = String::from_utf8(read_buf.clone()) {
            match line.trim() {
                "STORED" => Ok(()),
                _ => Err("TODO"),
            }
        } else {
            Err("Could not parse the returned message")
        }
    }

    fn net_timout(input_value: u32) -> u32 {
        match input_value {
            0 => DEFAULT_NET_TIMEOUT,
            _ => input_value,
        }
    }

    fn max_idle_conns(input_value: u8) -> u8 {
        match input_value {
            0 => DEFAULT_MAX_IDLE_CONNS,
            _ => input_value,
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
        // FIXME: try_clone / expect
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
        if let Err(_) = self.writer.flush() {
            return Err("Could not send version command to server");
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

fn legal_key(key: &String) -> bool {
    if key.len() > 250 {
        false;
    }
    true
}

#[cfg(test)]
mod tests {
    use crate::item::Item;

    use super::Client;
    const LOCALHOST_TCP_ADDR: &str = "127.0.0.1:11211";

    #[test]
    fn invalid_server_addr_returns_err() {
        let result = Client::new(String::from("alksdjasld"), 0, 0);
        match result {
            Ok(_) => panic!("Expected creation of new client to fail"),
            Err(_) => (),
        };
    }

    #[test]
    fn test_local_host() {
        let mut client = match Client::new(String::from(LOCALHOST_TCP_ADDR), 0, 0) {
            Ok(client) => client,
            Err(error) => panic!("Could not connect to local server: {:?}", error),
        };

        if let Err(_) = client.ping() {
            panic!("Expected ping to succeed")
        }

        // NOTE: Expiration 1 so tests don't fail on subsequent runs;
        let item = Item::new(String::from("color"), Vec::from("red"), 0, 1);
        if let Err(_) = client.add(item) {
            panic!("Expected item to be successfully persisted")
        }
    }
}
