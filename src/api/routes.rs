//! HTTP route definitions

use crate::api::handlers;
use crate::middleware::{auth::AuthLayer, rate_limit::RateLimitLayer};
use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

/// Create the main application router
pub async fn create_router(state: Arc<crate::AppState>) -> Router {
    // Get configuration for middleware
    let (auth_enabled, api_keys, rate_limit_enabled, rps, burst) = {
        let config = state.settings.read().await;
        (
            config.auth.enabled,
            config.auth.api_keys.clone(),
            config.rate_limit.enabled,
            config.rate_limit.requests_per_second,
            config.rate_limit.burst_size,
        )
    };

    // Build the API routes that require authentication and rate limiting
    let api_routes = Router::new()
        // Image generation endpoint (OpenAI compatible)
        .route("/images/generations", post(handlers::generate_image))
        // Backend management endpoints
        .route("/backends", get(handlers::list_backends))
        .route("/backends", post(handlers::add_backend))
        .route("/backends/:name", delete(handlers::remove_backend));

    // Apply middleware conditionally
    let api_routes = if rate_limit_enabled {
        api_routes.layer(RateLimitLayer::new(rps, burst))
    } else {
        api_routes
    };

    let api_routes = if auth_enabled {
        api_routes.layer(AuthLayer::new(api_keys))
    } else {
        api_routes
    };

    // Build the full router
    Router::new()
        // Health check endpoint (no auth required)
        .route("/health", get(handlers::health_check))
        // Metrics endpoint (no auth required)
        .route("/metrics", get(handlers::metrics))
        // Static file serving for generated images
        .nest_service("/images", tower_http::services::ServeDir::new("generated_images"))
        // API routes under /v1 prefix
        .nest("/v1", api_routes)
        // Add shared state
        .with_state(state)
        // Add tracing layer
        .layer(TraceLayer::new_for_http())
}

