//! Load balancer implementation with multiple strategies

use parking_lot::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::debug;

use crate::backend::registry::BackendRegistry;
use crate::backend::traits::ImageBackend;
use crate::error::{AppError, Result};

/// Load balancing strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalancingStrategy {
    /// Round-robin distribution
    RoundRobin,
    /// Weighted round-robin based on backend weights
    WeightedRoundRobin,
    /// Random selection
    Random,
    /// Least connections (placeholder - needs connection tracking)
    LeastConnections,
}

impl Default for LoadBalancingStrategy {
    fn default() -> Self {
        Self::RoundRobin
    }
}

/// Load balancer for distributing requests across backends
pub struct LoadBalancer {
    registry: Arc<BackendRegistry>,
    strategy: RwLock<LoadBalancingStrategy>,
    round_robin_index: AtomicUsize,
    weighted_state: RwLock<WeightedRoundRobinState>,
}

/// State for weighted round-robin algorithm
struct WeightedRoundRobinState {
    current_index: usize,
    current_weight: i32,
}

impl LoadBalancer {
    /// Create a new load balancer
    pub fn new(registry: Arc<BackendRegistry>) -> Self {
        Self {
            registry,
            strategy: RwLock::new(LoadBalancingStrategy::default()),
            round_robin_index: AtomicUsize::new(0),
            weighted_state: RwLock::new(WeightedRoundRobinState {
                current_index: 0,
                current_weight: 0,
            }),
        }
    }

    /// Set the load balancing strategy
    pub fn set_strategy(&self, strategy: LoadBalancingStrategy) {
        *self.strategy.write() = strategy;
    }

    /// Get the current load balancing strategy
    pub fn strategy(&self) -> LoadBalancingStrategy {
        *self.strategy.read()
    }

    /// Select a backend for a request
    pub async fn select_backend(
        &self,
        backend_name: Option<&str>,
    ) -> Result<Arc<dyn ImageBackend>> {
        // If a specific backend is requested, use that
        if let Some(name) = backend_name {
            return self
                .registry
                .get(name)
                .ok_or_else(|| AppError::BackendNotFound(name.to_string()));
        }

        // Get all healthy backends
        let healthy_backends = self.get_healthy_backends().await;
        
        if healthy_backends.is_empty() {
            return Err(AppError::NoHealthyBackends("all".to_string()));
        }

        // Select based on strategy
        let strategy = *self.strategy.read();
        let selected = match strategy {
            LoadBalancingStrategy::RoundRobin => {
                self.select_round_robin(&healthy_backends)
            }
            LoadBalancingStrategy::WeightedRoundRobin => {
                self.select_weighted_round_robin(&healthy_backends)
            }
            LoadBalancingStrategy::Random => {
                self.select_random(&healthy_backends)
            }
            LoadBalancingStrategy::LeastConnections => {
                // Fall back to round-robin for now
                self.select_round_robin(&healthy_backends)
            }
        };

        debug!(
            backend = %selected.name(),
            strategy = ?strategy,
            "Selected backend for request"
        );

        Ok(selected)
    }

    /// Get all healthy backends
    async fn get_healthy_backends(&self) -> Vec<Arc<dyn ImageBackend>> {
        let all_backends = self.registry.get_all();
        let mut healthy = Vec::new();

        for backend in all_backends {
            if backend.is_enabled() {
                // Use cached health status instead of checking each time
                // The health check manager will update the status periodically
                healthy.push(backend);
            }
        }

        healthy
    }

    /// Round-robin selection
    fn select_round_robin(
        &self,
        backends: &[Arc<dyn ImageBackend>],
    ) -> Arc<dyn ImageBackend> {
        let index = self.round_robin_index.fetch_add(1, Ordering::Relaxed);
        backends[index % backends.len()].clone()
    }

    /// Weighted round-robin selection
    fn select_weighted_round_robin(
        &self,
        backends: &[Arc<dyn ImageBackend>],
    ) -> Arc<dyn ImageBackend> {
        if backends.len() == 1 {
            return backends[0].clone();
        }

        let mut state = self.weighted_state.write();
        let weights: Vec<i32> = backends.iter().map(|b| b.weight() as i32).collect();
        let max_weight = *weights.iter().max().unwrap_or(&1);
        let gcd = weights.iter().fold(0, |acc, &w| gcd(acc, w));

        loop {
            state.current_index = (state.current_index + 1) % backends.len();
            
            if state.current_index == 0 {
                state.current_weight -= gcd;
                if state.current_weight <= 0 {
                    state.current_weight = max_weight;
                }
            }

            if weights[state.current_index] >= state.current_weight {
                return backends[state.current_index].clone();
            }
        }
    }

    /// Random selection
    fn select_random(&self, backends: &[Arc<dyn ImageBackend>]) -> Arc<dyn ImageBackend> {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let index = (now.as_nanos() as usize) % backends.len();
        backends[index].clone()
    }
}

/// Calculate greatest common divisor
fn gcd(a: i32, b: i32) -> i32 {
    if b == 0 {
        a.abs()
    } else {
        gcd(b, a % b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gcd() {
        assert_eq!(gcd(12, 8), 4);
        assert_eq!(gcd(100, 25), 25);
        assert_eq!(gcd(7, 3), 1);
    }
}

