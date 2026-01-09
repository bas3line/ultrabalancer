use crate::backend::server::Server;
use crate::error::Result;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct RoundRobinSelector {
    current_index: Arc<Mutex<usize>>,
}

impl RoundRobinSelector {
    pub fn new() -> Self {
        Self {
            current_index: Arc::new(Mutex::new(0)),
        }
    }

    pub fn select(&self, servers: &[Server]) -> Result<Server> {
        if servers.is_empty() {
            return Err(crate::error::LoadBalancerError::NoHealthyBackends);
        }

        let mut index = self.current_index.lock();
        let server = servers[*index % servers.len()].clone();
        *index = index.wrapping_add(1);

        Ok(server)
    }
}

impl Clone for RoundRobinSelector {
    fn clone(&self) -> Self {
        Self {
            current_index: Arc::clone(&self.current_index),
        }
    }
}
