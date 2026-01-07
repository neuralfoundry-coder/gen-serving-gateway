//! Application settings and configuration management

use crate::error::{AppError, Result};
use config::{Config, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Root configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub rate_limit: RateLimitConfig,
    pub storage: StorageConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub backends: Vec<BackendConfig>,
}

/// Server configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

/// Authentication configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub api_keys: Vec<String>,
}

fn default_true() -> bool {
    true
}

/// Rate limiting configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RateLimitConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_rps")]
    pub requests_per_second: u32,
    #[serde(default = "default_burst")]
    pub burst_size: u32,
}

fn default_rps() -> u32 {
    100
}

fn default_burst() -> u32 {
    200
}

/// Storage configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    #[serde(default = "default_storage_path")]
    pub base_path: String,
    #[serde(default = "default_url_prefix")]
    pub url_prefix: String,
}

fn default_storage_path() -> String {
    "./generated_images".to_string()
}

fn default_url_prefix() -> String {
    "http://localhost:8080/images".to_string()
}

/// Logging configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "json".to_string()
}

/// Backend configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackendConfig {
    pub name: String,
    #[serde(default = "default_protocol")]
    pub protocol: String,
    pub endpoints: Vec<String>,
    #[serde(default = "default_health_check_path")]
    pub health_check_path: String,
    #[serde(default = "default_health_check_interval")]
    pub health_check_interval_secs: u64,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_weight")]
    pub weight: u32,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_protocol() -> String {
    "http".to_string()
}

fn default_health_check_path() -> String {
    "/health".to_string()
}

fn default_health_check_interval() -> u64 {
    30
}

fn default_timeout() -> u64 {
    60000
}

fn default_weight() -> u32 {
    1
}

impl Settings {
    /// Load settings from configuration files and environment variables
    pub fn load() -> Result<Self> {
        Self::load_from_path("config/default.toml")
    }

    /// Load settings from a specific configuration file path
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let config = Config::builder()
            // Start with default values
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("auth.enabled", true)?
            .set_default("rate_limit.enabled", true)?
            .set_default("rate_limit.requests_per_second", 100)?
            .set_default("rate_limit.burst_size", 200)?
            // Load from configuration file
            .add_source(File::with_name(path.as_ref().to_str().unwrap_or("config/default")).required(false))
            // Override with environment variables (prefixed with IMG_SERVING_)
            .add_source(
                Environment::with_prefix("IMG_SERVING")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?;

        let settings: Settings = config.try_deserialize()?;
        Ok(settings)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate server config
        if self.server.port == 0 {
            return Err(AppError::Config(config::ConfigError::Message(
                "Server port cannot be 0".to_string(),
            )));
        }

        // Validate backends
        for backend in &self.backends {
            if backend.name.is_empty() {
                return Err(AppError::Config(config::ConfigError::Message(
                    "Backend name cannot be empty".to_string(),
                )));
            }
            if backend.endpoints.is_empty() {
                return Err(AppError::Config(config::ConfigError::Message(
                    format!("Backend '{}' must have at least one endpoint", backend.name),
                )));
            }
            if !["http", "grpc"].contains(&backend.protocol.as_str()) {
                return Err(AppError::Config(config::ConfigError::Message(
                    format!(
                        "Backend '{}' has invalid protocol '{}'. Must be 'http' or 'grpc'",
                        backend.name, backend.protocol
                    ),
                )));
            }
        }

        Ok(())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: default_host(),
                port: default_port(),
            },
            auth: AuthConfig {
                enabled: true,
                api_keys: vec![],
            },
            rate_limit: RateLimitConfig {
                enabled: true,
                requests_per_second: default_rps(),
                burst_size: default_burst(),
            },
            storage: StorageConfig {
                base_path: default_storage_path(),
                url_prefix: default_url_prefix(),
            },
            logging: LoggingConfig {
                level: default_log_level(),
                format: default_log_format(),
            },
            backends: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.server.host, "0.0.0.0");
        assert_eq!(settings.server.port, 8080);
        assert!(settings.auth.enabled);
        assert!(settings.rate_limit.enabled);
    }
}

