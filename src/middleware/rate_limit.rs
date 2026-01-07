//! Rate limiting middleware using the Governor crate

use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use futures::future::BoxFuture;
use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use serde::Serialize;
use std::{
    num::NonZeroU32,
    sync::Arc,
    task::{Context, Poll},
};
use tower::{Layer, Service};
use tracing::warn;

/// Rate limit error response
#[derive(Serialize)]
struct RateLimitError {
    error: RateLimitErrorDetail,
}

#[derive(Serialize)]
struct RateLimitErrorDetail {
    message: String,
    r#type: String,
    code: String,
}

type SharedRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>;

/// Rate limiting layer
#[derive(Clone)]
pub struct RateLimitLayer {
    limiter: SharedRateLimiter,
}

impl RateLimitLayer {
    pub fn new(requests_per_second: u32, burst_size: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(requests_per_second).unwrap_or(NonZeroU32::new(100).unwrap()))
            .allow_burst(NonZeroU32::new(burst_size).unwrap_or(NonZeroU32::new(200).unwrap()));
        
        let limiter = Arc::new(RateLimiter::direct(quota));
        
        Self { limiter }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitMiddleware {
            inner,
            limiter: self.limiter.clone(),
        }
    }
}

/// Rate limiting middleware service
#[derive(Clone)]
pub struct RateLimitMiddleware<S> {
    inner: S,
    limiter: SharedRateLimiter,
}

impl<S> Service<Request<Body>> for RateLimitMiddleware<S>
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
        // Skip rate limiting for health check and metrics endpoints
        let path = request.uri().path();
        if path == "/health" || path == "/metrics" {
            let future = self.inner.call(request);
            return Box::pin(async move { future.await });
        }

        // Check rate limit
        match self.limiter.check() {
            Ok(_) => {
                let future = self.inner.call(request);
                Box::pin(async move { future.await })
            }
            Err(_) => {
                warn!("Rate limit exceeded");
                Box::pin(async move {
                    Ok(create_rate_limit_error_response())
                })
            }
        }
    }
}

fn create_rate_limit_error_response() -> Response {
    let error = RateLimitError {
        error: RateLimitErrorDetail {
            message: "Rate limit exceeded. Please slow down your requests.".to_string(),
            r#type: "rate_limit_error".to_string(),
            code: "rate_limit_exceeded".to_string(),
        },
    };
    
    (StatusCode::TOO_MANY_REQUESTS, Json(error)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_layer_creation() {
        let layer = RateLimitLayer::new(100, 200);
        // Should not panic
        assert!(layer.limiter.check().is_ok());
    }
}

