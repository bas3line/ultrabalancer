use parking_lot::RwLock;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
        }
    }
}

pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    success_count: Arc<RwLock<u32>>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            config,
        }
    }

    pub fn is_available(&self) -> bool {
        let state = *self.state.read();

        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = *self.last_failure_time.read() {
                    if last_failure.elapsed() >= self.config.timeout {
                        self.transition_to_half_open();
                        return true;
                    }
                }
                false
            }
            CircuitState::HalfOpen => true,
        }
    }

    pub fn record_success(&self) {
        let state = *self.state.read();

        match state {
            CircuitState::Closed => {
                *self.failure_count.write() = 0;
            }
            CircuitState::HalfOpen => {
                let mut success_count = self.success_count.write();
                *success_count += 1;

                if *success_count >= self.config.success_threshold {
                    self.transition_to_closed();
                }
            }
            CircuitState::Open => {}
        }
    }

    pub fn record_failure(&self) {
        let state = *self.state.read();

        match state {
            CircuitState::Closed => {
                let mut failure_count = self.failure_count.write();
                *failure_count += 1;

                if *failure_count >= self.config.failure_threshold {
                    drop(failure_count);
                    self.transition_to_open();
                }
            }
            CircuitState::HalfOpen => {
                self.transition_to_open();
            }
            CircuitState::Open => {
                *self.last_failure_time.write() = Some(Instant::now());
            }
        }
    }

    pub fn state(&self) -> CircuitState {
        *self.state.read()
    }

    fn transition_to_closed(&self) {
        *self.state.write() = CircuitState::Closed;
        *self.failure_count.write() = 0;
        *self.success_count.write() = 0;
    }

    fn transition_to_open(&self) {
        *self.state.write() = CircuitState::Open;
        *self.last_failure_time.write() = Some(Instant::now());
        *self.success_count.write() = 0;
    }

    fn transition_to_half_open(&self) {
        *self.state.write() = CircuitState::HalfOpen;
        *self.success_count.write() = 0;
    }
}

impl Clone for CircuitBreaker {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            failure_count: Arc::clone(&self.failure_count),
            success_count: Arc::clone(&self.success_count),
            last_failure_time: Arc::clone(&self.last_failure_time),
            config: self.config.clone(),
        }
    }
}
