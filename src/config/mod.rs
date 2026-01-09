pub mod parser;
pub mod validator;

use anyhow::Result;
use parser::ConfigParser;
use serde::{Deserialize, Serialize};
use std::path::Path;
use validator::ConfigValidator;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub listen_address: String,
    pub listen_port: u16,
    pub algorithm: String,
    pub backends: Vec<BackendConfig>,
    #[serde(default = "default_health_check")]
    pub health_check: HealthCheckConfig,
    #[serde(default)]
    pub timeout: TimeoutConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub host: String,
    pub port: u16,
    #[serde(default = "default_weight")]
    pub weight: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_interval")]
    pub interval_ms: u64,
    #[serde(default = "default_max_failures")]
    pub max_failures: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    #[serde(default = "default_connect_timeout")]
    pub connect_ms: u64,
    #[serde(default = "default_request_timeout")]
    pub request_ms: u64,
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

fn default_connect_timeout() -> u64 {
    5000
}

fn default_request_timeout() -> u64 {
    30000
}

fn default_health_check() -> HealthCheckConfig {
    HealthCheckConfig {
        enabled: true,
        interval_ms: 5000,
        max_failures: 3,
    }
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect_ms: default_connect_timeout(),
            request_ms: default_request_timeout(),
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
}
