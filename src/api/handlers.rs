//! HTTP request handlers

use crate::api::models::{
    AddBackendRequest, BackendHealthSummary, BackendInfo, BackendListResponse,
    GenerateImageRequest, GenerateImageResponse, HealthResponse, ImageData, SuccessResponse,
};
use crate::backend::traits::GenerateRequest as BackendGenerateRequest;
use crate::config::BackendConfig;
use crate::error::AppError;
use crate::AppState;
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use std::sync::Arc;
use tracing::info;

/// Generate images from a prompt
pub async fn generate_image(
    State(state): State<Arc<AppState>>,
    Json(request): Json<GenerateImageRequest>,
) -> Result<Json<GenerateImageResponse>, AppError> {
    info!(prompt = %request.prompt, n = request.n, "Received image generation request");

    let (width, height) = request.parse_size();

    // Create backend request
    let backend_request = BackendGenerateRequest {
        prompt: request.prompt.clone(),
        negative_prompt: request.negative_prompt.clone(),
        n: request.n,
        width,
        height,
        model: request.model.clone(),
        seed: request.seed,
        guidance_scale: request.guidance_scale,
        num_inference_steps: request.num_inference_steps,
        response_format: request.response_format.clone(),
    };

    // Submit request to the queue for processing
    let response = state
        .request_queue
        .submit(backend_request, request.backend.as_deref())
        .await?;

    // Convert backend response to API response
    let image_data: Vec<ImageData> = response
        .images
        .into_iter()
        .map(|img| ImageData {
            b64_json: img.b64_json,
            url: img.url,
            revised_prompt: img.revised_prompt,
        })
        .collect();

    let api_response = GenerateImageResponse {
        created: Utc::now().timestamp(),
        data: image_data,
    };

    info!(
        images_generated = api_response.data.len(),
        "Image generation completed"
    );

    Ok(Json(api_response))
}

/// List all registered backends
pub async fn list_backends(
    State(state): State<Arc<AppState>>,
) -> Result<Json<BackendListResponse>, AppError> {
    let backends = state.backend_registry.list_backends().await;
    
    let backend_infos: Vec<BackendInfo> = backends
        .into_iter()
        .map(|b| BackendInfo {
            name: b.name,
            protocol: b.protocol,
            endpoints: b.endpoints,
            healthy: b.healthy,
            weight: b.weight,
            enabled: b.enabled,
        })
        .collect();

    Ok(Json(BackendListResponse {
        backends: backend_infos,
    }))
}

/// Add a new backend dynamically
pub async fn add_backend(
    State(state): State<Arc<AppState>>,
    Json(request): Json<AddBackendRequest>,
) -> Result<Json<SuccessResponse>, AppError> {
    info!(name = %request.name, protocol = %request.protocol, "Adding new backend");

    let backend_config = BackendConfig {
        name: request.name.clone(),
        protocol: request.protocol,
        endpoints: request.endpoints,
        health_check_path: request.health_check_path,
        health_check_interval_secs: request.health_check_interval_secs,
        timeout_ms: request.timeout_ms,
        weight: request.weight,
        enabled: true,
    };

    state.backend_registry.add_backend(backend_config).await?;

    Ok(Json(SuccessResponse {
        success: true,
        message: format!("Backend '{}' added successfully", request.name),
    }))
}

/// Remove a backend
pub async fn remove_backend(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<SuccessResponse>, AppError> {
    info!(name = %name, "Removing backend");

    state.backend_registry.remove_backend(&name).await?;

    Ok(Json(SuccessResponse {
        success: true,
        message: format!("Backend '{}' removed successfully", name),
    }))
}

/// Health check endpoint
pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> Result<Json<HealthResponse>, AppError> {
    let (total, healthy, unhealthy) = state.health_manager.get_health_summary().await;

    Ok(Json(HealthResponse {
        status: if healthy > 0 { "healthy" } else { "degraded" }.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        backends: BackendHealthSummary {
            total,
            healthy,
            unhealthy,
        },
    }))
}

/// Metrics endpoint (Prometheus format placeholder)
pub async fn metrics(State(_state): State<Arc<AppState>>) -> String {
    // TODO: Implement proper Prometheus metrics
    "# HELP img_serving_requests_total Total number of image generation requests\n\
     # TYPE img_serving_requests_total counter\n\
     img_serving_requests_total 0\n"
        .to_string()
}

