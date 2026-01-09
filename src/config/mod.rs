pub mod parser;
pub mod validator;

use anyhow::Result;
use parser::ConfigParser;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use validator::ConfigValidator;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub listen_address: String,
    pub listen_port: u16,
    pub algorithm: String,
    pub backends: Vec<BackendConfig>,

    #[serde(default)]
    pub workers: WorkerConfig,

    #[serde(default)]
    pub max_connections: Option<usize>,

    #[serde(default = "default_health_check")]
    pub health_check: HealthCheckConfig,

    #[serde(default)]
    pub timeout: TimeoutConfig,

    #[serde(default)]
    pub logging: LoggingConfig,

    #[serde(default)]
    pub metrics: MetricsConfig,

    #[serde(default)]
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub host: String,
    pub port: u16,

    #[serde(default = "default_weight")]
    pub weight: u32,

    #[serde(default)]
    pub max_connections: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WorkerConfig {
    Auto,
    Count(usize),
}

impl Default for WorkerConfig {
    fn default() -> Self {
        WorkerConfig::Auto
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default = "default_interval")]
    pub interval_ms: u64,

    #[serde(default = "default_max_failures")]
    pub max_failures: u32,

    #[serde(default = "default_health_path")]
    pub path: String,

    #[serde(default = "default_expected_status")]
    pub expected_status: u16,

    #[serde(default = "default_health_timeout")]
    pub timeout_ms: u64,

    #[serde(default)]
    pub headers: HashMap<String, String>,

    #[serde(default)]
    pub expected_body: Option<String>,

    #[serde(default)]
    pub circuit_breaker: Option<CircuitBreakerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default = "default_cb_failure_threshold")]
    pub failure_threshold: u32,

    #[serde(default = "default_cb_success_threshold")]
    pub success_threshold: u32,

    #[serde(default = "default_cb_timeout")]
    pub timeout_seconds: u64,

    #[serde(default = "default_cb_half_open")]
    pub half_open_requests: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    #[serde(default = "default_connect_timeout")]
    pub connect_ms: u64,

    #[serde(default = "default_request_timeout")]
    pub request_ms: u64,

    #[serde(default = "default_idle_timeout")]
    pub idle_ms: u64,

    #[serde(default = "default_keepalive")]
    pub keepalive_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,

    #[serde(default = "default_log_format")]
    pub format: String,

    #[serde(default = "default_log_output")]
    pub output: String,

    #[serde(default = "default_log_max_size")]
    pub max_size_mb: u64,

    #[serde(default = "default_log_max_files")]
    pub max_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default = "default_metrics_endpoint")]
    pub endpoint: String,

    #[serde(default = "default_prometheus_endpoint")]
    pub prometheus_endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    pub cert_path: String,
    pub key_path: String,

    #[serde(default)]
    pub key_password: Option<String>,

    #[serde(default)]
    pub client_auth: bool,

    #[serde(default = "default_tls_version")]
    pub min_version: String,
}

fn default_weight() -> u32 {
    100
}

fn default_true() -> bool {
    true
}

fn default_interval() -> u64 {
    5000
}

fn default_max_failures() -> u32 {
    3
}

fn default_health_path() -> String {
    "/".to_string()
}

fn default_expected_status() -> u16 {
    200
}

fn default_health_timeout() -> u64 {
    2000
}

fn default_cb_failure_threshold() -> u32 {
    5
}

fn default_cb_success_threshold() -> u32 {
    2
}

fn default_cb_timeout() -> u64 {
    60
}

fn default_cb_half_open() -> u32 {
    3
}

fn default_connect_timeout() -> u64 {
    5000
}

fn default_request_timeout() -> u64 {
    30000
}

fn default_idle_timeout() -> u64 {
    60000
}

fn default_keepalive() -> u64 {
    75000
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "text".to_string()
}

fn default_log_output() -> String {
    "stdout".to_string()
}

fn default_log_max_size() -> u64 {
    100
}

fn default_log_max_files() -> usize {
    10
}

fn default_metrics_endpoint() -> String {
    "/metrics".to_string()
}

fn default_prometheus_endpoint() -> String {
    "/prometheus".to_string()
}

fn default_tls_version() -> String {
    "1.2".to_string()
}

fn default_health_check() -> HealthCheckConfig {
    HealthCheckConfig {
        enabled: true,
        interval_ms: 5000,
        max_failures: 3,
        path: "/".to_string(),
        expected_status: 200,
        timeout_ms: 2000,
        headers: HashMap::new(),
        expected_body: None,
        circuit_breaker: None,
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect_ms: default_connect_timeout(),
            request_ms: default_request_timeout(),
            idle_ms: default_idle_timeout(),
            keepalive_ms: default_keepalive(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            output: default_log_output(),
            max_size_mb: default_log_max_size(),
            max_files: default_log_max_files(),
        }
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            endpoint: default_metrics_endpoint(),
            prometheus_endpoint: default_prometheus_endpoint(),
        }
    }
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        ConfigParser::from_file(path)
    }

    pub fn from_yaml(content: &str) -> Result<Self> {
        ConfigParser::from_yaml(content)
    }

    pub fn from_toml(content: &str) -> Result<Self> {
        ConfigParser::from_toml(content)
    }

    pub fn validate(&self) -> Result<()> {
        ConfigValidator::validate(self)
    }

    pub fn worker_count(&self) -> usize {
        match self.workers {
            WorkerConfig::Auto => num_cpus::get(),
            WorkerConfig::Count(n) => n,
        }
    }
}
