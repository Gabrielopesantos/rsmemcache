#![allow(dead_code)]
use std::net::SocketAddr;

use crate::errors::OperationError;

// TODO: SUPPORT CONCURRENCY;

// Server selector is the interface that selects a memcache server
// given an item's key
pub trait ServerSelector {
    fn pick_server(&mut self, key: &str) -> Result<SocketAddr, OperationError>;
    fn each(
        &mut self,
        f: fn(SocketAddr) -> Result<(), OperationError>,
    ) -> Result<(), OperationError>;
}

// NOTE: Let's not worry about possible concurrency for now
#[derive(Debug)]
pub struct ServerList {
    pub addrs: Vec<SocketAddr>, // NOTE pub
    key_buffer_pool: [u8; 256],
}

impl ServerList {
    pub fn new() -> Self {
        Self {
            addrs: Vec::new(),
            key_buffer_pool: [0; 256],
        }
    }

    pub fn set_servers(&mut self, servers: Vec<String>) -> Result<(), OperationError> {
        // let mut addrs = Vec::with_capacity(servers.len());
        for (_, srv) in servers.iter().enumerate() {
            let socket_addr: Result<SocketAddr, _> = srv.parse();
            match socket_addr {
                // NOTE: Do we need to record server indexes?
                // Ok(addr) => addrs[index] = addr,
                Ok(addr) => self.addrs.push(addr),
                // TODO: Return error instead
                Err(error) => {
                    return Err(OperationError::Client(format!(
                        "invalid server address provided: {}",
                        error
                    )))
                }
            }
        }
        Ok(())
    }
}

impl ServerSelector for ServerList {
    fn pick_server(&mut self, key: &str) -> Result<SocketAddr, OperationError> {
        match self.addrs.len() {
            0 => Err(OperationError::Client(
                "no servers configured or available".to_string(),
            )),
            1 => Ok(self.addrs[0]),
            _ => {
                self.key_buffer_pool[..key.len()].copy_from_slice(key.as_bytes());
                let checksum = crc32fast::hash(self.key_buffer_pool[..key.len()].as_ref());
                Ok(self.addrs[(checksum % self.addrs.len() as u32) as usize])
            }
        }
    }

    fn each(
        &mut self,
        f: fn(SocketAddr) -> Result<(), OperationError>,
    ) -> Result<(), OperationError> {
        for addr in self.addrs.iter() {
            f(*addr)?;
        }
        Ok(())
    }
}
