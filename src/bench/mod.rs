use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tracing::info;

pub struct BenchmarkConfig {
    pub target_url: String,
    pub total_requests: u64,
    pub concurrency: usize,
    pub duration_secs: Option<u64>,
    pub keep_alive: bool,
    pub timeout_ms: u64,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            target_url: "http://127.0.0.1:8080".to_string(),
            total_requests: 100_000,
            concurrency: 500,
            duration_secs: None,
            keep_alive: true,
            timeout_ms: 5000,
        }
    }
}

pub struct BenchmarkResults {
    pub total_requests: u64,
    pub successful: u64,
    pub failed: u64,
    pub duration: Duration,
    pub requests_per_second: f64,
    pub avg_latency_ms: f64,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    pub p50_latency_ms: f64,
    pub p90_latency_ms: f64,
    pub p99_latency_ms: f64,
}

impl std::fmt::Display for BenchmarkResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Benchmark Results")?;
        writeln!(f, "=================")?;
        writeln!(f, "Total Requests:     {}", self.total_requests)?;
        writeln!(f, "Successful:         {}", self.successful)?;
        writeln!(f, "Failed:             {}", self.failed)?;
        writeln!(f, "Duration:           {:.2}s", self.duration.as_secs_f64())?;
        writeln!(f, "Requests/sec:       {:.2}", self.requests_per_second)?;
        writeln!(f)?;
        writeln!(f, "Latency Statistics:")?;
        writeln!(f, "  Avg:              {:.2}ms", self.avg_latency_ms)?;
        writeln!(f, "  Min:              {:.2}ms", self.min_latency_ms)?;
        writeln!(f, "  Max:              {:.2}ms", self.max_latency_ms)?;
        writeln!(f, "  p50:              {:.2}ms", self.p50_latency_ms)?;
        writeln!(f, "  p90:              {:.2}ms", self.p90_latency_ms)?;
        writeln!(f, "  p99:              {:.2}ms", self.p99_latency_ms)?;
        Ok(())
    }
}

struct BenchmarkState {
    completed: AtomicU64,
    successful: AtomicU64,
    failed: AtomicU64,
}

pub async fn run_benchmark(config: BenchmarkConfig) -> anyhow::Result<BenchmarkResults> {
    info!(
        "Starting benchmark: {} requests with {} concurrent connections",
        config.total_requests, config.concurrency
    );
    info!("Target: {}", config.target_url);

    let state = Arc::new(BenchmarkState {
        completed: AtomicU64::new(0),
        successful: AtomicU64::new(0),
        failed: AtomicU64::new(0),
    });

    let latencies = Arc::new(parking_lot::Mutex::new(Vec::with_capacity(
        config.total_requests as usize,
    )));
    let semaphore = Arc::new(Semaphore::new(config.concurrency));

    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(config.concurrency)
        .timeout(Duration::from_millis(config.timeout_ms))
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(if config.keep_alive {
            Some(Duration::from_secs(60))
        } else {
            None
        })
        .build()?;

    let start = Instant::now();
    let mut handles = Vec::new();

    for _ in 0..config.total_requests {
        let permit = semaphore.clone().acquire_owned().await?;
        let client = client.clone();
        let url = config.target_url.clone();
        let state = Arc::clone(&state);
        let latencies = Arc::clone(&latencies);

        let handle = tokio::spawn(async move {
            let req_start = Instant::now();
            let result = client.get(&url).send().await;
            let latency = req_start.elapsed();

            state.completed.fetch_add(1, Ordering::Relaxed);

            match result {
                Ok(resp) if resp.status().is_success() => {
                    state.successful.fetch_add(1, Ordering::Relaxed);
                    latencies.lock().push(latency.as_secs_f64() * 1000.0);
                }
                Ok(_) => {
                    state.failed.fetch_add(1, Ordering::Relaxed);
                }
                Err(_) => {
                    state.failed.fetch_add(1, Ordering::Relaxed);
                }
            }

            drop(permit);
        });

        handles.push(handle);

        if let Some(duration) = config.duration_secs {
            if start.elapsed() > Duration::from_secs(duration) {
                break;
            }
        }
    }

    for handle in handles {
        let _ = handle.await;
    }

    let duration = start.elapsed();
    let total = state.completed.load(Ordering::Relaxed);
    let successful = state.successful.load(Ordering::Relaxed);
    let failed = state.failed.load(Ordering::Relaxed);

    let mut latencies_vec = latencies.lock().clone();
    latencies_vec.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let (avg, min, max, p50, p90, p99) = if !latencies_vec.is_empty() {
        let sum: f64 = latencies_vec.iter().sum();
        let len = latencies_vec.len();
        (
            sum / len as f64,
            latencies_vec[0],
            latencies_vec[len - 1],
            latencies_vec[len / 2],
            latencies_vec[(len as f64 * 0.90) as usize],
            latencies_vec[(len as f64 * 0.99) as usize],
        )
    } else {
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
    };

    let rps = total as f64 / duration.as_secs_f64();

    Ok(BenchmarkResults {
        total_requests: total,
        successful,
        failed,
        duration,
        requests_per_second: rps,
        avg_latency_ms: avg,
        min_latency_ms: min,
        max_latency_ms: max,
        p50_latency_ms: p50,
        p90_latency_ms: p90,
        p99_latency_ms: p99,
    })
}

pub async fn stress_test(
    target: &str,
    target_rps: u64,
    duration_secs: u64,
) -> anyhow::Result<BenchmarkResults> {
    info!(
        "Starting stress test: target {}+ RPS for {}s",
        target_rps, duration_secs
    );

    let concurrency = (target_rps / 100).clamp(500, 10000) as usize;

    let config = BenchmarkConfig {
        target_url: target.to_string(),
        total_requests: target_rps * duration_secs,
        concurrency,
        duration_secs: Some(duration_secs),
        keep_alive: true,
        timeout_ms: 2000,
    };

    run_benchmark(config).await
}

pub async fn quick_benchmark(target: &str) -> anyhow::Result<BenchmarkResults> {
    let config = BenchmarkConfig {
        target_url: target.to_string(),
        total_requests: 10_000,
        concurrency: 100,
        duration_secs: None,
        keep_alive: true,
        timeout_ms: 5000,
    };

    run_benchmark(config).await
}
