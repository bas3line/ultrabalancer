use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct ConnectionPool {
    max_connections: usize,
    semaphores: Arc<DashMap<String, Arc<Semaphore>>>,
}

impl ConnectionPool {
    pub fn new(max_connections_per_backend: usize) -> Self {
        Self {
            max_connections: max_connections_per_backend,
            semaphores: Arc::new(DashMap::new()),
        }
    }

    pub fn get_semaphore(&self, backend_id: &str) -> Arc<Semaphore> {
        self.semaphores
            .entry(backend_id.to_string())
            .or_insert_with(|| Arc::new(Semaphore::new(self.max_connections)))
            .clone()
    }

    pub fn get_permit(&self, backend_id: &str) -> Arc<Semaphore> {
        self.get_semaphore(backend_id)
    }

    pub fn available_connections(&self, backend_id: &str) -> usize {
        self.semaphores
            .get(backend_id)
            .map(|sem| sem.available_permits())
            .unwrap_or(self.max_connections)
    }

    pub fn clear_backend(&self, backend_id: &str) {
        self.semaphores.remove(backend_id);
    }
}

impl Clone for ConnectionPool {
    fn clone(&self) -> Self {
        Self {
            max_connections: self.max_connections,
            semaphores: Arc::clone(&self.semaphores),
        }
    }
}
