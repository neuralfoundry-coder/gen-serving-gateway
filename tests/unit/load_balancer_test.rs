//! Unit tests for load balancer

use generative_img_serving::backend::registry::BackendRegistry;
use generative_img_serving::config::BackendConfig;
use generative_img_serving::gateway::load_balancer::{LoadBalancer, LoadBalancingStrategy};
use std::sync::Arc;

fn create_test_config(name: &str, weight: u32) -> BackendConfig {
    BackendConfig {
        name: name.to_string(),
        protocol: "http".to_string(),
        endpoints: vec![format!("http://localhost:{}", 7860 + weight as u16)],
        health_check_path: "/health".to_string(),
        health_check_interval_secs: 30,
        timeout_ms: 60000,
        weight,
        enabled: true,
    }
}

#[tokio::test]
async fn test_load_balancer_creation() {
    let registry = Arc::new(BackendRegistry::new());
    let lb = LoadBalancer::new(registry);
    
    assert_eq!(lb.strategy(), LoadBalancingStrategy::RoundRobin);
}

#[tokio::test]
async fn test_load_balancer_set_strategy() {
    let registry = Arc::new(BackendRegistry::new());
    let lb = LoadBalancer::new(registry);
    
    lb.set_strategy(LoadBalancingStrategy::WeightedRoundRobin);
    assert_eq!(lb.strategy(), LoadBalancingStrategy::WeightedRoundRobin);
    
    lb.set_strategy(LoadBalancingStrategy::Random);
    assert_eq!(lb.strategy(), LoadBalancingStrategy::Random);
}

#[tokio::test]
async fn test_load_balancer_no_backends() {
    let registry = Arc::new(BackendRegistry::new());
    let lb = LoadBalancer::new(registry);
    
    let result = lb.select_backend(None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_load_balancer_specific_backend() {
    let registry = Arc::new(BackendRegistry::new());
    
    let config = create_test_config("test-backend", 1);
    registry.add_backend(config).await.unwrap();
    
    let lb = LoadBalancer::new(registry);
    
    let result = lb.select_backend(Some("test-backend")).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().name(), "test-backend");
}

#[tokio::test]
async fn test_load_balancer_nonexistent_backend() {
    let registry = Arc::new(BackendRegistry::new());
    let lb = LoadBalancer::new(registry);
    
    let result = lb.select_backend(Some("nonexistent")).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_round_robin_distribution() {
    let registry = Arc::new(BackendRegistry::new());
    
    // Add multiple backends
    for i in 1..=3 {
        let config = create_test_config(&format!("backend-{}", i), 1);
        registry.add_backend(config).await.unwrap();
    }
    
    let lb = LoadBalancer::new(registry);
    lb.set_strategy(LoadBalancingStrategy::RoundRobin);
    
    // Track selections
    let mut selections = std::collections::HashMap::new();
    
    for _ in 0..30 {
        let backend = lb.select_backend(None).await.unwrap();
        *selections.entry(backend.name().to_string()).or_insert(0) += 1;
    }
    
    // Each backend should be selected roughly equally
    for (_, count) in &selections {
        assert!(*count >= 8 && *count <= 12, "Expected roughly equal distribution");
    }
}

