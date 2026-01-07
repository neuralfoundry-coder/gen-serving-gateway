//! Dynamic router for routing requests to appropriate backends

use std::sync::Arc;
use tracing::debug;

use crate::backend::registry::BackendRegistry;
use crate::backend::traits::ImageBackend;
use crate::error::{AppError, Result};
use crate::gateway::health_check::HealthCheckManager;

/// Router configuration
#[derive(Debug, Clone)]
pub struct RouterConfig {
    /// Default backend to use when none is specified
    pub default_backend: Option<String>,
    /// Whether to route to any healthy backend if default is unavailable
    pub fallback_enabled: bool,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            default_backend: None,
            fallback_enabled: true,
        }
    }
}

/// Dynamic router for backend selection
pub struct Router {
    registry: Arc<BackendRegistry>,
    health_manager: Arc<HealthCheckManager>,
    config: RouterConfig,
}

impl Router {
    /// Create a new router
    pub fn new(
        registry: Arc<BackendRegistry>,
        health_manager: Arc<HealthCheckManager>,
    ) -> Self {
        Self {
            registry,
            health_manager,
            config: RouterConfig::default(),
        }
    }

    /// Create a new router with configuration
    pub fn with_config(
        registry: Arc<BackendRegistry>,
        health_manager: Arc<HealthCheckManager>,
        config: RouterConfig,
    ) -> Self {
        Self {
            registry,
            health_manager,
            config,
        }
    }

    /// Route a request to an appropriate backend
    pub async fn route(
        &self,
        backend_name: Option<&str>,
        model: Option<&str>,
    ) -> Result<Arc<dyn ImageBackend>> {
        // Priority 1: Explicitly requested backend
        if let Some(name) = backend_name {
            return self.get_healthy_backend(name).await;
        }

        // Priority 2: Route based on model name
        if let Some(model) = model {
            if let Some(backend) = self.route_by_model(model).await {
                return Ok(backend);
            }
        }

        // Priority 3: Default backend
        if let Some(ref default) = self.config.default_backend {
            if let Ok(backend) = self.get_healthy_backend(default).await {
                return Ok(backend);
            }
        }

        // Priority 4: Any healthy backend (if fallback enabled)
        if self.config.fallback_enabled {
            return self.get_any_healthy_backend().await;
        }

        Err(AppError::NoHealthyBackends("No available backends".to_string()))
    }

    /// Get a specific backend if it's healthy
    async fn get_healthy_backend(&self, name: &str) -> Result<Arc<dyn ImageBackend>> {
        let backend = self
            .registry
            .get(name)
            .ok_or_else(|| AppError::BackendNotFound(name.to_string()))?;

        if !backend.is_enabled() {
            return Err(AppError::BackendNotFound(format!(
                "Backend '{}' is disabled",
                name
            )));
        }

        if !self.health_manager.is_healthy(name) {
            return Err(AppError::NoHealthyBackends(name.to_string()));
        }

        debug!(backend = %name, "Routed to backend");
        Ok(backend)
    }

    /// Route based on model name
    /// This can be extended to support model-to-backend mapping
    async fn route_by_model(&self, model: &str) -> Option<Arc<dyn ImageBackend>> {
        // Simple heuristic: look for backends that might support the model
        // This could be enhanced with a proper model registry
        
        let model_lower = model.to_lowercase();
        
        for backend in self.registry.get_all() {
            if !backend.is_enabled() {
                continue;
            }
            
            if !self.health_manager.is_healthy(backend.name()) {
                continue;
            }

            let backend_name = backend.name().to_lowercase();
            
            // Check if backend name matches model name pattern
            if model_lower.contains(&backend_name) || backend_name.contains(&model_lower) {
                debug!(backend = %backend.name(), model = %model, "Routed by model name");
                return Some(backend);
            }

            // Check for common model patterns
            if model_lower.contains("stable") || model_lower.contains("sd") {
                if backend_name.contains("stable") || backend_name.contains("sd") {
                    return Some(backend);
                }
            }

            if model_lower.contains("dall") {
                if backend_name.contains("dall") || backend_name.contains("openai") {
                    return Some(backend);
                }
            }
        }

        None
    }

    /// Get any healthy backend
    async fn get_any_healthy_backend(&self) -> Result<Arc<dyn ImageBackend>> {
        for backend in self.registry.get_all() {
            if backend.is_enabled() && self.health_manager.is_healthy(backend.name()) {
                debug!(backend = %backend.name(), "Routed to first healthy backend");
                return Ok(backend);
            }
        }

        Err(AppError::NoHealthyBackends("all".to_string()))
    }

    /// Set the default backend
    pub fn set_default_backend(&mut self, name: Option<String>) {
        self.config.default_backend = name;
    }

    /// Enable or disable fallback routing
    pub fn set_fallback_enabled(&mut self, enabled: bool) {
        self.config.fallback_enabled = enabled;
    }
}

