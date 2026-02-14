use crate::backend::server::Server;
use crate::error::Result;

pub struct LeastConnectionsSelector;

impl LeastConnectionsSelector {
    pub fn new() -> Self {
        Self
    }

    /// Selects the server with the fewest active connections.
    ///
    /// Note: There's an inherent race condition between reading connection counts
    /// and the caller incrementing connections. This is acceptable for load balancing
    /// as perfect accuracy isn't required - the algorithm provides good distribution
    /// over time. For strict ordering, use weighted round-robin instead.
    pub fn select(&self, servers: &[Server]) -> Result<Server> {
        if servers.is_empty() {
            return Err(crate::error::LoadBalancerError::NoHealthyBackends);
        }

        // Collect counts once to minimize race window
        // Use weight as tiebreaker for servers with equal connections
        servers
            .iter()
            .min_by(|a, b| {
                let conn_a = a.connection_count();
                let conn_b = b.connection_count();
                conn_a
                    .cmp(&conn_b)
                    .then_with(|| b.weight().cmp(&a.weight())) // Higher weight wins ties
            })
            .cloned()
            .ok_or(crate::error::LoadBalancerError::NoHealthyBackends)
    }
}

impl Clone for LeastConnectionsSelector {
    fn clone(&self) -> Self {
        Self
    }
}

impl Default for LeastConnectionsSelector {
    fn default() -> Self {
        Self::new()
    }
}
