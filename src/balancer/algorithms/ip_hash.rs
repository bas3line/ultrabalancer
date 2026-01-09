use crate::backend::server::Server;
use crate::error::Result;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub struct IpHashSelector;

impl IpHashSelector {
    pub fn new() -> Self {
        Self
    }

    pub fn select(&self, servers: &[Server], client_ip: &str) -> Result<Server> {
        if servers.is_empty() {
            return Err(crate::error::LoadBalancerError::NoHealthyBackends);
        }

        let mut hasher = DefaultHasher::new();
        client_ip.hash(&mut hasher);
        let hash = hasher.finish();

        let index = (hash as usize) % servers.len();
        Ok(servers[index].clone())
    }
}

impl Clone for IpHashSelector {
    fn clone(&self) -> Self {
        Self
    }
}
