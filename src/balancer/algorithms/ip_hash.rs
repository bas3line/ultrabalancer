use crate::backend::server::Server;
use crate::error::Result;
use xxhash_rust::xxh3::xxh3_64;

/// IP hash selector for sticky sessions based on client IP.
/// Uses xxHash for fast, consistent hashing.
pub struct IpHashSelector;

impl IpHashSelector {
    pub fn new() -> Self {
        Self
    }

    pub fn select(&self, servers: &[Server], client_ip: &str) -> Result<Server> {
        if servers.is_empty() {
            return Err(crate::error::LoadBalancerError::NoHealthyBackends);
        }

        // Use xxHash for faster hashing than DefaultHasher
        let hash = xxh3_64(client_ip.as_bytes());
        let index = (hash as usize) % servers.len();
        
        Ok(servers[index].clone())
    }
}

impl Clone for IpHashSelector {
    fn clone(&self) -> Self {
        Self
    }
}

impl Default for IpHashSelector {
    fn default() -> Self {
        Self::new()
    }
}
