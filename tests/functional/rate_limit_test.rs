//! Functional tests for rate limiting

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use tower::ServiceExt;
use generative_img_serving::middleware::rate_limit::RateLimitLayer;

async fn create_test_app(rps: u32, burst: u32) -> Router {
    Router::new()
        .route("/test", axum::routing::get(|| async { "OK" }))
        .layer(RateLimitLayer::new(rps, burst))
}

#[tokio::test]
async fn test_rate_limit_allows_within_limit() {
    // Allow 100 requests per second with burst of 100
    let app = create_test_app(100, 100).await;
    
    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_rate_limit_health_bypass() {
    let app = Router::new()
        .route("/health", axum::routing::get(|| async { "healthy" }))
        .route("/test", axum::routing::get(|| async { "OK" }))
        .layer(RateLimitLayer::new(1, 1)); // Very strict limit
    
    // Health endpoint should bypass rate limiting
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_rate_limit_metrics_bypass() {
    let app = Router::new()
        .route("/metrics", axum::routing::get(|| async { "metrics" }))
        .route("/test", axum::routing::get(|| async { "OK" }))
        .layer(RateLimitLayer::new(1, 1)); // Very strict limit
    
    // Metrics endpoint should bypass rate limiting
    let response = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_rate_limit_exceeded() {
    // Very low limit: 1 request per second, no burst
    let rps = 1;
    let burst = 1;
    
    let app = Router::new()
        .route("/test", axum::routing::get(|| async { "OK" }))
        .layer(RateLimitLayer::new(rps, burst));
    
    // First request should succeed
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    // Rapid subsequent requests should eventually be rate limited
    let mut rate_limited = false;
    for _ in 0..10 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        if response.status() == StatusCode::TOO_MANY_REQUESTS {
            rate_limited = true;
            break;
        }
    }
    
    assert!(rate_limited, "Expected rate limiting to kick in");
}

#[tokio::test]
async fn test_rate_limit_burst_capacity() {
    // Allow burst of 5 requests
    let app = create_test_app(1, 5).await;
    
    // All 5 burst requests should succeed
    for _ in 0..5 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        
        assert_eq!(response.status(), StatusCode::OK);
    }
}

