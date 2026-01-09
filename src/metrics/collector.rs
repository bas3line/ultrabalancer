use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
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
}

pub struct MetricsCollector {
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    response_times: Arc<RwLock<Vec<Duration>>>,
    start_time: Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            response_times: Arc::new(RwLock::new(Vec::with_capacity(10000))),
            start_time: Instant::now(),
        }
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

    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
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
        }
    }

    pub fn reset(&self) {
        self.total_requests.store(0, Ordering::Relaxed);
        self.successful_requests.store(0, Ordering::Relaxed);
        self.failed_requests.store(0, Ordering::Relaxed);
        self.response_times.write().clear();
    }
}
