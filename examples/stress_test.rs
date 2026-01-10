use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

struct BenchResults {
    total: u64,
    success: u64,
    failed: u64,
    duration: Duration,
    latencies: Vec<f64>,
}

async fn run_stress_test(
    target: &str,
    requests: u64,
    concurrency: usize,
) -> anyhow::Result<BenchResults> {
    let success = Arc::new(AtomicU64::new(0));
    let failed = Arc::new(AtomicU64::new(0));
    let latencies = Arc::new(parking_lot::Mutex::new(Vec::with_capacity(requests as usize)));
    let semaphore = Arc::new(Semaphore::new(concurrency));

    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(concurrency)
        .pool_idle_timeout(Duration::from_secs(90))
        .timeout(Duration::from_secs(5))
        .tcp_keepalive(Duration::from_secs(60))
        .build()?;

    let start = Instant::now();
    let mut handles = Vec::with_capacity(requests as usize);

    for _ in 0..requests {
        let permit = semaphore.clone().acquire_owned().await?;
        let client = client.clone();
        let url = target.to_string();
        let success = Arc::clone(&success);
        let failed = Arc::clone(&failed);
        let latencies = Arc::clone(&latencies);

        let handle = tokio::spawn(async move {
            let req_start = Instant::now();
            let result = client.get(&url).send().await;
            let latency = req_start.elapsed().as_secs_f64() * 1000.0;

            match result {
                Ok(resp) if resp.status().is_success() => {
                    let _ = resp.bytes().await;
                    success.fetch_add(1, Ordering::Relaxed);
                    latencies.lock().push(latency);
                }
                _ => {
                    failed.fetch_add(1, Ordering::Relaxed);
                }
            }

            drop(permit);
        });

        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.await;
    }

    let duration = start.elapsed();

    Ok(BenchResults {
        total: requests,
        success: success.load(Ordering::Relaxed),
        failed: failed.load(Ordering::Relaxed),
        duration,
        latencies: latencies.lock().clone(),
    })
}

fn percentile(data: &[f64], p: f64) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let idx = ((data.len() as f64 * p) as usize).min(data.len() - 1);
    data[idx]
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let target = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://127.0.0.1:8080/health".to_string());

    let requests: u64 = std::env::args()
        .nth(2)
        .and_then(|r| r.parse().ok())
        .unwrap_or(100_000);

    let concurrency: usize = std::env::args()
        .nth(3)
        .and_then(|c| c.parse().ok())
        .unwrap_or(1000);

    println!("===========================================");
    println!("UltraBalancer Stress Test");
    println!("===========================================");
    println!("Target:      {}", target);
    println!("Requests:    {}", requests);
    println!("Concurrency: {}", concurrency);
    println!("-------------------------------------------");

    let results = run_stress_test(&target, requests, concurrency).await?;

    let rps = results.success as f64 / results.duration.as_secs_f64();

    let mut latencies = results.latencies.clone();
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let avg_latency: f64 = if !latencies.is_empty() {
        latencies.iter().sum::<f64>() / latencies.len() as f64
    } else {
        0.0
    };

    println!("\nResults:");
    println!("-------------------------------------------");
    println!("Total Requests:     {}", results.total);
    println!("Successful:         {}", results.success);
    println!("Failed:             {}", results.failed);
    println!("Duration:           {:.2}s", results.duration.as_secs_f64());
    println!("Requests/sec:       {:.2}", rps);
    println!();
    println!("Latency:");
    println!("  Avg:              {:.2}ms", avg_latency);
    if !latencies.is_empty() {
        println!("  Min:              {:.2}ms", latencies[0]);
        println!("  Max:              {:.2}ms", latencies[latencies.len() - 1]);
        println!("  p50:              {:.2}ms", percentile(&latencies, 0.50));
        println!("  p90:              {:.2}ms", percentile(&latencies, 0.90));
        println!("  p99:              {:.2}ms", percentile(&latencies, 0.99));
    }
    println!("===========================================");

    if rps >= 500_000.0 {
        println!("\n TARGET ACHIEVED: {:.0} RPS >= 500,000 RPS", rps);
    } else if rps >= 100_000.0 {
        println!("\n GOOD PERFORMANCE: {:.0} RPS", rps);
    } else {
        println!("\n Current RPS: {:.0}", rps);
    }

    Ok(())
}
