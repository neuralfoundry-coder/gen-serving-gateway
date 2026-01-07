//! API Key authentication middleware

use axum::{
    body::Body,
    http::{header::AUTHORIZATION, Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use futures::future::BoxFuture;
use serde::Serialize;
use std::{
    collections::HashSet,
    sync::Arc,
    task::{Context, Poll},
};
use tower::{Layer, Service};
use tracing::warn;

/// Authentication error response
#[derive(Serialize)]
struct AuthError {
    error: AuthErrorDetail,
}

#[derive(Serialize)]
struct AuthErrorDetail {
    message: String,
    r#type: String,
    code: String,
}

/// Authentication layer
#[derive(Clone)]
pub struct AuthLayer {
    api_keys: Arc<HashSet<String>>,
}

impl AuthLayer {
    pub fn new(api_keys: Vec<String>) -> Self {
        Self {
            api_keys: Arc::new(api_keys.into_iter().collect()),
        }
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware {
            inner,
            api_keys: self.api_keys.clone(),
        }
    }
}

/// Authentication middleware service
#[derive(Clone)]
pub struct AuthMiddleware<S> {
    inner: S,
    api_keys: Arc<HashSet<String>>,
}

impl<S> Service<Request<Body>> for AuthMiddleware<S>
where
    S: Service<Request<Body>, Response = Response> + Send + Clone + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        // Skip authentication for health check and metrics endpoints
        let path = request.uri().path();
        if path == "/health" || path == "/metrics" {
            let future = self.inner.call(request);
            return Box::pin(async move { future.await });
        }

        // Extract API key from Authorization header
        let auth_header = request
            .headers()
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok());

        let api_key = auth_header.and_then(|h| {
            if h.starts_with("Bearer ") {
                Some(h.trim_start_matches("Bearer ").to_string())
            } else {
                Some(h.to_string())
            }
        });

        // If no API keys are configured, allow all requests
        if self.api_keys.is_empty() {
            let future = self.inner.call(request);
            return Box::pin(async move { future.await });
        }

        // Validate API key
        match api_key {
            Some(key) if self.api_keys.contains(&key) => {
                let future = self.inner.call(request);
                Box::pin(async move { future.await })
            }
            Some(_) => {
                warn!("Invalid API key provided");
                Box::pin(async move {
                    Ok(create_auth_error_response("Invalid API key"))
                })
            }
            None => {
                warn!("No API key provided");
                Box::pin(async move {
                    Ok(create_auth_error_response(
                        "API key required. Provide via Authorization header: 'Bearer YOUR_API_KEY'",
                    ))
                })
            }
        }
    }
}

fn create_auth_error_response(message: &str) -> Response {
    let error = AuthError {
        error: AuthErrorDetail {
            message: message.to_string(),
            r#type: "authentication_error".to_string(),
            code: "invalid_api_key".to_string(),
        },
    };
    
    (StatusCode::UNAUTHORIZED, Json(error)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_layer_creation() {
        let layer = AuthLayer::new(vec!["test-key".to_string()]);
        assert!(layer.api_keys.contains("test-key"));
    }
}

