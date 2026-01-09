use super::algorithms::*;
use crate::backend::server::Server;
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    RoundRobin,
    LeastConnections,
    IpHash,
    Random,
    WeightedRoundRobin,
}

impl Algorithm {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "round-robin" | "roundrobin" | "rr" => Some(Algorithm::RoundRobin),
            "least-connections" | "leastconn" | "lc" => Some(Algorithm::LeastConnections),
            "ip-hash" | "iphash" | "hash" => Some(Algorithm::IpHash),
            "random" | "rand" => Some(Algorithm::Random),
            "weighted" | "weighted-round-robin" | "wrr" => Some(Algorithm::WeightedRoundRobin),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Algorithm::RoundRobin => "round-robin",
            Algorithm::LeastConnections => "least-connections",
            Algorithm::IpHash => "ip-hash",
            Algorithm::Random => "random",
            Algorithm::WeightedRoundRobin => "weighted",
        }
    }
}

#[derive(Clone)]
pub enum LoadBalancerSelector {
    RoundRobin(RoundRobinSelector),
    LeastConnections(LeastConnectionsSelector),
    IpHash(IpHashSelector),
    Random(RandomSelector),
    WeightedRoundRobin(WeightedRoundRobinSelector),
}

impl LoadBalancerSelector {
    pub fn new(algorithm: Algorithm) -> Self {
        match algorithm {
            Algorithm::RoundRobin => {
                LoadBalancerSelector::RoundRobin(RoundRobinSelector::new())
            }
            Algorithm::LeastConnections => {
                LoadBalancerSelector::LeastConnections(LeastConnectionsSelector::new())
            }
            Algorithm::IpHash => LoadBalancerSelector::IpHash(IpHashSelector::new()),
            Algorithm::Random => LoadBalancerSelector::Random(RandomSelector::new()),
            Algorithm::WeightedRoundRobin => {
                LoadBalancerSelector::WeightedRoundRobin(WeightedRoundRobinSelector::new())
            }
        }
    }

    pub fn select(&self, servers: &[Server], client_ip: Option<&str>) -> Result<Server> {
        match self {
            LoadBalancerSelector::RoundRobin(selector) => selector.select(servers),
            LoadBalancerSelector::LeastConnections(selector) => selector.select(servers),
            LoadBalancerSelector::IpHash(selector) => {
                let ip = client_ip.unwrap_or("0.0.0.0");
                selector.select(servers, ip)
            }
            LoadBalancerSelector::Random(selector) => selector.select(servers),
            LoadBalancerSelector::WeightedRoundRobin(selector) => selector.select(servers),
        }
    }
}
