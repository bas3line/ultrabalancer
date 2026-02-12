use crate::backend::server::Server;
use crate::error::Result;
use fastrand;

/// Power of Two Choices algorithm - selects two random servers and picks the better one.
/// Provides better load distribution than pure random selection with minimal overhead.
pub struct PowerOfTwoSelector;

impl PowerOfTwoSelector {
    pub fn new() -> Self {
        Self
    }

    /// Selects two random servers and chooses the one with fewer connections.
    /// This provides better load distribution than pure random with minimal overhead.
    pub fn select(&self, servers: &[Server]) -> Result<Server> {
        if servers.is_empty() {
            return Err(crate::error::LoadBalancerError::NoHealthyBackends);
        }

        if servers.len() == 1 {
            return Ok(servers[0].clone());
        }

        // Select two different random servers
        let idx1 = fastrand::usize(..servers.len());
        let mut idx2 = fastrand::usize(..servers.len());

        // Ensure we have two different indices
        while idx2 == idx1 && servers.len() > 1 {
            idx2 = fastrand::usize(..servers.len());
        }

        let server1 = &servers[idx1];
        let server2 = &servers[idx2];

        // Choose the server with fewer connections
        if server1.connection_count() <= server2.connection_count() {
            Ok(server1.clone())
        } else {
            Ok(server2.clone())
        }
    }
}

impl Clone for PowerOfTwoSelector {
    fn clone(&self) -> Self {
        Self
    }
}

impl Default for PowerOfTwoSelector {
    fn default() -> Self {
        Self::new()
    }
}
