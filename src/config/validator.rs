use super::Config;
use anyhow::Result;

pub struct ConfigValidator;

impl ConfigValidator {
    pub fn validate(config: &Config) -> Result<()> {
        if config.backends.is_empty() {
            anyhow::bail!("At least one backend server is required");
        }

        if config.listen_port == 0 || config.listen_port > 65535 {
            anyhow::bail!("Invalid listen port: {}", config.listen_port);
        }

        for backend in &config.backends {
            if backend.port == 0 || backend.port > 65535 {
                anyhow::bail!("Invalid backend port: {}", backend.port);
            }

            if backend.host.is_empty() {
                anyhow::bail!("Backend host cannot be empty");
            }

            if backend.weight == 0 {
                anyhow::bail!("Backend weight must be greater than 0");
            }
        }

        if config.health_check.interval_ms == 0 {
            anyhow::bail!("Health check interval must be greater than 0");
        }

        if config.health_check.max_failures == 0 {
            anyhow::bail!("Health check max failures must be greater than 0");
        }

        Ok(())
    }
}
