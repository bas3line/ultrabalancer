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

        use std::time::{SystemTime, UNIX_EPOCH};
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let index = (nanos as usize) % servers.len();
        Ok(servers[index].clone())
    }
}

impl Clone for RandomSelector {
    fn clone(&self) -> Self {
        Self
    }
}
