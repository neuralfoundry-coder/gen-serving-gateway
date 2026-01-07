//! Unit tests for request queue and batcher

use generative_img_serving::queue::batcher::{Batcher, BatchConfig};
use generative_img_serving::queue::request_queue::QueueConfig;

#[test]
fn test_queue_config_defaults() {
    let config = QueueConfig::default();
    
    assert_eq!(config.max_queue_size, 1000);
    assert_eq!(config.max_concurrent, 10);
    assert_eq!(config.timeout_ms, 120000);
}

#[test]
fn test_batch_config_defaults() {
    let config = BatchConfig::default();
    
    assert_eq!(config.max_batch_size, 4);
    assert_eq!(config.max_wait_ms, 100);
    assert!(config.enabled);
}

#[tokio::test]
async fn test_batcher_creation() {
    let batcher = Batcher::new();
    
    // Initially no pending requests
    assert_eq!(batcher.pending_count().await, 0);
}

#[tokio::test]
async fn test_batcher_with_custom_config() {
    let config = BatchConfig {
        max_batch_size: 8,
        max_wait_ms: 200,
        enabled: true,
    };
    
    let batcher = Batcher::with_config(config);
    assert_eq!(batcher.pending_count().await, 0);
}

#[tokio::test]
async fn test_batcher_disabled() {
    let config = BatchConfig {
        max_batch_size: 4,
        max_wait_ms: 100,
        enabled: false,
    };
    
    let batcher = Batcher::with_config(config);
    
    // Should not process when disabled
    assert!(!batcher.should_process().await);
}

