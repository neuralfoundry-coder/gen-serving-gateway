//! Common error types for the image serving framework

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

/// Application-wide error type
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),

    #[error("Backend not found: {0}")]
    BackendNotFound(String),

    #[error("No healthy backends available for: {0}")]
    NoHealthyBackends(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Backend error: {0}")]
    BackendError(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Error response format (OpenAI compatible)
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Serialize)]
pub struct ErrorDetail {
    pub message: String,
    pub r#type: String,
    pub code: Option<String>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, code) = match &self {
            AppError::Config(_) => (StatusCode::INTERNAL_SERVER_ERROR, "server_error", None),
            AppError::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, "server_error", None),
            AppError::Json(_) => (StatusCode::BAD_REQUEST, "invalid_request_error", Some("invalid_json")),
            AppError::HttpClient(_) => (StatusCode::BAD_GATEWAY, "backend_error", None),
            AppError::Grpc(_) => (StatusCode::BAD_GATEWAY, "backend_error", None),
            AppError::BackendNotFound(_) => (StatusCode::NOT_FOUND, "not_found_error", Some("backend_not_found")),
            AppError::NoHealthyBackends(_) => (StatusCode::SERVICE_UNAVAILABLE, "server_error", Some("no_healthy_backends")),
            AppError::AuthenticationFailed(_) => (StatusCode::UNAUTHORIZED, "authentication_error", Some("invalid_api_key")),
            AppError::RateLimitExceeded => (StatusCode::TOO_MANY_REQUESTS, "rate_limit_error", Some("rate_limit_exceeded")),
            AppError::InvalidRequest(_) => (StatusCode::BAD_REQUEST, "invalid_request_error", None),
            AppError::BackendError(_) => (StatusCode::BAD_GATEWAY, "backend_error", None),
            AppError::Timeout(_) => (StatusCode::GATEWAY_TIMEOUT, "timeout_error", None),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "server_error", None),
        };

        let body = Json(ErrorResponse {
            error: ErrorDetail {
                message: self.to_string(),
                r#type: error_type.to_string(),
                code: code.map(|c| c.to_string()),
            },
        });

        (status, body).into_response()
    }
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, AppError>;

