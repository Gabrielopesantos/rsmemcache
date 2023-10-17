#[allow(dead_code)]
mod errors;
mod item;
use crate::{
    errors::{ConnError, OperationError, WriteReadLineError},
    item::Item,
};
use std::io::{self, BufRead, Write};
use std::net::{SocketAddr, TcpStream};
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

const VERB_SET: &str = "set";
const VERB_ADD: &str = "add";
const VERB_REPLACE: &str = "replace";
const VERB_APPEND: &str = "append";
const VERB_PREPEND: &str = "prepend";
const VERB_CAS: &str = "cas";
const VERB_GET: &str = "get";
const VERB_GETS: &str = "gets";
const VERB_DELETE: &str = "delete";
const VERB_INCR: &str = "incr";
const VERB_DECR: &str = "decr";
const VERB_TOUCH: &str = "touch";
const VERB_GAT: &str = "gat";
const VERB_GATS: &str = "gats";
const VERB_STATS: &str = "stats";
const VERB_FLUSH_ALL: &str = "flush_all";
const VERB_VERSION: &str = "version";
const VERB_QUIT: &str = "quit";

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

impl Client {
    pub fn new(server_addr: String, timeout: u32, max_idle_conns: u8) -> Result<Self, ConnError> {
        let socket_addr = SocketAddr::from_str(&server_addr)?;
        let tcp_stream = TcpStream::connect(socket_addr)?;

        let mut server_conns: Vec<Conn> = Vec::new();
        // NOTE: Lazily create connections or start with one?
        let conn = Conn::new(tcp_stream).map_err(|error| {
            ConnError::TcpConnectError(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to create connection: {}", error.to_string()),
            ))
        })?;
        server_conns.push(conn);

        Ok(Self {
            server_addr: socket_addr,
            conns: server_conns,
            timeout: Client::net_timout(timeout),
            max_idle_cons: Client::max_idle_conns(max_idle_conns),
        })
    }

    pub fn ping(&mut self) -> Result<(), OperationError> {
        // TODO: Select server
        match self.conns[0].write_read_line(b"version\r\n") {
            Ok(_) => Ok(()),
            Err(error) => Err(OperationError::IoError(error)),
        }
    }

    // Abstraction `with_key_addr` missing as we only support a single server for now;
    // TODO: Unwraps
    pub fn get(&mut self, key: String) -> Result<Option<Item>, OperationError> {
        if !legal_key(&key) {
            return Err(OperationError::MalformedKeyError);
        }
        let conn = &mut self.conns[0];
        conn.writer
            .write_fmt(format_args!("{} {}\r\n", VERB_GET, key))
            .map_err(|error| OperationError::IoError(WriteReadLineError::WriteError(error)))?;
        conn.writer
            .flush()
            .map_err(|error| OperationError::IoError(WriteReadLineError::FlushError(error)))?;

        // Parse get response
        let mut read_buf: Vec<u8> = Vec::new();
        conn.reader
            .read_until(b'\n', &mut read_buf)
            .map_err(|error| OperationError::IoError(WriteReadLineError::ReadError(error)))?;
        if read_buf.as_slice() == RESULT_END {
            return Ok(None);
        }
        // Scan get response line
        if read_buf.ends_with(CR_LF) {
            read_buf.pop();
            read_buf.pop();
        }
        let mut split = read_buf.split(|&x| x == b' ');
        let _ = split.next(); // NOTE: Ignore first token
        let key = String::from_utf8(split.next().unwrap().to_vec()).map_err(|error| {
            OperationError::CorruptResponseError(format!("could not parse the item key: {}", error))
        })?;
        let flags = String::from_utf8(split.next().unwrap().to_vec()).map_err(|error| {
            OperationError::CorruptResponseError(format!("could not parse flags: {}", error))
        })?;
        let flags = match flags.parse::<u32>() {
            Ok(flags) => flags,
            Err(error) => {
                return Err(OperationError::CorruptResponseError(format!(
                    "could not convert flags into an integer: {}",
                    error
                )))
            }
        };

        let size = String::from_utf8(split.next().unwrap().to_vec()).map_err(|error| {
            OperationError::CorruptResponseError(format!("could not parse size: {}", error))
        })?;

        let size = match size.parse::<u32>() {
            Ok(size) => size,
            Err(error) => {
                return Err(OperationError::CorruptResponseError(format!(
                    "could parse the item value size: {}",
                    error
                )))
            }
        };

        let mut value_buf: Vec<u8> = Vec::new();
        conn.reader
            .read_until(b'\n', &mut value_buf)
            .map_err(|error| {
                OperationError::CorruptResponseError(format!("could not read value: {}", error))
            })?;
        if value_buf.ends_with(CR_LF) {
            value_buf.pop();
            value_buf.pop();
        }

        if size != value_buf.len().try_into().unwrap() {
            return Err(OperationError::CorruptResponseError(String::from(
                "Size does not match value length",
            )));
        }

        return Ok(Some(Item::new(key, value_buf, flags, 0)));
    }

    // NOTE: Item reference?
    pub fn add(&mut self, item: Item) -> Result<(), OperationError> {
        Client::populate_one(&mut self.conns[0], VERB_ADD, item)
    }

    pub fn set(&mut self, item: Item) -> Result<(), OperationError> {
        Client::populate_one(&mut self.conns[0], VERB_SET, item)
    }

    // TODO: returns?
    // NOTE: Populate one what?
    fn populate_one(conn: &mut Conn, verb: &str, item: Item) -> Result<(), OperationError> {
        if !legal_key(&item.key) {
            return Err(OperationError::MalformedKeyError);
        }
        // NOTE: Include all in one write?
        conn.writer
            .write_fmt(format_args!(
                "{} {} {} {} {}\r\n",
                verb,
                item.key,
                item.flags,
                item.expiration,
                item.value.len(),
            ))
            .map_err(|error| OperationError::IoError(WriteReadLineError::WriteError(error)))?;
        conn.writer
            .write_all(&item.value)
            .map_err(|error| OperationError::IoError(WriteReadLineError::WriteError(error)))?;
        conn.writer
            .write_all(b"\r\n")
            .map_err(|error| OperationError::IoError(WriteReadLineError::WriteError(error)))?;
        conn.writer
            .flush()
            .map_err(|error| OperationError::IoError(WriteReadLineError::FlushError(error)))?;
        let mut read_buf: Vec<u8> = Vec::new();
        conn.reader
            .read_until(b'\n', &mut read_buf)
            .map_err(|error| OperationError::IoError(WriteReadLineError::ReadError(error)))?;

        match read_buf.as_slice() {
            RESULT_STORED => Ok(()),
            RESULT_NOT_STORED => Err(OperationError::NotStoredError),
            RESULT_EXISTS => Err(OperationError::CASConflictError),
            RESULT_NOT_FOUND => Err(OperationError::CacheMissError),
            _ => Err(OperationError::CorruptResponseError(format!(
                "Unexpected response from server: {}",
                String::from_utf8(read_buf).unwrap(), // TODO: Unwrap
            ))),
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
    // stream: TcpStream, // NOTE: Is this needed?
    reader: io::BufReader<TcpStream>,
    writer: io::BufWriter<TcpStream>,
}

impl Conn {
    fn new(stream: TcpStream) -> Result<Self, std::io::Error> {
        Ok(Self {
            reader: io::BufReader::new(stream.try_clone()?),
            writer: io::BufWriter::new(stream),
        })
    }

    fn write_read_line(&mut self, write_buf: &[u8]) -> Result<Vec<u8>, WriteReadLineError> {
        self.writer
            .write_all(write_buf)
            .map_err(WriteReadLineError::WriteError)?;
        self.writer
            .flush()
            .map_err(WriteReadLineError::FlushError)?;
        let mut read_buf: Vec<u8> = Vec::new();
        self.reader
            .read_until(b'\n', &mut read_buf)
            .map_err(WriteReadLineError::ReadError)?;
        Ok(read_buf)
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
    use crate::{errors::ConnError, item::Item};

    use super::Client;
    const LOCALHOST_TCP_ADDR: &str = "127.0.0.1:11211";

    #[test]
    fn invalid_server_addr_returns_err() {
        let result = Client::new(String::from("alksdjasld"), 0, 0);
        match result {
            Ok(_) => panic!("Expected creation of new client to fail"),
            Err(error) => match error {
                ConnError::AddrParseError(_) => (), // Expected error,
                _ => panic!("Unexpected error. Got: {:?}", error),
            },
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

        // NOTE: Clone?
        let item = match client.get(item_key.clone()) {
            Ok(item) => item,
            Err(error) => panic!("Expected item to be successfully retrieved: {}", error),
        };

        if let Some(item) = item {
            if item.value != item_value {
                panic!("Expected value to be red")
            }
            if item.flags != item_flags {
                panic!("Expected flags to be 0")
            }
        } else {
            panic!("Expected an item")
        }
    }
}
