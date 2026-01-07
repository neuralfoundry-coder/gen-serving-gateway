//! API request and response models (OpenAI compatible)

use serde::{Deserialize, Serialize};

/// Image generation request (OpenAI compatible)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GenerateImageRequest {
    /// The prompt to generate images from
    pub prompt: String,
    
    /// The model to use for generation (optional, uses default if not specified)
    #[serde(default)]
    pub model: Option<String>,
    
    /// Number of images to generate (1-10)
    #[serde(default = "default_n")]
    pub n: u32,
    
    /// The size of the generated images (e.g., "1024x1024")
    #[serde(default = "default_size")]
    pub size: String,
    
    /// The format of the response: "url", "b64_json", or "file"
    #[serde(default = "default_response_format")]
    pub response_format: String,
    
    /// Negative prompt (extension, not in OpenAI API)
    #[serde(default)]
    pub negative_prompt: Option<String>,
    
    /// Random seed for reproducibility (extension)
    #[serde(default)]
    pub seed: Option<i64>,
    
    /// Guidance scale / CFG scale (extension)
    #[serde(default)]
    pub guidance_scale: Option<f32>,
    
    /// Number of inference steps (extension)
    #[serde(default)]
    pub num_inference_steps: Option<u32>,
    
    /// Specific backend to use (extension)
    #[serde(default)]
    pub backend: Option<String>,
}

fn default_n() -> u32 {
    1
}

fn default_size() -> String {
    "1024x1024".to_string()
}

fn default_response_format() -> String {
    "url".to_string()
}

impl GenerateImageRequest {
    /// Parse size string into width and height
    pub fn parse_size(&self) -> (u32, u32) {
        let parts: Vec<&str> = self.size.split('x').collect();
        if parts.len() == 2 {
            let width = parts[0].parse().unwrap_or(1024);
            let height = parts[1].parse().unwrap_or(1024);
            (width, height)
        } else {
            (1024, 1024)
        }
    }
}

/// Image data in the response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImageData {
    /// Base64 encoded image (when response_format is b64_json)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b64_json: Option<String>,
    
    /// URL to the generated image (when response_format is url)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    
    /// Revised prompt (if model modified the prompt)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revised_prompt: Option<String>,
}

/// Image generation response (OpenAI compatible)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GenerateImageResponse {
    /// Unix timestamp of creation
    pub created: i64,
    
    /// List of generated images
    pub data: Vec<ImageData>,
}

/// Backend information for management API
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackendInfo {
    pub name: String,
    pub protocol: String,
    pub endpoints: Vec<String>,
    pub healthy: bool,
    pub weight: u32,
    pub enabled: bool,
}

/// Backend list response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackendListResponse {
    pub backends: Vec<BackendInfo>,
}

/// Add backend request
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AddBackendRequest {
    pub name: String,
    pub protocol: String,
    pub endpoints: Vec<String>,
    #[serde(default = "default_health_check_path")]
    pub health_check_path: String,
    #[serde(default = "default_health_check_interval")]
    pub health_check_interval_secs: u64,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default = "default_weight")]
    pub weight: u32,
}

fn default_health_check_path() -> String {
    "/health".to_string()
}

fn default_health_check_interval() -> u64 {
    30
}

fn default_timeout() -> u64 {
    60000
}

fn default_weight() -> u32 {
    1
}

/// Health check response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub backends: BackendHealthSummary,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackendHealthSummary {
    pub total: usize,
    pub healthy: usize,
    pub unhealthy: usize,
}

/// Generic success response
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

