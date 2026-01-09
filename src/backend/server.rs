use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerStatus {
    Up,
    Down,
    Draining,
    Maintenance,
}

impl fmt::Display for ServerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerStatus::Up => write!(f, "UP"),
            ServerStatus::Down => write!(f, "DOWN"),
            ServerStatus::Draining => write!(f, "DRAINING"),
            ServerStatus::Maintenance => write!(f, "MAINTENANCE"),
        }
    }
}

#[derive(Clone)]
pub struct Server {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub weight: u32,
    status: Arc<RwLock<ServerStatus>>,
    fail_count: Arc<AtomicU32>,
    success_count: Arc<AtomicU64>,
    total_requests: Arc<AtomicU64>,
    active_connections: Arc<AtomicU32>,
    last_check: Arc<RwLock<Option<Instant>>>,
    last_response_time: Arc<RwLock<Option<Duration>>>,
}

impl Server {
    pub fn new(host: String, port: u16, weight: u32) -> Self {
        let id = format!("{}:{}", host, port);
        Self {
            id: id.clone(),
            host,
            port,
            weight,
            status: Arc::new(RwLock::new(ServerStatus::Up)),
            fail_count: Arc::new(AtomicU32::new(0)),
            success_count: Arc::new(AtomicU64::new(0)),
            total_requests: Arc::new(AtomicU64::new(0)),
            active_connections: Arc::new(AtomicU32::new(0)),
            last_check: Arc::new(RwLock::new(None)),
            last_response_time: Arc::new(RwLock::new(None)),
        }
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn is_healthy(&self) -> bool {
        matches!(*self.status.read(), ServerStatus::Up)
    }

    pub fn is_available(&self) -> bool {
        matches!(
            *self.status.read(),
            ServerStatus::Up | ServerStatus::Draining
        )
    }

    pub fn status(&self) -> ServerStatus {
        *self.status.read()
    }

    pub fn set_status(&self, status: ServerStatus) {
        *self.status.write() = status;
        *self.last_check.write() = Some(Instant::now());
    }

    pub fn mark_healthy(&self) {
        self.set_status(ServerStatus::Up);
        self.fail_count.store(0, Ordering::Relaxed);
        self.success_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn mark_unhealthy(&self) {
        self.set_status(ServerStatus::Down);
    }

    pub fn increment_failures(&self) -> u32 {
        self.fail_count.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn reset_failures(&self) {
        self.fail_count.store(0, Ordering::Relaxed);
    }

    pub fn failure_count(&self) -> u32 {
        self.fail_count.load(Ordering::Relaxed)
    }

    pub fn increment_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decrement_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn connection_count(&self) -> u32 {
        self.active_connections.load(Ordering::Relaxed)
    }

    pub fn total_requests(&self) -> u64 {
        self.total_requests.load(Ordering::Relaxed)
    }

    pub fn success_count(&self) -> u64 {
        self.success_count.load(Ordering::Relaxed)
    }

    pub fn set_response_time(&self, duration: Duration) {
        *self.last_response_time.write() = Some(duration);
    }

    pub fn last_response_time(&self) -> Option<Duration> {
        *self.last_response_time.read()
    }

    pub fn last_check_time(&self) -> Option<Instant> {
        *self.last_check.read()
    }
}

impl fmt::Debug for Server {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Server")
            .field("id", &self.id)
            .field("address", &self.address())
            .field("weight", &self.weight)
            .field("status", &self.status())
            .field("active_connections", &self.connection_count())
            .finish()
    }
}
