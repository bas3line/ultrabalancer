use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

static TOTAL_SUCCESS: AtomicU64 = AtomicU64::new(0);
static TOTAL_FAILED: AtomicU64 = AtomicU64::new(0);

async fn worker(
    client: reqwest::Client,
    url: String,
    requests_per_worker: u64,
) {
    for _ in 0..requests_per_worker {
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let _ = resp.bytes().await;
                TOTAL_SUCCESS.fetch_add(1, Ordering::Relaxed);
            }
            _ => {
                TOTAL_FAILED.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 16)]
async fn main() -> anyhow::Result<()> {
    let target = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://127.0.0.1:8080/health".to_string());

    let total_requests: u64 = std::env::args()
        .nth(2)
        .and_then(|r| r.parse().ok())
        .unwrap_or(1_000_000);

    let workers: usize = std::env::args()
        .nth(3)
        .and_then(|c| c.parse().ok())
        .unwrap_or(16);

    let connections_per_worker: usize = std::env::args()
        .nth(4)
        .and_then(|c| c.parse().ok())
        .unwrap_or(500);

    println!("===========================================");
    println!("UltraBalancer Multi-Worker Stress Test");
    println!("===========================================");
    println!("Target:           {}", target);
    println!("Total Requests:   {}", total_requests);
    println!("Workers:          {}", workers);
    println!("Conns/Worker:     {}", connections_per_worker);
    println!("-------------------------------------------");

    let requests_per_worker = total_requests / workers as u64;

    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(connections_per_worker * workers)
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(Duration::from_secs(60))
        .timeout(Duration::from_secs(5))
        .tcp_nodelay(true)
        .build()?;

    let start = Instant::now();

    let mut handles = Vec::with_capacity(workers);
    for _ in 0..workers {
        let client = client.clone();
        let url = target.clone();
        handles.push(tokio::spawn(async move {
            let mut tasks = Vec::with_capacity(connections_per_worker);
            let per_task = requests_per_worker / connections_per_worker as u64;

            for _ in 0..connections_per_worker {
                let c = client.clone();
                let u = url.clone();
                tasks.push(tokio::spawn(worker(c, u, per_task)));
            }

            for t in tasks {
                let _ = t.await;
            }
        }));
    }

    for handle in handles {
        let _ = handle.await;
    }

    let duration = start.elapsed();
    let success = TOTAL_SUCCESS.load(Ordering::Relaxed);
    let failed = TOTAL_FAILED.load(Ordering::Relaxed);
    let rps = success as f64 / duration.as_secs_f64();

    println!("\nResults:");
    println!("-------------------------------------------");
    println!("Successful:       {}", success);
    println!("Failed:           {}", failed);
    println!("Duration:         {:.2}s", duration.as_secs_f64());
    println!("Requests/sec:     {:.2}", rps);
    println!("===========================================");

    if rps >= 500_000.0 {
        println!("\n TARGET ACHIEVED: {:.0} RPS >= 500,000 RPS", rps);
    } else if rps >= 100_000.0 {
        println!("\n EXCELLENT: {:.0} RPS", rps);
    } else {
        println!("\n Current RPS: {:.0}", rps);
    }

    Ok(())
}
