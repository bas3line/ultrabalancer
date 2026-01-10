use std::time::Duration;
use tokio::time::sleep;

#[derive(Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub multiplier: f64,
    pub jitter: bool,
    pub retry_statuses: Vec<u16>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 10000,
            multiplier: 2.0,
            jitter: true,
            retry_statuses: vec![502, 503, 504],
        }
    }
}

pub struct RetryMiddleware {
    config: RetryConfig,
}

impl RetryMiddleware {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    pub fn should_retry(&self, attempt: u32, status: Option<u16>, is_error: bool) -> bool {
        if attempt >= self.config.max_attempts {
            return false;
        }

        if is_error {
            return true;
        }

        if let Some(status) = status {
            return self.config.retry_statuses.contains(&status);
        }

        false
    }

    /// Wait before retry attempt. Called with attempt=0 for first retry, attempt=1 for second, etc.
    /// Delay: initial_delay_ms * multiplier^attempt (e.g., 100ms, 200ms, 400ms for multiplier=2)
    pub async fn wait(&self, attempt: u32) {
        let base_delay = self.config.initial_delay_ms as f64
            * self.config.multiplier.powi(attempt as i32);
        let capped_delay = base_delay.min(self.config.max_delay_ms as f64);

        let delay = if self.config.jitter {
            let jitter = fastrand::f64() * 0.3;
            capped_delay * (1.0 + jitter - 0.15)
        } else {
            capped_delay
        };

        sleep(Duration::from_millis(delay as u64)).await;
    }

    pub fn max_attempts(&self) -> u32 {
        self.config.max_attempts
    }
}

impl Clone for RetryMiddleware {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
        }
    }
}

pub struct RetryState {
    pub attempt: u32,
    pub last_error: Option<String>,
    pub last_status: Option<u16>,
}

impl RetryState {
    pub fn new() -> Self {
        Self {
            attempt: 0,
            last_error: None,
            last_status: None,
        }
    }

    pub fn increment(&mut self, error: Option<String>, status: Option<u16>) {
        self.attempt += 1;
        self.last_error = error;
        self.last_status = status;
    }
}

impl Default for RetryState {
    fn default() -> Self {
        Self::new()
    }
}
