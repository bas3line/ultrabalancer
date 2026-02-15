use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_response_time_ms: f64,
    pub min_response_time_ms: f64,
    pub max_response_time_ms: f64,
    pub p50_response_time_ms: f64,
    pub p95_response_time_ms: f64,
    pub p99_response_time_ms: f64,
    pub uptime_seconds: u64,
    pub requests_per_second: f64,
    pub backend_metrics: HashMap<String, BackendMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_response_time_ms: f64,
    pub active_connections: u64,
    pub last_response_time_ms: f64,
    pub status: String,
}

struct BackendMetricsData {
    total_requests: Arc<AtomicU64>,
    successful_requests: Arc<AtomicU64>,
    failed_requests: Arc<AtomicU64>,
    response_times: Arc<RwLock<Vec<Duration>>>,
    active_connections: Arc<AtomicU32>,
    status: Arc<RwLock<String>>,
}

impl BackendMetricsData {
    fn new() -> Self {
        Self {
            total_requests: Arc::new(AtomicU64::new(0)),
            successful_requests: Arc::new(AtomicU64::new(0)),
            failed_requests: Arc::new(AtomicU64::new(0)),
            response_times: Arc::new(RwLock::new(Vec::with_capacity(1000))),
            active_connections: Arc::new(AtomicU32::new(0)),
            status: Arc::new(RwLock::new("up".to_string())),
        }
    }

    fn set_status(&self, status: &str) {
        *self.status.write() = status.to_string();
    }

    fn increment_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    fn decrement_connections(&self) {
        let _ =
            self.active_connections
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                    Some(current.saturating_sub(1))
                });
    }
}

impl Clone for BackendMetricsData {
    fn clone(&self) -> Self {
        Self {
            total_requests: Arc::clone(&self.total_requests),
            successful_requests: Arc::clone(&self.successful_requests),
            failed_requests: Arc::clone(&self.failed_requests),
            response_times: Arc::clone(&self.response_times),
            active_connections: Arc::clone(&self.active_connections),
            status: Arc::clone(&self.status),
        }
    }
}

fn active_connections_to_u64(connections: u32) -> u64 {
    connections as u64
}

pub struct MetricsCollector {
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    response_times: Arc<RwLock<Vec<Duration>>>,
    start_time: Instant,
    backend_metrics: Arc<RwLock<HashMap<String, BackendMetricsData>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            response_times: Arc::new(RwLock::new(Vec::with_capacity(10000))),
            start_time: Instant::now(),
            backend_metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn init_backends(&self, backends: &[String]) {
        let mut backends_lock = self.backend_metrics.write();
        for backend in backends {
            backends_lock
                .entry(backend.clone())
                .or_insert_with(BackendMetricsData::new);
        }
    }

    fn get_or_create_backend_metrics(&self, backend: &str) -> BackendMetricsData {
        let mut backends = self.backend_metrics.write();
        backends
            .entry(backend.to_string())
            .or_insert_with(BackendMetricsData::new)
            .clone()
    }

    pub fn increment_total_requests(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_successful_requests(&self) {
        self.successful_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_failed_requests(&self) {
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_response_time(&self, duration: Duration) {
        let mut times = self.response_times.write();
        times.push(duration);

        if times.len() > 10000 {
            times.drain(0..5000);
        }
    }

    pub fn record_backend_request(&self, backend: &str, success: bool, duration: Duration) {
        let backends = self.backend_metrics.read();
        if let Some(backend_data) = backends.get(backend) {
            backend_data.total_requests.fetch_add(1, Ordering::Relaxed);
            if success {
                backend_data
                    .successful_requests
                    .fetch_add(1, Ordering::Relaxed);
            } else {
                backend_data.failed_requests.fetch_add(1, Ordering::Relaxed);
            }

            let mut times = backend_data.response_times.write();
            times.push(duration);
            if times.len() > 1000 {
                times.drain(0..500);
            }
        } else {
            drop(backends);
            self.get_or_create_backend_metrics(backend);
            self.record_backend_request(backend, success, duration);
        }
    }

    pub fn increment_backend_connections(&self, backend: &str) {
        let backends = self.backend_metrics.read();
        if let Some(backend_data) = backends.get(backend) {
            backend_data.increment_connections();
        } else {
            drop(backends);
            self.get_or_create_backend_metrics(backend);
            self.increment_backend_connections(backend);
        }
    }

    pub fn decrement_backend_connections(&self, backend: &str) {
        let backends = self.backend_metrics.read();
        if let Some(backend_data) = backends.get(backend) {
            backend_data.decrement_connections();
        }
    }

    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn update_backend_status(&self, backend: &str, healthy: bool) {
        let backends = self.backend_metrics.read();
        if let Some(backend_data) = backends.get(backend) {
            backend_data.set_status(if healthy { "up" } else { "down" });
        }
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        let times = self.response_times.read();
        let total = self.total_requests.load(Ordering::Relaxed);
        let uptime = self.uptime();

        let (avg, min, max, p50, p95, p99) = if times.is_empty() {
            (0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
        } else {
            let mut sorted_times: Vec<Duration> = times.clone();
            sorted_times.sort();

            let sum: Duration = sorted_times.iter().sum();
            let avg = sum.as_secs_f64() / sorted_times.len() as f64 * 1000.0;
            let min = sorted_times.first().unwrap().as_secs_f64() * 1000.0;
            let max = sorted_times.last().unwrap().as_secs_f64() * 1000.0;

            let p50_idx = sorted_times.len() * 50 / 100;
            let p95_idx = sorted_times.len() * 95 / 100;
            let p99_idx = sorted_times.len() * 99 / 100;

            let p50 = sorted_times.get(p50_idx).unwrap().as_secs_f64() * 1000.0;
            let p95 = sorted_times.get(p95_idx).unwrap().as_secs_f64() * 1000.0;
            let p99 = sorted_times.get(p99_idx).unwrap().as_secs_f64() * 1000.0;

            (avg, min, max, p50, p95, p99)
        };

        let rps = if uptime.as_secs() > 0 {
            total as f64 / uptime.as_secs() as f64
        } else {
            0.0
        };

        let backend_metrics = {
            let backends = self.backend_metrics.read();
            backends
                .iter()
                .map(|(name, data)| {
                    let times = data.response_times.read();
                    let avg = if times.is_empty() {
                        0.0
                    } else {
                        let sum: Duration = times.iter().sum();
                        sum.as_secs_f64() / times.len() as f64 * 1000.0
                    };
                    let last_rt = times
                        .last()
                        .map(|d| d.as_secs_f64() * 1000.0)
                        .unwrap_or(0.0);
                    let active_conn = data.active_connections.load(Ordering::Relaxed);

                    (
                        name.clone(),
                        BackendMetrics {
                            total_requests: data.total_requests.load(Ordering::Relaxed),
                            successful_requests: data.successful_requests.load(Ordering::Relaxed),
                            failed_requests: data.failed_requests.load(Ordering::Relaxed),
                            avg_response_time_ms: avg,
                            active_connections: active_conn as u64,
                            last_response_time_ms: last_rt,
                            status: data.status.read().clone(),
                        },
                    )
                })
                .collect()
        };

        MetricsSnapshot {
            total_requests: total,
            successful_requests: self.successful_requests.load(Ordering::Relaxed),
            failed_requests: self.failed_requests.load(Ordering::Relaxed),
            avg_response_time_ms: avg,
            min_response_time_ms: min,
            max_response_time_ms: max,
            p50_response_time_ms: p50,
            p95_response_time_ms: p95,
            p99_response_time_ms: p99,
            uptime_seconds: uptime.as_secs(),
            requests_per_second: rps,
            backend_metrics,
        }
    }

    pub fn reset(&self) {
        self.total_requests.store(0, Ordering::Relaxed);
        self.successful_requests.store(0, Ordering::Relaxed);
        self.failed_requests.store(0, Ordering::Relaxed);
        self.response_times.write().clear();
    }
}
