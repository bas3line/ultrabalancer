use crate::backend::server::Server;
use crate::error::Result;

pub struct LeastConnectionsSelector;

impl LeastConnectionsSelector {
    pub fn new() -> Self {
        Self
    }

    pub fn select(&self, servers: &[Server]) -> Result<Server> {
        servers
            .iter()
            .min_by_key(|s| s.connection_count())
            .cloned()
            .ok_or(crate::error::LoadBalancerError::NoHealthyBackends)
    }
}

impl Clone for LeastConnectionsSelector {
    fn clone(&self) -> Self {
        Self
    }
}
