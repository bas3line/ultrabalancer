use bytes::Bytes;
use dashmap::DashMap;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::{Request, Response};
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

type HttpClient = Client<hyper_util::client::legacy::connect::HttpConnector, Full<Bytes>>;

pub struct PooledConnection {
    client: HttpClient,
    created_at: Instant,
    requests_served: AtomicU64,
}

impl PooledConnection {
    fn new() -> Self {
        let connector = hyper_util::client::legacy::connect::HttpConnector::new();
        let client = Client::builder(TokioExecutor::new())
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(100)
            .http2_only(false)
            .build(connector);

        Self {
            client,
            created_at: Instant::now(),
            requests_served: AtomicU64::new(0),
        }
    }

    pub async fn send(
        &self,
        req: Request<Full<Bytes>>,
    ) -> Result<Response<Incoming>, hyper_util::client::legacy::Error> {
        self.requests_served.fetch_add(1, Ordering::Relaxed);
        self.client.request(req).await
    }

    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    pub fn requests_count(&self) -> u64 {
        self.requests_served.load(Ordering::Relaxed)
    }
}

pub struct ConnectionPool {
    max_per_backend: usize,
    clients: DashMap<String, Arc<PooledConnection>>,
    semaphores: DashMap<String, Arc<Semaphore>>,
    total_connections: AtomicU64,
}

impl ConnectionPool {
    pub fn new(max_connections_per_backend: usize) -> Self {
        Self {
            max_per_backend: max_connections_per_backend,
            clients: DashMap::new(),
            semaphores: DashMap::new(),
            total_connections: AtomicU64::new(0),
        }
    }

    pub fn get_client(&self, backend: &str) -> Arc<PooledConnection> {
        self.clients
            .entry(backend.to_string())
            .or_insert_with(|| {
                self.total_connections.fetch_add(1, Ordering::Relaxed);
                Arc::new(PooledConnection::new())
            })
            .clone()
    }

    /// Returns an owned permit that auto-releases when dropped
    pub fn acquire(&self, backend: &str) -> Option<OwnedSemaphorePermit> {
        let sem = self
            .semaphores
            .entry(backend.to_string())
            .or_insert_with(|| Arc::new(Semaphore::new(self.max_per_backend)))
            .clone();

        sem.try_acquire_owned().ok()
    }

    pub fn active_count(&self, backend: &str) -> usize {
        self.semaphores
            .get(backend)
            .map(|sem| self.max_per_backend - sem.available_permits())
            .unwrap_or(0)
    }

    pub fn available(&self, backend: &str) -> usize {
        self.semaphores
            .get(backend)
            .map(|sem| sem.available_permits())
            .unwrap_or(self.max_per_backend)
    }

    pub fn total_connections(&self) -> u64 {
        self.total_connections.load(Ordering::Relaxed)
    }

    pub fn remove_backend(&self, backend: &str) {
        self.clients.remove(backend);
        self.semaphores.remove(backend);
    }
}

impl Clone for ConnectionPool {
    fn clone(&self) -> Self {
        Self {
            max_per_backend: self.max_per_backend,
            clients: self.clients.clone(),
            semaphores: self.semaphores.clone(),
            total_connections: AtomicU64::new(self.total_connections.load(Ordering::Relaxed)),
        }
    }
}
