//! HTTP backend client implementation

use async_trait::async_trait;
use parking_lot::RwLock;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, warn};

use crate::backend::traits::{
    BackendEndpoint, GenerateRequest, GenerateResponse, GeneratedImage, ImageBackend,
};
use crate::config::BackendConfig;
use crate::error::{AppError, Result};

/// HTTP-based image generation backend
pub struct HttpBackend {
    name: String,
    client: Client,
    endpoints: Arc<RwLock<Vec<BackendEndpoint>>>,
    health_check_path: String,
    weight: u32,
    enabled: bool,
    current_endpoint_index: Arc<RwLock<usize>>,
}

/// Generic API request for HTTP backends
#[derive(Debug, Serialize)]
struct ApiGenerateRequest {
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    negative_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    n: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    guidance_scale: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_inference_steps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<String>,
}

/// Generic API response from HTTP backends
#[derive(Debug, Deserialize)]
struct ApiGenerateResponse {
    #[serde(default)]
    images: Vec<ApiImageData>,
    #[serde(default)]
    data: Vec<ApiImageData>,
    #[serde(default)]
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiImageData {
    #[serde(default)]
    b64_json: Option<String>,
    #[serde(default, alias = "base64")]
    base64: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    revised_prompt: Option<String>,
    #[serde(default)]
    seed: Option<i64>,
}

impl HttpBackend {
    /// Create a new HTTP backend from configuration
    pub fn new(config: &BackendConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .map_err(|e| AppError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        let endpoints: Vec<BackendEndpoint> = config
            .endpoints
            .iter()
            .map(|url| BackendEndpoint::new(url.clone()))
            .collect();

        Ok(Self {
            name: config.name.clone(),
            client,
            endpoints: Arc::new(RwLock::new(endpoints)),
            health_check_path: config.health_check_path.clone(),
            weight: config.weight,
            enabled: config.enabled,
            current_endpoint_index: Arc::new(RwLock::new(0)),
        })
    }

    /// Get the next healthy endpoint using round-robin
    fn get_next_endpoint(&self) -> Option<String> {
        let endpoints = self.endpoints.read();
        let healthy_endpoints: Vec<_> = endpoints
            .iter()
            .filter(|e| e.healthy)
            .collect();

        if healthy_endpoints.is_empty() {
            return None;
        }

        let mut index = self.current_endpoint_index.write();
        *index = (*index + 1) % healthy_endpoints.len();
        Some(healthy_endpoints[*index].url.clone())
    }

    /// Mark an endpoint as unhealthy
    fn mark_endpoint_unhealthy(&self, url: &str) {
        let mut endpoints = self.endpoints.write();
        if let Some(endpoint) = endpoints.iter_mut().find(|e| e.url == url) {
            endpoint.mark_unhealthy();
            warn!(backend = %self.name, url = %url, "Marked endpoint as unhealthy");
        }
    }

    /// Mark an endpoint as healthy
    fn mark_endpoint_healthy(&self, url: &str) {
        let mut endpoints = self.endpoints.write();
        if let Some(endpoint) = endpoints.iter_mut().find(|e| e.url == url) {
            endpoint.mark_healthy();
            debug!(backend = %self.name, url = %url, "Marked endpoint as healthy");
        }
    }
}

#[async_trait]
impl ImageBackend for HttpBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn protocol(&self) -> &str {
        "http"
    }

    fn endpoints(&self) -> Vec<String> {
        self.endpoints.read().iter().map(|e| e.url.clone()).collect()
    }

    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse> {
        let endpoint = self
            .get_next_endpoint()
            .ok_or_else(|| AppError::NoHealthyBackends(self.name.clone()))?;

        debug!(backend = %self.name, endpoint = %endpoint, "Sending generate request");

        let api_request = ApiGenerateRequest {
            prompt: request.prompt,
            negative_prompt: request.negative_prompt,
            n: Some(request.n),
            width: Some(request.width),
            height: Some(request.height),
            model: request.model,
            seed: request.seed,
            guidance_scale: request.guidance_scale,
            num_inference_steps: request.num_inference_steps,
            response_format: Some(request.response_format),
        };

        // Try different endpoint patterns that common image generation APIs use
        let urls_to_try = vec![
            format!("{}/v1/images/generations", endpoint),
            format!("{}/generate", endpoint),
            format!("{}/api/generate", endpoint),
            format!("{}/sdapi/v1/txt2img", endpoint), // Automatic1111 style
        ];

        let mut last_error = None;

        for url in &urls_to_try {
            match self
                .client
                .post(url)
                .json(&api_request)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.json::<ApiGenerateResponse>().await {
                            Ok(api_response) => {
                                self.mark_endpoint_healthy(&endpoint);
                                
                                // Combine images from both possible response formats
                                let mut all_images = api_response.images;
                                all_images.extend(api_response.data);
                                
                                let images: Vec<GeneratedImage> = all_images
                                    .into_iter()
                                    .map(|img| GeneratedImage {
                                        b64_json: img.b64_json.or(img.base64),
                                        url: img.url,
                                        revised_prompt: img.revised_prompt,
                                        seed: img.seed,
                                    })
                                    .collect();

                                return Ok(GenerateResponse {
                                    images,
                                    model: api_response.model,
                                });
                            }
                            Err(e) => {
                                last_error = Some(AppError::BackendError(format!(
                                    "Failed to parse response: {}",
                                    e
                                )));
                            }
                        }
                    } else {
                        let status = response.status();
                        let body = response.text().await.unwrap_or_default();
                        last_error = Some(AppError::BackendError(format!(
                            "Backend returned {}: {}",
                            status, body
                        )));
                    }
                }
                Err(e) if e.is_connect() || e.is_timeout() => {
                    // Connection or timeout error - don't try other URL patterns
                    self.mark_endpoint_unhealthy(&endpoint);
                    return Err(AppError::BackendError(format!(
                        "Connection failed to {}: {}",
                        endpoint, e
                    )));
                }
                Err(e) => {
                    last_error = Some(AppError::HttpClient(e));
                }
            }
        }

        // If we get here, none of the URL patterns worked
        self.mark_endpoint_unhealthy(&endpoint);
        Err(last_error.unwrap_or_else(|| AppError::BackendError("Unknown error".to_string())))
    }

    async fn health_check(&self) -> bool {
        let endpoints = self.endpoints.read().clone();
        let mut any_healthy = false;

        for endpoint in &endpoints {
            let url = format!("{}{}", endpoint.url, self.health_check_path);
            
            match self.client.get(&url).send().await {
                Ok(response) if response.status().is_success() => {
                    self.mark_endpoint_healthy(&endpoint.url);
                    any_healthy = true;
                    debug!(
                        backend = %self.name,
                        endpoint = %endpoint.url,
                        "Health check passed"
                    );
                }
                Ok(response) => {
                    self.mark_endpoint_unhealthy(&endpoint.url);
                    debug!(
                        backend = %self.name,
                        endpoint = %endpoint.url,
                        status = %response.status(),
                        "Health check failed"
                    );
                }
                Err(e) => {
                    self.mark_endpoint_unhealthy(&endpoint.url);
                    debug!(
                        backend = %self.name,
                        endpoint = %endpoint.url,
                        error = %e,
                        "Health check failed"
                    );
                }
            }
        }

        any_healthy
    }

    fn weight(&self) -> u32 {
        self.weight
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}

