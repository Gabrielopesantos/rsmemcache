#![allow(dead_code)]
mod errors;
mod item;
mod selector;

use crate::{
    errors::{ConnError, OperationError, WriteReadLineError},
    item::Item,
    selector::{ServerList, ServerSelector},
};
use std::net::{SocketAddr, TcpStream};
use std::str::FromStr;
use std::{
    collections::HashMap,
    io::{self, BufRead, Read, Write},
};

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
const RESULT_CLIENT_ERROR_PREFIX: &[u8] = b"CLIENT_ERROR ";

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

#[derive(Debug)]
// pub struct Client<T: ServerSelector> {
pub struct Client<'a> {
    // Server Selector
    // selector: T,
    selector: ServerList,
    // Socket read/write timeout.
    timeout: u32,
    // Free connections
    free_conns: HashMap<String, Vec<Conn<'a>>>,
    // Max idle connections
    max_idle_cons: u8,
}

// impl<T: ServerSelector> Client<T> {
impl<'a> Client<'a> {
    pub fn new(servers: Vec<String>) -> Result<Self, OperationError> {
        let mut selector = ServerList::new();
        selector.set_servers(servers)?;
        Ok(Self::new_from_selector(selector))
    }

    // pub fn new_from_selector(selector: T) -> Self {
    pub fn new_from_selector(selector: ServerList) -> Self {
        Self {
            selector,
            timeout: DEFAULT_NET_TIMEOUT,
            free_conns: HashMap::new(),
            max_idle_cons: DEFAULT_MAX_IDLE_CONNS,
        }
    }

    // TODO: addr
    fn put_free_conn(&mut self, addr: SocketAddr, conn: Conn<'a>) {
        let addr_str = addr.to_string();
        match self.free_conns.get_mut(&addr_str) {
            Some(addr_conns) => addr_conns.push(conn),
            None => {
                let mut addr_conns = Vec::new();
                addr_conns.push(conn);
                self.free_conns.insert(addr_str, addr_conns);
            }
        }
    }

    // TODO: addr
    fn get_free_conn(&mut self, addr: SocketAddr) -> Option<Conn<'a>> {
        match self.free_conns.get_mut(&addr.to_string()) {
            Some(addr_conns) => addr_conns.pop(),
            None => None,
        }
    }
    // TODO: addr
    fn get_conn(&mut self, addr: SocketAddr) -> Result<Conn, OperationError> {
        // TODO: Clone
        if let Some(conn) = self.get_free_conn(addr) {
            // TODO: Extend deadline
            return Ok(conn);
        }
        // let socket_addr = SocketAddr::from_str(&server_addr)?;
        let tcp_stream = TcpStream::connect(addr).map_err(|_| OperationError::NoServers)?; // TODO: Err

        // let mut server_conns: Vec<Conn> = Vec::new();
        Ok(Conn::new(tcp_stream, self).map_err(|_| OperationError::NoServers)?)
        // TODO: Err
    }

    pub fn ping(&mut self) -> Result<(), OperationError> {
        for addr in self.selector.addrs.iter() {
            let conn = self.get_conn(*addr)?;
            Self::internal_ping(&conn);
            // self.put_free_conn(*addr, conn);
        }
        Ok(())
    }

    fn internal_ping(conn: &Conn<'a>) -> Result<(), OperationError> {
        match conn.write_read_line(format!("{}\r\n", VERB_VERSION).as_bytes()) {
            Ok(_) => Ok(()),
            Err(error) => Err(OperationError::Io(error)),
        }
    }
}
// Abstraction `with_key_addr` missing as we only support a single server for now;
// TODO: Unwraps
//     pub fn get(&mut self, key: String) -> Result<Option<Item>, OperationError> {
//         if !legal_key(&key) {
//             return Err(OperationError::MalformedKey);
//         }
//         let conn = &mut self.conns[0];
//         conn.writer
//             .write_fmt(format_args!("{} {}\r\n", VERB_GET, key))
//             .map_err(|error| OperationError::Io(WriteReadLineError::Write(error)))?;
//         conn.writer
//             .flush()
//             .map_err(|error| OperationError::Io(WriteReadLineError::Flush(error)))?;
//
//         // Parse get response
//         let mut read_buf: Vec<u8> = Vec::new();
//         conn.reader
//             .read_until(b'\n', &mut read_buf)
//             .map_err(|error| OperationError::Io(WriteReadLineError::Read(error)))?;
//         if read_buf.as_slice() == RESULT_END {
//             return Ok(None);
//         }
//         // Scan get response line
//         if read_buf.ends_with(CR_LF) {
//             read_buf.pop();
//             read_buf.pop();
//         }
//         let mut split = read_buf.split(|&x| x == b' ');
//         let _ = split.next(); // NOTE: Ignore first token
//         let key = String::from_utf8(split.next().unwrap().to_vec()).map_err(|error| {
//             OperationError::CorruptResponse(format!("could not parse the item key: {}", error))
//         })?;
//         let flags = String::from_utf8(split.next().unwrap().to_vec()).map_err(|error| {
//             OperationError::CorruptResponse(format!("could not parse flags: {}", error))
//         })?;
//         let flags = match flags.parse::<u32>() {
//             Ok(flags) => flags,
//             Err(error) => {
//                 return Err(OperationError::CorruptResponse(format!(
//                     "could not convert flags into an integer: {}",
//                     error
//                 )))
//             }
//         };
//
//         let size = String::from_utf8(split.next().unwrap().to_vec()).map_err(|error| {
//             OperationError::CorruptResponse(format!("could not parse size: {}", error))
//         })?;
//
//         let size = match size.parse::<u32>() {
//             Ok(size) => size,
//             Err(error) => {
//                 return Err(OperationError::CorruptResponse(format!(
//                     "could parse the item value size: {}",
//                     error
//                 )))
//             }
//         };
//
//         let mut value_buf = vec![0; size as usize + 2];
//         conn.reader.read_exact(&mut value_buf).map_err(|error| {
//             OperationError::CorruptResponse(format!("could not read value: {}", error))
//         })?;
//         if !value_buf.ends_with(CR_LF) {
//             return Err(OperationError::CorruptResponse(
//                 "corrupt get result read".to_string(),
//             ));
//         } else {
//             value_buf.pop();
//             value_buf.pop();
//         }
//
//         // NOTE: Still missing read `END\r\n`
//         let _ = conn.reader.read_until(b'\n', &mut Vec::new());
//
//         Ok(Some(Item::new(key, value_buf, flags, 0)))
//     }
//
//     // NOTE: Item reference?
//     pub fn add(&mut self, item: Item) -> Result<(), OperationError> {
//         Self::populate_one(&mut self.conns[0], VERB_ADD, item)
//     }
//
//     pub fn set(&mut self, item: Item) -> Result<(), OperationError> {
//         Self::populate_one(&mut self.conns[0], VERB_SET, item)
//     }
//
//     pub fn replace(&mut self, item: Item) -> Result<(), OperationError> {
//         Self::populate_one(&mut self.conns[0], VERB_REPLACE, item)
//     }
//
//     pub fn append(&mut self, item: Item) -> Result<(), OperationError> {
//         Self::populate_one(&mut self.conns[0], VERB_APPEND, item)
//     }
//
//     pub fn prepend(&mut self, item: Item) -> Result<(), OperationError> {
//         Self::populate_one(&mut self.conns[0], VERB_PREPEND, item)
//     }
//
//     pub fn increment(&mut self, key: String, delta: u64) -> Result<u64, OperationError> {
//         Self::incr_decr(&mut self.conns[0], VERB_INCR, key, delta)
//     }
//
//     pub fn decrement(&mut self, key: String, delta: u64) -> Result<u64, OperationError> {
//         Self::incr_decr(&mut self.conns[0], VERB_DECR, key, delta)
//     }
//
//     pub fn delete(&mut self, key: String) -> Result<(), OperationError> {
//         Self::write_expectf(
//             &mut self.conns[0],
//             RESULT_DELETED,
//             format!("{} {}\r\n", VERB_DELETE, key).as_bytes(),
//         )
//     }
//
//     // NOTE: Doesn't support optional `expiration` in seconds parameter;
//     pub fn flush_all(&mut self) -> Result<(), OperationError> {
//         Self::write_expectf(
//             &mut self.conns[0],
//             RESULT_OK,
//             format!("{}\r\n", VERB_FLUSH_ALL).as_bytes(),
//         )
//     }
//
//     pub fn delete_all(&mut self) -> Result<(), OperationError> {
//         Self::write_expectf(
//             &mut self.conns[0],
//             RESULT_OK,
//             format!("{}\r\n", VERB_FLUSH_ALL).as_bytes(),
//         )
//     }
//
//     // TODO
//     pub fn touch(&mut self, key: String, seconds: u32) -> Result<(), OperationError> {
//         todo!()
//     }
//
//     // TODO: returns?
//     // NOTE: Populate one what?
//     // NOTE: Why does this not use `write_read_line`?
//     fn populate_one(conn: &mut Conn, verb: &str, item: Item) -> Result<(), OperationError> {
//         if !legal_key(&item.key) {
//             return Err(OperationError::MalformedKey);
//         }
//         // NOTE: Include all in one write?
//         conn.writer
//             .write_fmt(format_args!(
//                 "{} {} {} {} {}\r\n",
//                 verb,
//                 item.key,
//                 item.flags,
//                 item.expiration,
//                 item.value.len(),
//             ))
//             .map_err(|error| OperationError::Io(WriteReadLineError::Write(error)))?;
//         conn.writer
//             .write_all(&item.value)
//             .map_err(|error| OperationError::Io(WriteReadLineError::Write(error)))?;
//         conn.writer
//             .write_all(b"\r\n")
//             .map_err(|error| OperationError::Io(WriteReadLineError::Write(error)))?;
//         conn.writer
//             .flush()
//             .map_err(|error| OperationError::Io(WriteReadLineError::Flush(error)))?;
//         let mut read_buf: Vec<u8> = Vec::new();
//         conn.reader
//             .read_until(b'\n', &mut read_buf)
//             .map_err(|error| OperationError::Io(WriteReadLineError::Read(error)))?;
//
//         match read_buf.as_slice() {
//             RESULT_STORED => Ok(()),
//             RESULT_NOT_STORED => Err(OperationError::NotStored),
//             RESULT_EXISTS => Err(OperationError::CASConflict),
//             RESULT_NOT_FOUND => Err(OperationError::CacheMiss),
//             _ => Err(OperationError::CorruptResponse(format!(
//                 "unexpected response from server: {}",
//                 String::from_utf8(read_buf).unwrap_or_default(), // TODO: Unwrap
//             ))),
//         }
//     }
//
//     fn incr_decr(
//         conn: &mut Conn,
//         verb: &str,
//         key: String,
//         delta: u64,
//     ) -> Result<u64, OperationError> {
//         let line = conn
//             .write_read_line(format!("{} {} {}\r\n", verb, key, delta).as_bytes())
//             .map_err(OperationError::Io)?;
//         if line.as_slice() == RESULT_NOT_FOUND {
//             return Err(OperationError::CacheMiss);
//         }
//         if line.starts_with(RESULT_CLIENT_ERROR_PREFIX) {
//             let error_msg =
//                 String::from_utf8(line[RESULT_CLIENT_ERROR_PREFIX.len()..&line.len() - 2].to_vec())
//                     .unwrap_or_default(); // TODO: FIX
//             return Err(OperationError::Client(error_msg));
//         }
//         String::from_utf8(line[..line.len() - 2].to_vec())
//             .map_err(|_| OperationError::CorruptResponse("invalid UTF-8 sequence".to_string()))?
//             .parse::<u64>()
//             .map_err(|_| OperationError::CorruptResponse("failed to parse integer".to_string()))
//     }
//
//     // NOTE: `expect` String?
//     // NOTE: Different arguments from Go's implementation;
//     fn write_expectf(
//         conn: &mut Conn,
//         expect: &[u8],
//         write_buf: &[u8],
//     ) -> Result<(), OperationError> {
//         let line = conn
//             .write_read_line(write_buf) // TODO: ?
//             .map_err(OperationError::Io)?;
//
//         match line.as_slice() {
//             _ if line.as_slice() == expect => Ok(()),
//             RESULT_OK => Ok(()),
//             RESULT_NOT_STORED => Err(OperationError::NotStored),
//             RESULT_EXISTS => Err(OperationError::CASConflict),
//             RESULT_NOT_FOUND => Err(OperationError::CacheMiss),
//             _ => Err(OperationError::CorruptResponse(format!(
//                 "unexpected response line: {}", // TODO: Include command here `from {}`
//                 String::from_utf8(line).unwrap_or_default()  // TODO: Unwrap
//             ))),
//         }
//     }
//
//     fn net_timout(input_value: u32) -> u32 {
//         match input_value {
//             0 => DEFAULT_NET_TIMEOUT,
//             _ => input_value,
//         }
//     }
//
//     fn max_idle_conns(input_value: u8) -> u8 {
//         match input_value {
//             0 => DEFAULT_MAX_IDLE_CONNS,
//             _ => input_value,
//         }
//     }
// }

#[derive(Debug)]
struct Conn<'a> {
    stream: TcpStream,
    reader: io::BufReader<TcpStream>,
    writer: io::BufWriter<TcpStream>,
    client: &'a Client<'a>,
}

impl<'a> Conn<'a> {
    fn new(stream: TcpStream, client: &'a Client) -> Result<Self, std::io::Error> {
        Ok(Self {
            stream: stream.try_clone()?,
            reader: io::BufReader::new(stream.try_clone()?),
            writer: io::BufWriter::new(stream),
            client,
        })
    }

    fn write_read_line(&mut self, write_buf: &[u8]) -> Result<Vec<u8>, WriteReadLineError> {
        self.writer
            .write_all(write_buf)
            .map_err(WriteReadLineError::Write)?;
        self.writer.flush().map_err(WriteReadLineError::Flush)?;
        let mut read_buf: Vec<u8> = Vec::new();
        self.reader
            .read_until(b'\n', &mut read_buf)
            .map_err(WriteReadLineError::Read)?;
        Ok(read_buf)
    }
}

fn legal_key(key: &String) -> bool {
    if key.len() > 250 {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use crate::selector::ServerList;
    use crate::{errors::ConnError, item::Item};

    use super::Client;
    const LOCALHOST_TCP_ADDR: &str = "127.0.0.1:11211";

    // #[test]
    // fn invalid_server_addr_returns_err() {
    //     // TODO: Fix `ServerList`
    //     let result = Client::new(String::from("alksdjasld"), ServerList {}, 0, 0);
    //     match result {
    //         Ok(_) => panic!("expected creation of new client to fail"),
    //         Err(error) => match error {
    //             ConnError::AddrParseError(_) => (), // Expected error,
    //             _ => panic!("unexpected error. Got: {:?}", error),
    //         },
    //     };
    // }

    #[test]
    fn test_local_host() {
        // TODO: Fix `ServerList`
        let mut client = match Client::new(vec![LOCALHOST_TCP_ADDR.to_string()]) {
            Ok(client) => client,
            Err(error) => panic!("error creating client: {}", error),
        };

        if let Err(error) = client.ping() {
            panic!("expected ping to succeed: {}", error)
        }

        // NOTE: Setting `expiration` to 5 seconds so tests don't fail on subsequent runs;
        // let item_key = "color".to_string();
        // let item_value = Vec::from("red");
        // let item_flags = 32;
        // let item = Item::new(item_key.clone(), item_value.clone(), item_flags, 5);
        // if let Err(_) = client.add(item) {
        //     panic!("expected item to be successfully persisted")
        // }
        //
        // // NOTE: Clone?
        // let item = match client.get(item_key.clone()) {
        //     Ok(item) => item,
        //     Err(error) => panic!("expected item to be successfully retrieved: {}", error),
        // };
        //
        // if let Some(item) = item {
        //     if item.value != item_value {
        //         panic!("expected value to be red")
        //     }
        //     if item.flags != item_flags {
        //         panic!("expected flags to be 0")
        //     }
        // } else {
        //     panic!("expected an item")
        // }
        //
        // // Test `increment` and `decrement`
        // let item_key = "number".to_string();
        // let num = 26;
        // let delta = 10;
        // let num_item = Item::new(item_key.clone(), Vec::from(num.to_string()), 0, 15);
        // if let Err(error) = client.set(num_item) {
        //     panic!("did not expect set to fail: {}", error)
        // }
        //
        // match client.increment(item_key.clone(), delta) {
        //     Ok(incr_num) => {
        //         if incr_num != num + delta {
        //             panic!("expected incremented number ({}) to match with the initial number plus delta ({})", incr_num, num + delta)
        //         }
        //     }
        //     Err(error) => {
        //         panic!("did not expected increment to fail: {}", error)
        //     }
        // }
        //
        // match client.decrement(item_key.clone(), delta) {
        //     Ok(incr_num) => {
        //         if incr_num != num {
        //             panic!(
        //                 "expected decremented number ({}) to match with the initial number ({})",
        //                 incr_num, num
        //             )
        //         }
        //     }
        //     Err(error) => {
        //         panic!("did not expected increment to fail: {}", error)
        //     }
        // }
        //
        // // Test `delete`
        // if let Err(error) = client.delete(item_key) {
        //     panic!("Did not expect delete to fail: {}", error)
        // }
        // // Test `flush_all`
        // if let Err(error) = client.flush_all() {
        //     panic!("Did not expect flush all to fail: {}", error)
        // }
    }
}
