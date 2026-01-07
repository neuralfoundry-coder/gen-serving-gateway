//! gRPC backend client implementation

use async_trait::async_trait;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;
use tonic::transport::{Channel, Endpoint};
use tracing::{debug, warn};

use crate::backend::traits::{
    BackendEndpoint, GenerateRequest, GenerateResponse, ImageBackend,
};
use crate::config::BackendConfig;
use crate::error::{AppError, Result};

/// gRPC-based image generation backend
pub struct GrpcBackend {
    name: String,
    endpoints: Arc<RwLock<Vec<BackendEndpoint>>>,
    channels: Arc<RwLock<Vec<Option<Channel>>>>,
    timeout_ms: u64,
    weight: u32,
    enabled: bool,
    current_endpoint_index: Arc<RwLock<usize>>,
}

impl GrpcBackend {
    /// Create a new gRPC backend from configuration
    pub async fn new(config: &BackendConfig) -> Result<Self> {
        let endpoints: Vec<BackendEndpoint> = config
            .endpoints
            .iter()
            .map(|url| BackendEndpoint::new(url.clone()))
            .collect();

        let channels: Vec<Option<Channel>> = vec![None; endpoints.len()];

        Ok(Self {
            name: config.name.clone(),
            endpoints: Arc::new(RwLock::new(endpoints)),
            channels: Arc::new(RwLock::new(channels)),
            timeout_ms: config.timeout_ms,
            weight: config.weight,
            enabled: config.enabled,
            current_endpoint_index: Arc::new(RwLock::new(0)),
        })
    }

    /// Get or create a channel to an endpoint
    async fn get_channel(&self, index: usize) -> Result<Channel> {
        // Check if we already have a channel
        {
            let channels = self.channels.read();
            if let Some(Some(channel)) = channels.get(index) {
                return Ok(channel.clone());
            }
        }

        // Create a new channel
        let endpoint_url = {
            let endpoints = self.endpoints.read();
            endpoints
                .get(index)
                .map(|e| e.url.clone())
                .ok_or_else(|| AppError::Internal("Invalid endpoint index".to_string()))?
        };

        let endpoint = Endpoint::from_shared(endpoint_url.clone())
            .map_err(|e| AppError::Internal(format!("Invalid endpoint URL: {}", e)))?
            .timeout(Duration::from_millis(self.timeout_ms))
            .connect_timeout(Duration::from_secs(10));

        let channel = endpoint
            .connect()
            .await
            .map_err(|e| AppError::Grpc(tonic::Status::unavailable(format!("Connection failed: {}", e))))?;

        // Store the channel
        {
            let mut channels = self.channels.write();
            if let Some(slot) = channels.get_mut(index) {
                *slot = Some(channel.clone());
            }
        }

        Ok(channel)
    }

    /// Get the next healthy endpoint index
    fn get_next_healthy_index(&self) -> Option<usize> {
        let endpoints = self.endpoints.read();
        let healthy_indices: Vec<usize> = endpoints
            .iter()
            .enumerate()
            .filter(|(_, e)| e.healthy)
            .map(|(i, _)| i)
            .collect();

        if healthy_indices.is_empty() {
            return None;
        }

        let mut index = self.current_endpoint_index.write();
        *index = (*index + 1) % healthy_indices.len();
        Some(healthy_indices[*index])
    }

    /// Mark an endpoint as unhealthy
    fn mark_endpoint_unhealthy(&self, index: usize) {
        let mut endpoints = self.endpoints.write();
        if let Some(endpoint) = endpoints.get_mut(index) {
            endpoint.mark_unhealthy();
            warn!(backend = %self.name, url = %endpoint.url, "Marked gRPC endpoint as unhealthy");
        }

        // Clear the channel so it will be recreated
        let mut channels = self.channels.write();
        if let Some(slot) = channels.get_mut(index) {
            *slot = None;
        }
    }

    /// Mark an endpoint as healthy
    fn mark_endpoint_healthy(&self, index: usize) {
        let mut endpoints = self.endpoints.write();
        if let Some(endpoint) = endpoints.get_mut(index) {
            endpoint.mark_healthy();
            debug!(backend = %self.name, url = %endpoint.url, "Marked gRPC endpoint as healthy");
        }
    }
}

#[async_trait]
impl ImageBackend for GrpcBackend {
    fn name(&self) -> &str {
        &self.name
    }

    fn protocol(&self) -> &str {
        "grpc"
    }

    fn endpoints(&self) -> Vec<String> {
        self.endpoints.read().iter().map(|e| e.url.clone()).collect()
    }

    async fn generate(&self, _request: GenerateRequest) -> Result<GenerateResponse> {
        let index = self
            .get_next_healthy_index()
            .ok_or_else(|| AppError::NoHealthyBackends(self.name.clone()))?;

        let _channel = self.get_channel(index).await?;

        debug!(backend = %self.name, "Sending gRPC generate request");

        // Create gRPC client and make request
        // For now, we'll simulate the gRPC call since we don't have a real gRPC server
        // In production, this would use the generated protobuf client
        
        // Simulated response for demonstration
        // TODO: Replace with actual gRPC client call when backend is available
        /*
        use crate::backend::proto::imagebackend::image_backend_service_client::ImageBackendServiceClient;
        use crate::backend::proto::imagebackend::GenerateRequest as ProtoRequest;
        
        let mut client = ImageBackendServiceClient::new(channel);
        
        let proto_request = ProtoRequest {
            prompt: request.prompt,
            negative_prompt: request.negative_prompt.unwrap_or_default(),
            n: request.n as i32,
            width: request.width as i32,
            height: request.height as i32,
            model: request.model.unwrap_or_default(),
            seed: request.seed.unwrap_or(-1),
            guidance_scale: request.guidance_scale.unwrap_or(7.5),
            num_inference_steps: request.num_inference_steps.unwrap_or(50) as i32,
            response_format: request.response_format,
            extra_params: String::new(),
        };
        
        let response = client.generate(proto_request).await?;
        */

        // For now, return an error indicating gRPC backend needs actual server
        self.mark_endpoint_healthy(index);
        
        // Placeholder response - in real implementation, this would come from gRPC server
        Err(AppError::BackendError(
            "gRPC backend requires a running gRPC server. Please configure an HTTP backend or start the gRPC server.".to_string()
        ))
    }

    async fn health_check(&self) -> bool {
        let endpoints_len = self.endpoints.read().len();
        let mut any_healthy = false;

        for index in 0..endpoints_len {
            match self.get_channel(index).await {
                Ok(_channel) => {
                    // TODO: Make actual gRPC health check call
                    // For now, just check if we can connect
                    self.mark_endpoint_healthy(index);
                    any_healthy = true;
                    debug!(
                        backend = %self.name,
                        index = index,
                        "gRPC health check passed (connection test)"
                    );
                }
                Err(e) => {
                    self.mark_endpoint_unhealthy(index);
                    debug!(
                        backend = %self.name,
                        index = index,
                        error = %e,
                        "gRPC health check failed"
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

