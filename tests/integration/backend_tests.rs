//! Backend integration tests

use generative_img_serving::backend::registry::BackendRegistry;
use generative_img_serving::config::BackendConfig;

#[tokio::test]
async fn test_registry_creation() {
    let registry = BackendRegistry::new();
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
}

#[tokio::test]
async fn test_registry_add_http_backend() {
    let registry = BackendRegistry::new();
    
    let config = BackendConfig {
        name: "test-backend".to_string(),
        protocol: "http".to_string(),
        endpoints: vec!["http://localhost:7860".to_string()],
        health_check_path: "/health".to_string(),
        health_check_interval_secs: 30,
        timeout_ms: 60000,
        weight: 1,
        enabled: true,
    };

    let result = registry.add_backend(config).await;
    assert!(result.is_ok());
    assert_eq!(registry.len(), 1);
    assert!(registry.contains("test-backend"));
}

#[tokio::test]
async fn test_registry_remove_backend() {
    let registry = BackendRegistry::new();
    
    let config = BackendConfig {
        name: "test-backend".to_string(),
        protocol: "http".to_string(),
        endpoints: vec!["http://localhost:7860".to_string()],
        health_check_path: "/health".to_string(),
        health_check_interval_secs: 30,
        timeout_ms: 60000,
        weight: 1,
        enabled: true,
    };

    registry.add_backend(config).await.unwrap();
    assert_eq!(registry.len(), 1);

    let result = registry.remove_backend("test-backend").await;
    assert!(result.is_ok());
    assert!(registry.is_empty());
}

#[tokio::test]
async fn test_registry_remove_nonexistent() {
    let registry = BackendRegistry::new();
    let result = registry.remove_backend("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_registry_duplicate_backend() {
    let registry = BackendRegistry::new();
    
    let config = BackendConfig {
        name: "test-backend".to_string(),
        protocol: "http".to_string(),
        endpoints: vec!["http://localhost:7860".to_string()],
        health_check_path: "/health".to_string(),
        health_check_interval_secs: 30,
        timeout_ms: 60000,
        weight: 1,
        enabled: true,
    };

    registry.add_backend(config.clone()).await.unwrap();
    let result = registry.add_backend(config).await;
    assert!(result.is_err()); // Should fail because backend already exists
}

