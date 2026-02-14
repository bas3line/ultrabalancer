use crate::backend::server::Server;
use crate::error::Result;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct WeightedRoundRobinSelector {
    state: Arc<Mutex<WeightedState>>,
}

struct WeightedState {
    current_index: usize,
    current_weight: i32,
}

impl WeightedRoundRobinSelector {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(WeightedState {
                current_index: 0,
                current_weight: 0,
            })),
        }
    }

    pub fn select(&self, servers: &[Server]) -> Result<Server> {
        if servers.is_empty() {
            return Err(crate::error::LoadBalancerError::NoHealthyBackends);
        }

        let total_weight: i32 = servers.iter().map(|s| s.weight() as i32).sum();
        if total_weight == 0 {
            // All weights are 0, fall back to simple round-robin
            let mut state = self.state.lock();
            let idx = state.current_index % servers.len();
            state.current_index = state.current_index.wrapping_add(1);
            return Ok(servers[idx].clone());
        }

        let max_weight = servers.iter().map(|s| s.weight()).max().unwrap_or(1) as i32;
        let gcd = self.gcd_weights(servers);

        let mut state = self.state.lock();

        // Limit iterations to prevent infinite loop (max: servers.len() * max_weight / gcd)
        let max_iterations = servers.len() * (max_weight as usize) / (gcd as usize).max(1);

        for _ in 0..max_iterations {
            state.current_index = (state.current_index + 1) % servers.len();

            if state.current_index == 0 {
                state.current_weight -= gcd;
                if state.current_weight <= 0 {
                    state.current_weight = max_weight;
                }
            }

            if servers[state.current_index].weight() as i32 >= state.current_weight {
                return Ok(servers[state.current_index].clone());
            }
        }

        // Fallback: return first server with non-zero weight, or first server
        Ok(servers
            .iter()
            .find(|s| s.weight() > 0)
            .cloned()
            .unwrap_or_else(|| servers[0].clone()))
    }

    fn gcd_weights(&self, servers: &[Server]) -> i32 {
        // Iterative GCD to avoid stack overflow on large values
        fn gcd(mut a: i32, mut b: i32) -> i32 {
            while b != 0 {
                let t = b;
                b = a % b;
                a = t;
            }
            a.abs()
        }

        servers
            .iter()
            .map(|s| s.weight() as i32)
            .filter(|&w| w > 0)
            .fold(0, gcd)
            .max(1)
    }
}

impl Clone for WeightedRoundRobinSelector {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
        }
    }
}

impl Default for WeightedRoundRobinSelector {
    fn default() -> Self {
        Self::new()
    }
}
