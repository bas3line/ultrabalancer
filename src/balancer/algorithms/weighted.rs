use crate::backend::server::Server;
use crate::error::Result;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct WeightedRoundRobinSelector {
    current_index: Arc<Mutex<usize>>,
    current_weight: Arc<Mutex<i32>>,
}

impl WeightedRoundRobinSelector {
    pub fn new() -> Self {
        Self {
            current_index: Arc::new(Mutex::new(0)),
            current_weight: Arc::new(Mutex::new(0)),
        }
    }

    pub fn select(&self, servers: &[Server]) -> Result<Server> {
        if servers.is_empty() {
            return Err(crate::error::LoadBalancerError::NoHealthyBackends);
        }

        let total_weight: i32 = servers.iter().map(|s| s.weight as i32).sum();
        if total_weight == 0 {
            return Ok(servers[0].clone());
        }

        let max_weight = servers.iter().map(|s| s.weight).max().unwrap_or(1) as i32;

        let mut index = self.current_index.lock();
        let mut current_weight = self.current_weight.lock();

        loop {
            *index = (*index + 1) % servers.len();

            if *index == 0 {
                *current_weight -= 1;
                if *current_weight <= 0 {
                    *current_weight = max_weight;
                }
            }

            if servers[*index].weight as i32 >= *current_weight {
                return Ok(servers[*index].clone());
            }
        }
    }
}

impl Clone for WeightedRoundRobinSelector {
    fn clone(&self) -> Self {
        Self {
            current_index: Arc::clone(&self.current_index),
            current_weight: Arc::clone(&self.current_weight),
        }
    }
}
