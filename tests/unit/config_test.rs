//! Unit tests for configuration module

use generative_img_serving::config::{Settings, BackendConfig};

#[test]
fn test_default_settings() {
    let settings = Settings::default();
    
    assert_eq!(settings.server.host, "0.0.0.0");
    assert_eq!(settings.server.port, 8080);
    assert!(settings.auth.enabled);
    assert!(settings.rate_limit.enabled);
    assert_eq!(settings.rate_limit.requests_per_second, 100);
    assert_eq!(settings.rate_limit.burst_size, 200);
}

#[test]
fn test_settings_validation_valid() {
    let mut settings = Settings::default();
    settings.server.port = 8080;
    settings.backends = vec![
        BackendConfig {
            name: "test-backend".to_string(),
            protocol: "http".to_string(),
            endpoints: vec!["http://localhost:7860".to_string()],
            health_check_path: "/health".to_string(),
            health_check_interval_secs: 30,
            timeout_ms: 60000,
            weight: 1,
            enabled: true,
        }
    ];
    
    assert!(settings.validate().is_ok());
}

#[test]
fn test_settings_validation_invalid_port() {
    let mut settings = Settings::default();
    settings.server.port = 0;
    
    assert!(settings.validate().is_err());
}

#[test]
fn test_settings_validation_empty_backend_name() {
    let mut settings = Settings::default();
    settings.backends = vec![
        BackendConfig {
            name: "".to_string(),
            protocol: "http".to_string(),
            endpoints: vec!["http://localhost:7860".to_string()],
            health_check_path: "/health".to_string(),
            health_check_interval_secs: 30,
            timeout_ms: 60000,
            weight: 1,
            enabled: true,
        }
    ];
    
    assert!(settings.validate().is_err());
}

#[test]
fn test_settings_validation_no_endpoints() {
    let mut settings = Settings::default();
    settings.backends = vec![
        BackendConfig {
            name: "test".to_string(),
            protocol: "http".to_string(),
            endpoints: vec![],
            health_check_path: "/health".to_string(),
            health_check_interval_secs: 30,
            timeout_ms: 60000,
            weight: 1,
            enabled: true,
        }
    ];
    
    assert!(settings.validate().is_err());
}

#[test]
fn test_settings_validation_invalid_protocol() {
    let mut settings = Settings::default();
    settings.backends = vec![
        BackendConfig {
            name: "test".to_string(),
            protocol: "websocket".to_string(),
            endpoints: vec!["ws://localhost:7860".to_string()],
            health_check_path: "/health".to_string(),
            health_check_interval_secs: 30,
            timeout_ms: 60000,
            weight: 1,
            enabled: true,
        }
    ];
    
    assert!(settings.validate().is_err());
}

#[test]
fn test_backend_config_defaults() {
    let config = BackendConfig {
        name: "test".to_string(),
        protocol: "http".to_string(),
        endpoints: vec!["http://localhost:7860".to_string()],
        health_check_path: "/health".to_string(),
        health_check_interval_secs: 30,
        timeout_ms: 60000,
        weight: 1,
        enabled: true,
    };
    
    assert_eq!(config.weight, 1);
    assert!(config.enabled);
}

