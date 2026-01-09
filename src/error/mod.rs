use thiserror::Error;

#[derive(Error, Debug)]
pub enum LoadBalancerError {
    #[error("No healthy backends available")]
    NoHealthyBackends,

    #[error("Backend selection failed: {0}")]
    BackendSelectionFailed(String),

    #[error("Invalid backend address: {0}")]
    InvalidBackendAddress(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Health check failed for backend {backend}: {reason}")]
    HealthCheckFailed { backend: String, reason: String },

    #[error("Connection pool exhausted")]
    PoolExhausted,

    #[error("Request timeout after {0}ms")]
    RequestTimeout(u64),

    #[error("Connection refused: {0}")]
    ConnectionRefused(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Circuit breaker open for backend: {0}")]
    CircuitBreakerOpen(String),

    #[error("Proxy error: {0}")]
    ProxyError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    ParseError(String),
}

pub type Result<T> = std::result::Result<T, LoadBalancerError>;
