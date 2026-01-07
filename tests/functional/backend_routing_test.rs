//! Functional tests for backend routing

use generative_img_serving::backend::registry::BackendRegistry;
use generative_img_serving::config::BackendConfig;
use generative_img_serving::gateway::health_check::HealthCheckManager;
use generative_img_serving::gateway::router::{Router as GatewayRouter, RouterConfig};
use std::sync::Arc;

fn create_test_config(name: &str, protocol: &str, weight: u32) -> BackendConfig {
    BackendConfig {
        name: name.to_string(),
        protocol: protocol.to_string(),
        endpoints: vec![format!("http://{}:8000", name)],
        health_check_path: "/health".to_string(),
        health_check_interval_secs: 30,
        timeout_ms: 60000,
        weight,
        enabled: true,
    }
}

#[tokio::test]
async fn test_router_with_specific_backend() {
    let registry = Arc::new(BackendRegistry::new());
    
    // Add backends
    registry.add_backend(create_test_config("backend-1", "http", 1)).await.unwrap();
    registry.add_backend(create_test_config("backend-2", "http", 1)).await.unwrap();
    
    let health_manager = Arc::new(HealthCheckManager::new(registry.clone()));
    let router = GatewayRouter::new(registry, health_manager);
    
    // Should route to specific backend when requested
    let backend = router.route(Some("backend-1"), None).await.unwrap();
    assert_eq!(backend.name(), "backend-1");
}

#[tokio::test]
async fn test_router_nonexistent_backend() {
    let registry = Arc::new(BackendRegistry::new());
    let health_manager = Arc::new(HealthCheckManager::new(registry.clone()));
    let router = GatewayRouter::new(registry, health_manager);
    
    // Should fail for nonexistent backend
    let result = router.route(Some("nonexistent"), None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_router_default_backend() {
    let registry = Arc::new(BackendRegistry::new());
    
    registry.add_backend(create_test_config("default-backend", "http", 1)).await.unwrap();
    registry.add_backend(create_test_config("other-backend", "http", 1)).await.unwrap();
    
    let health_manager = Arc::new(HealthCheckManager::new(registry.clone()));
    
    let config = RouterConfig {
        default_backend: Some("default-backend".to_string()),
        fallback_enabled: true,
    };
    
    let router = GatewayRouter::with_config(registry, health_manager, config);
    
    // Without specific backend, should use default
    let backend = router.route(None, None).await.unwrap();
    assert_eq!(backend.name(), "default-backend");
}

#[tokio::test]
async fn test_router_fallback_when_no_default() {
    let registry = Arc::new(BackendRegistry::new());
    
    registry.add_backend(create_test_config("backend-1", "http", 1)).await.unwrap();
    
    let health_manager = Arc::new(HealthCheckManager::new(registry.clone()));
    
    let config = RouterConfig {
        default_backend: None,
        fallback_enabled: true,
    };
    
    let router = GatewayRouter::with_config(registry, health_manager, config);
    
    // Should fall back to any healthy backend
    let result = router.route(None, None).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_router_no_fallback() {
    let registry = Arc::new(BackendRegistry::new());
    
    registry.add_backend(create_test_config("backend-1", "http", 1)).await.unwrap();
    
    let health_manager = Arc::new(HealthCheckManager::new(registry.clone()));
    
    let config = RouterConfig {
        default_backend: None,
        fallback_enabled: false,
    };
    
    let router = GatewayRouter::with_config(registry, health_manager, config);
    
    // Without default and fallback disabled, should fail
    let result = router.route(None, None).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_router_model_based_routing() {
    let registry = Arc::new(BackendRegistry::new());
    
    registry.add_backend(create_test_config("stable-diffusion", "http", 1)).await.unwrap();
    registry.add_backend(create_test_config("dalle-backend", "http", 1)).await.unwrap();
    
    let health_manager = Arc::new(HealthCheckManager::new(registry.clone()));
    
    let config = RouterConfig {
        default_backend: None,
        fallback_enabled: true,
    };
    
    let router = GatewayRouter::with_config(registry, health_manager, config);
    
    // Should route based on model name matching backend name
    let backend = router.route(None, Some("stable-diffusion-v1")).await.unwrap();
    assert_eq!(backend.name(), "stable-diffusion");
}

#[tokio::test]
async fn test_registry_list_backends() {
    let registry = Arc::new(BackendRegistry::new());
    
    registry.add_backend(create_test_config("backend-1", "http", 1)).await.unwrap();
    registry.add_backend(create_test_config("backend-2", "http", 2)).await.unwrap();
    registry.add_backend(create_test_config("backend-3", "grpc", 1)).await.unwrap();
    
    let backends = registry.list_backends().await;
    
    assert_eq!(backends.len(), 3);
    
    // Check that we have backends of different types
    let protocols: Vec<_> = backends.iter().map(|b| b.protocol.as_str()).collect();
    assert!(protocols.contains(&"http"));
    assert!(protocols.contains(&"grpc"));
}

#[tokio::test]
async fn test_registry_remove_backend() {
    let registry = Arc::new(BackendRegistry::new());
    
    registry.add_backend(create_test_config("to-remove", "http", 1)).await.unwrap();
    assert!(registry.contains("to-remove"));
    
    registry.remove_backend("to-remove").await.unwrap();
    assert!(!registry.contains("to-remove"));
}

#[tokio::test]
async fn test_registry_duplicate_backend() {
    let registry = Arc::new(BackendRegistry::new());
    
    registry.add_backend(create_test_config("duplicate", "http", 1)).await.unwrap();
    
    // Adding duplicate should fail
    let result = registry.add_backend(create_test_config("duplicate", "http", 1)).await;
    assert!(result.is_err());
}

