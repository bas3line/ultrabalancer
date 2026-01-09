use crate::backend::{Backend, BackendPool};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};

pub struct HealthChecker {
    pool: Arc<BackendPool>,
    interval: Duration,
    max_failures: u32,
    enabled: bool,
}

impl HealthChecker {
    pub fn new(pool: Arc<BackendPool>, interval: Duration, max_failures: u32, enabled: bool) -> Self {
        Self {
            pool,
            interval,
            max_failures,
            enabled,
        }
    }

    pub async fn start(self: Arc<Self>) {
        if !self.enabled {
            info!("Health checking disabled");
            return;
        }

        info!(
            "Health checker started (interval: {}s, max_failures: {})",
            self.interval.as_secs(),
            self.max_failures
        );

        let mut interval_timer = time::interval(self.interval);
        interval_timer.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

        loop {
            interval_timer.tick().await;
            self.check_all_backends().await;
        }
    }

    async fn check_all_backends(&self) {
        let backends = self.pool.get_all_backends();

        for backend in backends {
            self.check_backend(backend).await;
        }
    }

    async fn check_backend(&self, backend: Backend) {
        let url = format!("http://{}", backend.address());
        let was_healthy = backend.is_healthy();

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();

        match client.head(&url).send().await {
            Ok(response) => {
                let status = response.status().as_u16();

                if status >= 200 && status < 400 {
                    backend.reset_failures();
                    if !was_healthy {
                        backend.mark_healthy();
                        info!("✓ Backend {} is UP", backend.address());
                    }
                } else {
                    self.handle_failure(&backend, was_healthy).await;
                }
            }
            Err(e) => {
                self.handle_failure(&backend, was_healthy).await;
            }
        }
    }

    async fn handle_failure(&self, backend: &Backend, was_healthy: bool) {
        let failures = backend.increment_failures();

        if failures >= self.max_failures && was_healthy {
            backend.mark_unhealthy();
            warn!(
                "✗ Backend {} marked DOWN after {} failed checks",
                backend.address(),
                failures
            );
        }
    }
}
