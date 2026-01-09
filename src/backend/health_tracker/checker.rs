use crate::backend::pool::ServerPool;
use crate::backend::server::Server;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    pub interval: Duration,
    pub timeout: Duration,
    pub max_failures: u32,
    pub enabled: bool,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(5),
            timeout: Duration::from_secs(2),
            max_failures: 3,
            enabled: true,
        }
    }
}

pub struct HealthChecker {
    pool: ServerPool,
    config: HealthCheckConfig,
}

impl HealthChecker {
    pub fn new(pool: ServerPool, config: HealthCheckConfig) -> Self {
        Self { pool, config }
    }

    pub async fn start(self: Arc<Self>) {
        if !self.config.enabled {
            info!("Health checking disabled");
            return;
        }

        info!(
            "Health checker started (interval: {:?}, max_failures: {})",
            self.config.interval, self.config.max_failures
        );

        let mut interval_timer = time::interval(self.config.interval);
        interval_timer.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        loop {
            interval_timer.tick().await;
            self.check_all_servers().await;
        }
    }

    async fn check_all_servers(&self) {
        let servers = self.pool.get_all_servers();
        let mut handles = Vec::new();

        for server in servers {
            let checker = self.clone();
            let handle = tokio::spawn(async move {
                checker.check_server(server).await;
            });
            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }
    }

    async fn check_server(&self, server: Server) {
        let url = format!("http://{}", server.address());
        let was_healthy = server.is_healthy();

        let client = reqwest::Client::builder()
            .timeout(self.config.timeout)
            .build()
            .unwrap();

        let start = std::time::Instant::now();

        match client.head(&url).send().await {
            Ok(response) => {
                let duration = start.elapsed();
                let status = response.status().as_u16();

                debug!(
                    "Health check response from {}: {} ({:?})",
                    server.address(),
                    status,
                    duration
                );

                if (200..400).contains(&status) {
                    server.reset_failures();
                    server.set_response_time(duration);

                    if !was_healthy {
                        server.mark_healthy();
                        info!("✓ Server {} recovered [UP]", server.address());
                    }
                } else {
                    self.handle_failure(server, was_healthy, format!("HTTP {}", status))
                        .await;
                }
            }
            Err(e) => {
                self.handle_failure(server, was_healthy, e.to_string())
                    .await;
            }
        }
    }

    async fn handle_failure(&self, server: Server, was_healthy: bool, reason: String) {
        let failures = server.increment_failures();

        debug!(
            "Health check failed for {} (count: {}): {}",
            server.address(),
            failures,
            reason
        );

        if failures >= self.config.max_failures && was_healthy {
            server.mark_unhealthy();
            warn!(
                "✗ Server {} marked [DOWN] after {} failures",
                server.address(),
                failures
            );
        }
    }
}

impl Clone for HealthChecker {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            config: self.config.clone(),
        }
    }
}
