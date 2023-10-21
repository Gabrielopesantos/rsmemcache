#![allow(dead_code)]
use std::net::SocketAddr;
// use std::sync::RwLock;

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
    fn new() -> Self {
        Self { addrs: Vec::new() }
    }

    fn set_servers(&mut self, servers: Vec<String>) -> Self {
        todo!()
    }
}

impl ServerSelector for ServerList {
    fn pick_server() {}
    fn each() {}
}
