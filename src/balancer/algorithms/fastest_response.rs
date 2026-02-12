use crate::backend::server::Server;
use crate::error::Result;

/// Fastest response time selector that routes to the server with the lowest latency.
pub struct FastestResponseSelector;

impl FastestResponseSelector {
    pub fn new() -> Self {
        Self
    }

    /// Selects the server with the fastest recent response time.
    /// Falls back to least connections if no response times are available.
    pub fn select(&self, servers: &[Server]) -> Result<Server> {
        if servers.is_empty() {
            return Err(crate::error::LoadBalancerError::NoHealthyBackends);
        }

        // Find server with minimum response time
        let fastest = servers.iter().filter(|s| s.is_healthy()).min_by(|a, b| {
            // Compare response times, with fallback to connection count
            match (a.last_response_time(), b.last_response_time()) {
                (Some(time_a), Some(time_b)) => time_a.cmp(&time_b),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => {
                    // Both have no response time, fall back to least connections
                    a.connection_count().cmp(&b.connection_count())
                }
            }
        });

        match fastest {
            Some(server) => Ok(server.clone()),
            None => {
                // No healthy servers, return error
                Err(crate::error::LoadBalancerError::NoHealthyBackends)
            }
        }
    }
}

impl Clone for FastestResponseSelector {
    fn clone(&self) -> Self {
        Self
    }
}

impl Default for FastestResponseSelector {
    fn default() -> Self {
        Self::new()
    }
}
