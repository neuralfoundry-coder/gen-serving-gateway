//! Backend registry for managing multiple image generation backends

use dashmap::DashMap;
use std::sync::Arc;
use tracing::{info, warn};

use crate::backend::grpc_backend::GrpcBackend;
use crate::backend::http_backend::HttpBackend;
use crate::backend::traits::{BackendStatus, ImageBackend};
use crate::config::BackendConfig;
use crate::error::{AppError, Result};

/// Registry for managing image generation backends
pub struct BackendRegistry {
    backends: DashMap<String, Arc<dyn ImageBackend>>,
}

impl BackendRegistry {
    /// Create a new empty backend registry
    pub fn new() -> Self {
        Self {
            backends: DashMap::new(),
        }
    }

    /// Initialize the registry from configuration
    pub async fn initialize_from_config(&self, configs: &[BackendConfig]) -> Result<()> {
        for config in configs {
            if !config.enabled {
                info!(name = %config.name, "Skipping disabled backend");
                continue;
            }

            match self.create_backend(config).await {
                Ok(backend) => {
                    self.backends.insert(config.name.clone(), backend);
                    info!(name = %config.name, protocol = %config.protocol, "Registered backend");
                }
                Err(e) => {
                    warn!(name = %config.name, error = %e, "Failed to create backend");
                }
            }
        }

        Ok(())
    }

    /// Create a backend from configuration
    async fn create_backend(&self, config: &BackendConfig) -> Result<Arc<dyn ImageBackend>> {
        match config.protocol.as_str() {
            "http" => {
                let backend = HttpBackend::new(config)?;
                Ok(Arc::new(backend))
            }
            "grpc" => {
                let backend = GrpcBackend::new(config).await?;
                Ok(Arc::new(backend))
            }
            _ => Err(AppError::Config(config::ConfigError::Message(format!(
                "Unknown protocol: {}",
                config.protocol
            )))),
        }
    }

    /// Add a new backend dynamically
    pub async fn add_backend(&self, config: BackendConfig) -> Result<()> {
        if self.backends.contains_key(&config.name) {
            return Err(AppError::InvalidRequest(format!(
                "Backend '{}' already exists",
                config.name
            )));
        }

        let backend = self.create_backend(&config).await?;
        self.backends.insert(config.name.clone(), backend);
        info!(name = %config.name, "Added new backend");

        Ok(())
    }

    /// Remove a backend
    pub async fn remove_backend(&self, name: &str) -> Result<()> {
        if self.backends.remove(name).is_none() {
            return Err(AppError::BackendNotFound(name.to_string()));
        }

        info!(name = %name, "Removed backend");
        Ok(())
    }

    /// Get a backend by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn ImageBackend>> {
        self.backends.get(name).map(|r| r.value().clone())
    }

    /// Get all backends
    pub fn get_all(&self) -> Vec<Arc<dyn ImageBackend>> {
        self.backends
            .iter()
            .map(|r| r.value().clone())
            .collect()
    }

    /// Get all healthy backends
    pub async fn get_healthy(&self) -> Vec<Arc<dyn ImageBackend>> {
        let mut healthy = Vec::new();
        for backend in self.get_all() {
            if backend.is_enabled() && backend.health_check().await {
                healthy.push(backend);
            }
        }
        healthy
    }

    /// List all backends with their status
    pub async fn list_backends(&self) -> Vec<BackendStatus> {
        // First, collect all backends to avoid holding the DashMap lock during async calls
        let backends: Vec<Arc<dyn ImageBackend>> = self.get_all();
        
        let mut statuses = Vec::new();
        
        for backend in backends {
            let healthy = backend.health_check().await;
            
            statuses.push(BackendStatus {
                name: backend.name().to_string(),
                protocol: backend.protocol().to_string(),
                endpoints: backend.endpoints(),
                healthy,
                weight: backend.weight(),
                enabled: backend.is_enabled(),
            });
        }

        statuses
    }

    /// Get the number of registered backends
    pub fn len(&self) -> usize {
        self.backends.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.backends.is_empty()
    }

    /// Check if a backend exists
    pub fn contains(&self, name: &str) -> bool {
        self.backends.contains_key(name)
    }
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_creation() {
        let registry = BackendRegistry::new();
        assert!(registry.is_empty());
    }
}

