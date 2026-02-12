use crate::backend::server::Server;
use crate::error::Result;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Lock-free round-robin selector using atomic operations.
/// Better performance under high contention than mutex-based approach.
pub struct RoundRobinSelector {
    current_index: Arc<AtomicUsize>,
}

impl RoundRobinSelector {
    pub fn new() -> Self {
        Self {
            current_index: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn select(&self, servers: &[Server]) -> Result<Server> {
        if servers.is_empty() {
            return Err(crate::error::LoadBalancerError::NoHealthyBackends);
        }

        // Atomic fetch_add is lock-free and scales well under contention.
        // Wrapping on overflow is intentional - modulo handles it correctly.
        let index = self.current_index.fetch_add(1, Ordering::Relaxed);
        let server = &servers[index % servers.len()];

        Ok(server.clone())
    }
}

impl Clone for RoundRobinSelector {
    fn clone(&self) -> Self {
        Self {
            current_index: Arc::clone(&self.current_index),
        }
    }
}

impl Default for RoundRobinSelector {
    fn default() -> Self {
        Self::new()
    }
}
