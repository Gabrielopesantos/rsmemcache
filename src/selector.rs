#![allow(dead_code)]
use std::net::SocketAddr;

// TODO: SUPPORT CONCURRENCY;

// Server selector is the interface that selects a memcache server
// given an item's key
pub trait ServerSelector {
    fn pick_server();
    fn each();
}

// NOTE: Let's not worry about possible concurrency for now
pub struct ServerList {
    addrs: Vec<SocketAddr>,
}

impl ServerList {
    pub fn new() -> Self {
        Self { addrs: Vec::new() } // NOTE: Empty vector?
    }

    pub fn set_servers(&mut self, servers: Vec<String>) -> Self {
        let mut addrs = Vec::with_capacity(servers.len());
        for (index, srv) in servers.iter().enumerate() {
            let socket_addr: Result<SocketAddr, _> = srv.parse();
            match socket_addr {
                Ok(addr) => addrs[index] = addr,
                Err(error) => println!("Could not parse addr {}: {}", srv, error),
            }
        }

        Self { addrs }
    }
}

impl ServerSelector for ServerList {
    fn pick_server() {
        todo!()
    }

    fn each() {
        todo!()
    }
}
