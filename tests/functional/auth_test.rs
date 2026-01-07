//! Functional tests for API Key authentication

use axum::{
    body::Body,
    http::{header::AUTHORIZATION, Request, StatusCode},
    Router,
};
use tower::ServiceExt;
use generative_img_serving::middleware::auth::AuthLayer;

async fn create_test_app() -> Router {
    Router::new()
        .route("/test", axum::routing::get(|| async { "OK" }))
        .layer(AuthLayer::new(vec![
            "valid-key-1".to_string(),
            "valid-key-2".to_string(),
        ]))
}

#[tokio::test]
async fn test_auth_with_valid_bearer_token() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header(AUTHORIZATION, "Bearer valid-key-1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_auth_with_valid_key_no_bearer() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header(AUTHORIZATION, "valid-key-1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_auth_with_invalid_key() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header(AUTHORIZATION, "Bearer invalid-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_without_header() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_health_endpoint_bypass() {
    let app = Router::new()
        .route("/health", axum::routing::get(|| async { "healthy" }))
        .route("/test", axum::routing::get(|| async { "OK" }))
        .layer(AuthLayer::new(vec!["valid-key".to_string()]));
    
    // Health endpoint should bypass auth
    let response = app
        .clone()
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
async fn test_auth_empty_keys_allows_all() {
    let app = Router::new()
        .route("/test", axum::routing::get(|| async { "OK" }))
        .layer(AuthLayer::new(vec![]));
    
    // When no keys configured, all requests should be allowed
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
async fn test_auth_second_valid_key() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(
            Request::builder()
                .uri("/test")
                .header(AUTHORIZATION, "Bearer valid-key-2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}

