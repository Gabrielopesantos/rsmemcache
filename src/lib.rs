mod item;

use crate::item::Item;
use std::io::{self, BufRead, Write};
use std::net::{AddrParseError, SocketAddr, TcpStream};
use std::str::FromStr;

const DEFAULT_NET_TIMEOUT: u32 = 500;
const DEFAULT_MAX_IDLE_CONNS: u8 = 2;

const CR_LF: &[u8] = b"\r\n";
const RESULT_OK: &[u8] = b"OK\r\n";
const RESULT_STORED: &[u8] = b"STORED\r\n";
const RESULT_NOT_STORED: &[u8] = b"NOT_STORED\r\n";
const RESULT_EXISTS: &[u8] = b"EXISTS\r\n";
const RESULT_NOT_FOUND: &[u8] = b"NOT_FOUND\r\n";
const RESULT_DELETED: &[u8] = b"DELETED\r\n";
const RESULT_END: &[u8] = b"END\r\n";
const RESULT_TOUCHED: &[u8] = b"TOUCHED\r\n";

const VERB_SET: &[u8] = b"set";
const VERB_ADD: &[u8] = b"add";
const VERB_REPLACE: &[u8] = b"replace";
const VERB_APPEND: &[u8] = b"append";
const VERB_PREPEND: &[u8] = b"prepend";
const VERB_CAS: &[u8] = b"cas";
const VERB_GET: &str = "get";
const VERB_GETS: &[u8] = b"gets";
const VERB_DELETE: &[u8] = b"delete";
const VERB_INCR: &[u8] = b"incr";
const VERB_DECR: &[u8] = b"decr";
const VERB_TOUCH: &[u8] = b"touch";
const VERB_GAT: &[u8] = b"gat";
const VERB_GATS: &[u8] = b"gats";
const VERB_STATS: &[u8] = b"stats";
const VERB_FLUSH_ALL: &[u8] = b"flush_all";
const VERB_VERSION: &[u8] = b"version";
const VERB_QUIT: &[u8] = b"quit";

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

    // Abstraction `with_key_addr` missing as we only support a single server for now;
    pub fn get(&mut self, key: String) -> Result<Item, &'static str> {
        if !legal_key(&key) {
            return Err("Invalid item key");
        }
        let conn = &mut self.conns[0];
        conn.writer
            .write_fmt(format_args!("{} {}\r\n", VERB_GET, key))
            .map_err(|_| "Could not write get command")?;
        conn.writer
            .flush()
            .map_err(|_| "Could not send get command to server")?;

        // Parse get response
        let mut read_buf: Vec<u8> = Vec::new();
        conn.reader
            .read_until(b'\n', &mut read_buf)
            .map_err(|_| "Could not read server response")?;
        if read_buf.as_slice() == RESULT_END {
            // Different behavior from gomemcache
            return Err("Item not found");
        }
        // Scan get response line
        if read_buf.ends_with(CR_LF) {
            read_buf.pop();
            read_buf.pop();
        }
        let mut split = read_buf.split(|&x| x == b' ');
        let _ = split.next(); // NOTE: Ignore first token
        let key = String::from_utf8(split.next().unwrap().to_vec())
            .map_err(|_| "Could not parse key: {}")?;
        let flags = String::from_utf8(split.next().unwrap().to_vec())
            .map_err(|_| "Could not parse flags")?;
        let flags = match flags.parse::<u32>() {
            Ok(flags) => flags,
            Err(_) => return Err("Could not parse flags"),
        };

        let size = String::from_utf8(split.next().unwrap().to_vec())
            .map_err(|_| "Could not parse size")?;

        let size = match size.parse::<u32>() {
            Ok(size) => size,
            Err(_) => return Err("Could not parse the item value size"),
        };

        let mut value_buf: Vec<u8> = Vec::new();
        conn.reader
            .read_until(b'\n', &mut value_buf)
            .map_err(|_| "Could not read value")?;
        if value_buf.ends_with(CR_LF) {
            value_buf.pop();
            value_buf.pop();
        }

        // NOTE: Unwrap
        if size != value_buf.len().try_into().unwrap() {
            return Err("Size does not match value length");
        }

        return Ok(Item::new(key, value_buf, flags, 0));
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
        // Errors
        match read_buf.as_slice() {
            RESULT_STORED => Ok(()),
            RESULT_NOT_STORED => Err("Item not stored"),
            RESULT_EXISTS => Err("Item already exists"),
            RESULT_NOT_FOUND => Err("Item not found"),
            _ => Err("Unknown error"),
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
            return Err("Could not write to buffer");
        }
        if let Err(_) = self.writer.flush() {
            return Err("Could not send buffer to server");
        }

        let mut read_buf: Vec<u8> = Vec::new();
        match self.reader.read_until(b'\n', &mut read_buf) {
            Ok(bytes_read) => {
                print!("Successfully read {} bytes", bytes_read);
                Ok(read_buf)
            }
            Err(_) => return Err("Could not read from server"),
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

        // NOTE: Expiration 5 so tests don't fail on subsequent runs;
        let item_key = String::from("color");
        let item_value = Vec::from("red");
        let item_flags = 32;
        let item = Item::new(item_key.clone(), item_value.clone(), item_flags, 5);
        if let Err(_) = client.add(item) {
            panic!("Expected item to be successfully persisted")
        }

        // Clone?
        let item = match client.get(item_key.clone()) {
            Ok(item) => item,
            Err(error) => panic!("Expected item to be successfully retrieved: {}", error),
        };

        if item.value != item_value {
            panic!("Expected value to be red")
        }
        if item.flags != item_flags {
            panic!("Expected flags to be 0")
        }
    }
}
