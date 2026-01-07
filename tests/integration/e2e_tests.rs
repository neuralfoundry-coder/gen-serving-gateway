//! End-to-end integration tests

use generative_img_serving::api::models::{GenerateImageRequest, GenerateImageResponse};
use generative_img_serving::backend::registry::BackendRegistry;
use generative_img_serving::config::{BackendConfig, Settings};
use generative_img_serving::gateway::health_check::HealthCheckManager;
use generative_img_serving::gateway::load_balancer::LoadBalancer;
use generative_img_serving::queue::request_queue::RequestQueue;
use generative_img_serving::AppState;
use std::sync::Arc;
use tokio::sync::RwLock;

fn create_test_settings() -> Settings {
    let mut settings = Settings::default();
    settings.auth.enabled = false;
    settings.rate_limit.enabled = false;
    settings
}

fn create_test_backend_config(name: &str, port: u16) -> BackendConfig {
    BackendConfig {
        name: name.to_string(),
        protocol: "http".to_string(),
        endpoints: vec![format!("http://localhost:{}", port)],
        health_check_path: "/health".to_string(),
        health_check_interval_secs: 30,
        timeout_ms: 60000,
        weight: 1,
        enabled: true,
    }
}

async fn create_test_app_state() -> Arc<AppState> {
    let settings = Arc::new(RwLock::new(create_test_settings()));
    let backend_registry = Arc::new(BackendRegistry::new());
    let load_balancer = Arc::new(LoadBalancer::new(backend_registry.clone()));
    let health_manager = Arc::new(HealthCheckManager::new(backend_registry.clone()));
    let request_queue = Arc::new(RequestQueue::new(load_balancer.clone()));

    Arc::new(AppState {
        settings,
        backend_registry,
        load_balancer,
        health_manager,
        request_queue,
    })
}

#[tokio::test]
async fn test_app_state_creation() {
    let state = create_test_app_state().await;
    
    assert!(state.backend_registry.is_empty());
    
    let settings = state.settings.read().await;
    assert!(!settings.auth.enabled);
}

#[tokio::test]
async fn test_backend_registration_via_state() {
    let state = create_test_app_state().await;
    
    let config = create_test_backend_config("test-backend", 7860);
    state.backend_registry.add_backend(config).await.unwrap();
    
    assert!(!state.backend_registry.is_empty());
    assert!(state.backend_registry.contains("test-backend"));
}

#[tokio::test]
async fn test_load_balancer_with_registered_backends() {
    let state = create_test_app_state().await;
    
    // Register multiple backends
    for i in 1..=3 {
        let config = create_test_backend_config(&format!("backend-{}", i), 7860 + i);
        state.backend_registry.add_backend(config).await.unwrap();
    }
    
    // Load balancer should be able to select backends
    let backend = state.load_balancer.select_backend(None).await;
    assert!(backend.is_ok());
}

#[tokio::test]
async fn test_health_summary_integration() {
    let state = create_test_app_state().await;
    
    // Add backends
    for i in 1..=2 {
        let config = create_test_backend_config(&format!("backend-{}", i), 7860 + i);
        state.backend_registry.add_backend(config).await.unwrap();
    }
    
    let (total, _healthy, _unhealthy) = state.health_manager.get_health_summary().await;
    assert_eq!(total, 2);
}

#[tokio::test]
async fn test_generate_request_parsing() {
    let request = GenerateImageRequest {
        prompt: "A beautiful sunset".to_string(),
        model: None,
        n: 2,
        size: "512x768".to_string(),
        response_format: "b64_json".to_string(),
        negative_prompt: Some("blurry".to_string()),
        seed: Some(42),
        guidance_scale: Some(7.5),
        num_inference_steps: Some(50),
        backend: None,
    };
    
    let (width, height) = request.parse_size();
    assert_eq!(width, 512);
    assert_eq!(height, 768);
}

#[tokio::test]
async fn test_generate_request_default_size() {
    let request = GenerateImageRequest {
        prompt: "Test".to_string(),
        model: None,
        n: 1,
        size: "invalid".to_string(),
        response_format: "url".to_string(),
        negative_prompt: None,
        seed: None,
        guidance_scale: None,
        num_inference_steps: None,
        backend: None,
    };
    
    let (width, height) = request.parse_size();
    // Should fall back to default 1024x1024
    assert_eq!(width, 1024);
    assert_eq!(height, 1024);
}

#[tokio::test]
async fn test_settings_reload() {
    let state = create_test_app_state().await;
    
    // Modify settings
    {
        let mut settings = state.settings.write().await;
        settings.rate_limit.requests_per_second = 500;
    }
    
    // Verify change persists
    {
        let settings = state.settings.read().await;
        assert_eq!(settings.rate_limit.requests_per_second, 500);
    }
}

#[tokio::test]
async fn test_backend_dynamic_management() {
    let state = create_test_app_state().await;
    
    // Add a backend
    let config = create_test_backend_config("dynamic-backend", 7860);
    state.backend_registry.add_backend(config).await.unwrap();
    assert_eq!(state.backend_registry.len(), 1);
    
    // Remove it
    state.backend_registry.remove_backend("dynamic-backend").await.unwrap();
    assert_eq!(state.backend_registry.len(), 0);
}

