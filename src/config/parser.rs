use super::Config;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub struct ConfigParser;

impl ConfigParser {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Config> {
        let content = fs::read_to_string(&path).context("Failed to read config file")?;
        let path_str = path.as_ref().to_string_lossy();

        if path_str.ends_with(".yaml") || path_str.ends_with(".yml") {
            Self::from_yaml(&content)
        } else if path_str.ends_with(".toml") {
            Self::from_toml(&content)
        } else {
            Self::from_yaml(&content)
        }
    }

    pub fn from_yaml(content: &str) -> Result<Config> {
        serde_yaml::from_str(content).context("Failed to parse YAML config")
    }

    pub fn from_toml(content: &str) -> Result<Config> {
        toml::from_str(content).context("Failed to parse TOML config")
    }
}
