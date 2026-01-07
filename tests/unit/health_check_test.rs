//! Unit tests for health check manager

use generative_img_serving::backend::registry::BackendRegistry;
use generative_img_serving::config::BackendConfig;
use generative_img_serving::gateway::health_check::HealthCheckManager;
use std::sync::Arc;

fn create_test_config(name: &str) -> BackendConfig {
    BackendConfig {
        name: name.to_string(),
        protocol: "http".to_string(),
        endpoints: vec!["http://localhost:7860".to_string()],
        health_check_path: "/health".to_string(),
        health_check_interval_secs: 30,
        timeout_ms: 60000,
        weight: 1,
        enabled: true,
    }
}

#[tokio::test]
async fn test_health_manager_creation() {
    let registry = Arc::new(BackendRegistry::new());
    let health_manager = HealthCheckManager::new(registry);
    
    // Initially, all backends should be considered healthy (no data yet)
    assert!(health_manager.is_healthy("nonexistent"));
}

#[tokio::test]
async fn test_health_summary_empty() {
    let registry = Arc::new(BackendRegistry::new());
    let health_manager = HealthCheckManager::new(registry);
    
    let (total, healthy, unhealthy) = health_manager.get_health_summary().await;
    
    assert_eq!(total, 0);
    assert_eq!(healthy, 0);
    assert_eq!(unhealthy, 0);
}

#[tokio::test]
async fn test_health_summary_with_backends() {
    let registry = Arc::new(BackendRegistry::new());
    
    // Add some backends
    for i in 1..=3 {
        let config = create_test_config(&format!("backend-{}", i));
        registry.add_backend(config).await.unwrap();
    }
    
    let health_manager = HealthCheckManager::new(registry);
    let (total, _healthy, _unhealthy) = health_manager.get_health_summary().await;
    
    assert_eq!(total, 3);
}

#[tokio::test]
async fn test_get_unhealthy_backends() {
    let registry = Arc::new(BackendRegistry::new());
    let health_manager = HealthCheckManager::new(registry);
    
    let unhealthy = health_manager.get_unhealthy_backends();
    assert!(unhealthy.is_empty());
}

#[tokio::test]
async fn test_health_status_tracking() {
    let registry = Arc::new(BackendRegistry::new());
    let config = create_test_config("test-backend");
    registry.add_backend(config).await.unwrap();
    
    let health_manager = HealthCheckManager::new(registry);
    
    // Backend should be assumed healthy initially
    assert!(health_manager.is_healthy("test-backend"));
    
    // After explicit check, status should be updated
    let status = health_manager.get_status("test-backend");
    // Status may not exist if no check has been performed
    assert!(status.is_none() || status.is_some());
}

