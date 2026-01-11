use crate::backend::server::Server;
use crate::error::Result;

pub struct RandomSelector;

impl RandomSelector {
    pub fn new() -> Self {
        Self
    }

    pub fn select(&self, servers: &[Server]) -> Result<Server> {
        if servers.is_empty() {
            return Err(crate::error::LoadBalancerError::NoHealthyBackends);
        }

        // Use fastrand for proper randomness (thread-local RNG)
        let index = fastrand::usize(..servers.len());
        Ok(servers[index].clone())
    }
}

impl Clone for RandomSelector {
    fn clone(&self) -> Self {
        Self
    }
}

impl Default for RandomSelector {
    fn default() -> Self {
        Self::new()
    }
}
