//! Common traits and types for image generation backends

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;

/// Request to generate images
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    /// The prompt to generate images from
    pub prompt: String,
    
    /// Negative prompt (things to avoid)
    pub negative_prompt: Option<String>,
    
    /// Number of images to generate
    pub n: u32,
    
    /// Image width
    pub width: u32,
    
    /// Image height
    pub height: u32,
    
    /// Model identifier
    pub model: Option<String>,
    
    /// Random seed for reproducibility
    pub seed: Option<i64>,
    
    /// Guidance scale / CFG scale
    pub guidance_scale: Option<f32>,
    
    /// Number of inference steps
    pub num_inference_steps: Option<u32>,
    
    /// Response format: "b64_json", "url", or "file"
    pub response_format: String,
}

/// Generated image data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedImage {
    /// Base64 encoded image data
    pub b64_json: Option<String>,
    
    /// URL to the image
    pub url: Option<String>,
    
    /// Revised prompt if the model modified it
    pub revised_prompt: Option<String>,
    
    /// Seed used for generation
    pub seed: Option<i64>,
}

/// Response from image generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateResponse {
    /// List of generated images
    pub images: Vec<GeneratedImage>,
    
    /// Model used for generation
    pub model: Option<String>,
}

/// Backend status information
#[derive(Debug, Clone)]
pub struct BackendStatus {
    pub name: String,
    pub protocol: String,
    pub endpoints: Vec<String>,
    pub healthy: bool,
    pub weight: u32,
    pub enabled: bool,
}

/// Trait for image generation backends
#[async_trait]
pub trait ImageBackend: Send + Sync {
    /// Get the backend name
    fn name(&self) -> &str;
    
    /// Get the backend protocol (http or grpc)
    fn protocol(&self) -> &str;
    
    /// Get the list of endpoints
    fn endpoints(&self) -> Vec<String>;
    
    /// Generate images from a request
    async fn generate(&self, request: GenerateRequest) -> Result<GenerateResponse>;
    
    /// Check if the backend is healthy
    async fn health_check(&self) -> bool;
    
    /// Get the backend weight for load balancing
    fn weight(&self) -> u32;
    
    /// Check if the backend is enabled
    fn is_enabled(&self) -> bool;
    
    /// Get current status
    fn status(&self) -> BackendStatus {
        BackendStatus {
            name: self.name().to_string(),
            protocol: self.protocol().to_string(),
            endpoints: self.endpoints(),
            healthy: true, // Will be updated by health check
            weight: self.weight(),
            enabled: self.is_enabled(),
        }
    }
}

/// Backend endpoint with health status
#[derive(Debug, Clone)]
pub struct BackendEndpoint {
    pub url: String,
    pub healthy: bool,
    pub last_check: Option<std::time::Instant>,
    pub consecutive_failures: u32,
}

impl BackendEndpoint {
    pub fn new(url: String) -> Self {
        Self {
            url,
            healthy: true, // Assume healthy until proven otherwise
            last_check: None,
            consecutive_failures: 0,
        }
    }
    
    pub fn mark_healthy(&mut self) {
        self.healthy = true;
        self.last_check = Some(std::time::Instant::now());
        self.consecutive_failures = 0;
    }
    
    pub fn mark_unhealthy(&mut self) {
        self.consecutive_failures += 1;
        if self.consecutive_failures >= 3 {
            self.healthy = false;
        }
        self.last_check = Some(std::time::Instant::now());
    }
}

