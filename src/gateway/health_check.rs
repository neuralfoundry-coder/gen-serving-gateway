//! Health check manager for monitoring backend health

use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use crate::backend::registry::BackendRegistry;

/// Health status of a backend
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub healthy: bool,
    pub last_check: std::time::Instant,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self {
            healthy: true, // Assume healthy until proven otherwise
            last_check: std::time::Instant::now(),
            consecutive_failures: 0,
            consecutive_successes: 0,
        }
    }
}

/// Health check manager
pub struct HealthCheckManager {
    registry: Arc<BackendRegistry>,
    health_status: DashMap<String, HealthStatus>,
    check_task: RwLock<Option<JoinHandle<()>>>,
    /// Number of consecutive failures before marking unhealthy
    failure_threshold: u32,
    /// Number of consecutive successes before marking healthy again
    recovery_threshold: u32,
}

impl HealthCheckManager {
    /// Create a new health check manager
    pub fn new(registry: Arc<BackendRegistry>) -> Self {
        Self {
            registry,
            health_status: DashMap::new(),
            check_task: RwLock::new(None),
            failure_threshold: 3,
            recovery_threshold: 2,
        }
    }

    /// Start the health check background task
    pub async fn start(&self, interval_secs: u64) {
        let registry = self.registry.clone();
        let health_status = self.health_status.clone();
        let failure_threshold = self.failure_threshold;
        let recovery_threshold = self.recovery_threshold;

        let handle = tokio::spawn(async move {
            let interval = Duration::from_secs(interval_secs);
            
            loop {
                // Check all backends
                for backend in registry.get_all() {
                    let name = backend.name().to_string();
                    let is_healthy = backend.health_check().await;

                    let mut status = health_status
                        .entry(name.clone())
                        .or_insert_with(HealthStatus::default);

                    status.last_check = std::time::Instant::now();

                    if is_healthy {
                        status.consecutive_failures = 0;
                        status.consecutive_successes += 1;

                        if !status.healthy && status.consecutive_successes >= recovery_threshold {
                            status.healthy = true;
                            info!(backend = %name, "Backend recovered and marked healthy");
                        }
                    } else {
                        status.consecutive_successes = 0;
                        status.consecutive_failures += 1;

                        if status.healthy && status.consecutive_failures >= failure_threshold {
                            status.healthy = false;
                            warn!(
                                backend = %name,
                                failures = status.consecutive_failures,
                                "Backend marked unhealthy after consecutive failures"
                            );
                        }
                    }

                    debug!(
                        backend = %name,
                        healthy = status.healthy,
                        consecutive_failures = status.consecutive_failures,
                        consecutive_successes = status.consecutive_successes,
                        "Health check completed"
                    );
                }

                tokio::time::sleep(interval).await;
            }
        });

        *self.check_task.write().await = Some(handle);
        info!(interval_secs = interval_secs, "Started health check background task");
    }

    /// Stop the health check background task
    pub async fn stop(&self) {
        if let Some(handle) = self.check_task.write().await.take() {
            handle.abort();
            info!("Stopped health check background task");
        }
    }

    /// Check if a specific backend is healthy
    pub fn is_healthy(&self, name: &str) -> bool {
        self.health_status
            .get(name)
            .map(|s| s.healthy)
            .unwrap_or(true) // Assume healthy if not checked yet
    }

    /// Get health status for a backend
    pub fn get_status(&self, name: &str) -> Option<HealthStatus> {
        self.health_status.get(name).map(|s| s.clone())
    }

    /// Get health summary (total, healthy, unhealthy)
    pub async fn get_health_summary(&self) -> (usize, usize, usize) {
        let backends = self.registry.get_all();
        let total = backends.len();
        
        let mut healthy = 0;
        let mut unhealthy = 0;

        for backend in backends {
            if self.is_healthy(backend.name()) {
                healthy += 1;
            } else {
                unhealthy += 1;
            }
        }

        (total, healthy, unhealthy)
    }

    /// Force a health check for a specific backend
    pub async fn check_now(&self, name: &str) -> Option<bool> {
        let backend = self.registry.get(name)?;
        let is_healthy = backend.health_check().await;

        let mut status = self.health_status
            .entry(name.to_string())
            .or_insert_with(HealthStatus::default);

        status.last_check = std::time::Instant::now();
        status.healthy = is_healthy;

        if is_healthy {
            status.consecutive_failures = 0;
            status.consecutive_successes += 1;
        } else {
            status.consecutive_successes = 0;
            status.consecutive_failures += 1;
        }

        Some(is_healthy)
    }

    /// Get all unhealthy backends
    pub fn get_unhealthy_backends(&self) -> Vec<String> {
        self.health_status
            .iter()
            .filter(|entry| !entry.healthy)
            .map(|entry| entry.key().clone())
            .collect()
    }
}

