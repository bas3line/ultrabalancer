use crate::backend::pool::ServerPool;
use crate::backend::server::Server;
use crate::config::HealthCheckConfig;
use super::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{debug, info, warn};

pub struct HealthChecker {
    pool: ServerPool,
    config: HealthCheckConfig,
    circuit_breakers: Arc<parking_lot::RwLock<HashMap<String, CircuitBreaker>>>,
}

impl HealthChecker {
    pub fn new(pool: ServerPool, config: HealthCheckConfig) -> Self {
        Self {
            pool,
            config,
            circuit_breakers: Arc::new(parking_lot::RwLock::new(HashMap::new())),
        }
    }

    fn get_or_create_circuit_breaker(&self, server_addr: &str) -> Option<CircuitBreaker> {
        let cb_config = self.config.circuit_breaker.as_ref()?;

        if !cb_config.enabled {
            return None;
        }

        let mut breakers = self.circuit_breakers.write();
        Some(
            breakers
                .entry(server_addr.to_string())
                .or_insert_with(|| {
                    CircuitBreaker::new(CircuitBreakerConfig {
                        failure_threshold: cb_config.failure_threshold,
                        success_threshold: cb_config.success_threshold,
                        timeout: Duration::from_secs(cb_config.timeout_seconds),
                    })
                })
                .clone(),
        )
    }

    pub async fn start(self: Arc<Self>) {
        if !self.config.enabled {
            info!("Health checking disabled");
            return;
        }

        info!(
            "Health checker started (interval: {}ms, max_failures: {})",
            self.config.interval_ms, self.config.max_failures
        );

        let mut interval_timer = time::interval(Duration::from_millis(self.config.interval_ms));
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
        let circuit_breaker = self.get_or_create_circuit_breaker(&server.address());

        if let Some(ref cb) = circuit_breaker {
            if !cb.is_available() {
                debug!("Circuit breaker OPEN for {}, skipping health check", server.address());
                return;
            }
        }

        let url = format!("http://{}{}", server.address(), self.config.path);
        let was_healthy = server.is_healthy();

        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(self.config.timeout_ms))
            .build()
            .unwrap();
        let start = std::time::Instant::now();

        let mut request = client.get(&url);
        for (key, value) in &self.config.headers {
            request = request.header(key, value);
        }

        match request.send().await {
            Ok(response) => {
                let duration = start.elapsed();
                let status = response.status().as_u16();

                debug!(
                    "Health check response from {}: {} ({:?})",
                    server.address(),
                    status,
                    duration
                );

                let status_ok = status == self.config.expected_status;
                let mut body_ok = true;

                if let Some(ref expected_body) = self.config.expected_body {
                    if let Ok(body_text) = response.text().await {
                        body_ok = body_text.contains(expected_body);
                    } else {
                        body_ok = false;
                    }
                }

                if status_ok && body_ok {
                    server.reset_failures();
                    server.set_response_time(duration);

                    if let Some(cb) = circuit_breaker {
                        cb.record_success();
                    }

                    if !was_healthy {
                        server.mark_healthy();
                        info!("✓ Server {} recovered [UP]", server.address());
                    }
                } else {
                    let reason = if !status_ok {
                        format!("HTTP {} (expected {})", status, self.config.expected_status)
                    } else {
                        "Body validation failed".to_string()
                    };

                    if let Some(cb) = circuit_breaker {
                        cb.record_failure();
                    }

                    self.handle_failure(server, was_healthy, reason).await;
                }
            }
            Err(e) => {
                if let Some(cb) = circuit_breaker {
                    cb.record_failure();
                }

                self.handle_failure(server, was_healthy, e.to_string()).await;
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
            circuit_breakers: Arc::clone(&self.circuit_breakers),
        }
    }
}
