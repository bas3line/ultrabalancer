use super::server::{Server, ServerStatus};
use crate::error::{LoadBalancerError, Result};
use arc_swap::ArcSwap;
use dashmap::DashMap;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct ServerPool {
    servers: Arc<ArcSwap<Vec<Server>>>,
    server_map: Arc<DashMap<String, Server>>,
    /// Lock for mutating operations to prevent TOCTOU races
    mutation_lock: Arc<Mutex<()>>,
}

impl ServerPool {
    pub fn new(servers: Vec<Server>) -> Self {
        let server_map = DashMap::new();
        for server in &servers {
            server_map.insert(server.id.clone(), server.clone());
        }

        Self {
            servers: Arc::new(ArcSwap::from_pointee(servers)),
            server_map: Arc::new(server_map),
            mutation_lock: Arc::new(Mutex::new(())),
        }
    }

    pub fn get_healthy_servers(&self) -> Vec<Server> {
        self.servers
            .load()
            .iter()
            .filter(|s| s.is_healthy())
            .cloned()
            .collect()
    }

    pub fn get_available_servers(&self) -> Vec<Server> {
        self.servers
            .load()
            .iter()
            .filter(|s| s.is_available())
            .cloned()
            .collect()
    }

    pub fn get_all_servers(&self) -> Vec<Server> {
        self.servers.load().as_ref().clone()
    }

    pub fn get_server(&self, id: &str) -> Option<Server> {
        self.server_map.get(id).map(|entry| entry.clone())
    }

    pub fn update_server_status(&self, id: &str, status: ServerStatus) -> Result<()> {
        let server = self
            .server_map
            .get(id)
            .ok_or_else(|| LoadBalancerError::InvalidBackendAddress(id.to_string()))?;

        server.set_status(status);
        Ok(())
    }

    pub fn mark_server_down(&self, id: &str) {
        if let Some(server) = self.server_map.get(id) {
            server.mark_unhealthy();
        }
    }

    pub fn mark_server_up(&self, id: &str) {
        if let Some(server) = self.server_map.get(id) {
            server.mark_healthy();
        }
    }

    /// Add a server to the pool. Thread-safe.
    /// Returns true if server was added, false if it already exists.
    pub fn add_server(&self, server: Server) -> bool {
        let _guard = self.mutation_lock.lock();
        
        let id = server.id.clone();
        
        // Idempotent: skip if already exists
        if self.server_map.contains_key(&id) {
            return false;
        }
        
        self.server_map.insert(id, server.clone());

        let mut servers = self.servers.load().as_ref().clone();
        servers.push(server);
        self.servers.store(Arc::new(servers));
        true
    }

    /// Remove a server from the pool. Thread-safe.
    pub fn remove_server(&self, id: &str) -> Result<()> {
        let _guard = self.mutation_lock.lock();
        
        self.server_map
            .remove(id)
            .ok_or_else(|| LoadBalancerError::InvalidBackendAddress(id.to_string()))?;

        let servers: Vec<Server> = self
            .servers
            .load()
            .iter()
            .filter(|s| s.id != id)
            .cloned()
            .collect();

        self.servers.store(Arc::new(servers));
        Ok(())
    }

    pub fn server_count(&self) -> usize {
        self.servers.load().len()
    }

    pub fn healthy_count(&self) -> usize {
        self.get_healthy_servers().len()
    }

    pub fn total_connections(&self) -> u32 {
        self.servers
            .load()
            .iter()
            .map(|s| s.connection_count())
            .sum()
    }
}

impl Clone for ServerPool {
    fn clone(&self) -> Self {
        Self {
            servers: Arc::clone(&self.servers),
            server_map: Arc::clone(&self.server_map),
            mutation_lock: Arc::clone(&self.mutation_lock),
        }
    }
}
