//! Dynamic batch processor for grouping requests

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{oneshot, Mutex};
use tracing::{debug, info};

use crate::backend::traits::{GenerateRequest, GenerateResponse, GeneratedImage, ImageBackend};
use crate::error::Result;

/// Configuration for the batch processor
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum batch size
    pub max_batch_size: usize,
    /// Maximum wait time before processing a partial batch (milliseconds)
    pub max_wait_ms: u64,
    /// Whether batching is enabled
    pub enabled: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 4,
            max_wait_ms: 100,
            enabled: true,
        }
    }
}

/// A request waiting to be batched
struct BatchedRequest {
    request: GenerateRequest,
    response_tx: oneshot::Sender<Result<GenerateResponse>>,
}

/// Batch processor for grouping multiple requests
pub struct Batcher {
    config: BatchConfig,
    pending_requests: Arc<Mutex<Vec<BatchedRequest>>>,
    last_batch_time: Arc<Mutex<Instant>>,
}

impl Batcher {
    /// Create a new batcher with default configuration
    pub fn new() -> Self {
        Self::with_config(BatchConfig::default())
    }

    /// Create a new batcher with custom configuration
    pub fn with_config(config: BatchConfig) -> Self {
        Self {
            config,
            pending_requests: Arc::new(Mutex::new(Vec::new())),
            last_batch_time: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Add a request to the batch
    pub async fn add_request(&self, request: GenerateRequest) -> oneshot::Receiver<Result<GenerateResponse>> {
        let (response_tx, response_rx) = oneshot::channel();

        if !self.config.enabled {
            // If batching is disabled, return immediately
            // The caller will process the request directly
            return response_rx;
        }

        let batched = BatchedRequest {
            request,
            response_tx,
        };

        let mut pending = self.pending_requests.lock().await;
        pending.push(batched);

        response_rx
    }

    /// Check if the batch should be processed
    pub async fn should_process(&self) -> bool {
        let pending = self.pending_requests.lock().await;
        let last_time = self.last_batch_time.lock().await;

        if pending.is_empty() {
            return false;
        }

        // Process if batch is full
        if pending.len() >= self.config.max_batch_size {
            return true;
        }

        // Process if max wait time exceeded
        if last_time.elapsed() >= Duration::from_millis(self.config.max_wait_ms) {
            return true;
        }

        false
    }

    /// Process the current batch
    pub async fn process_batch<B: ImageBackend + ?Sized>(&self, backend: &B) -> Result<()> {
        let mut pending = self.pending_requests.lock().await;
        
        if pending.is_empty() {
            return Ok(());
        }

        let batch: Vec<BatchedRequest> = pending.drain(..).collect();
        drop(pending); // Release lock early

        // Update last batch time
        *self.last_batch_time.lock().await = Instant::now();

        let batch_size = batch.len();
        debug!(batch_size = batch_size, "Processing batch");

        // For now, process each request individually
        // A more sophisticated implementation could combine requests
        // for backends that support true batching
        for batched in batch {
            let result = backend.generate(batched.request).await;
            let _ = batched.response_tx.send(result);
        }

        info!(batch_size = batch_size, "Batch processed");
        Ok(())
    }

    /// Get the number of pending requests
    pub async fn pending_count(&self) -> usize {
        self.pending_requests.lock().await.len()
    }

    /// Create a combined request from multiple requests (for backends that support batching)
    #[allow(dead_code)]
    fn combine_requests(requests: &[GenerateRequest]) -> GenerateRequest {
        // Take the first request as the base
        // Sum up the number of images to generate
        let total_n: u32 = requests.iter().map(|r| r.n).sum();
        
        let mut combined = requests[0].clone();
        combined.n = total_n;
        combined
    }

    /// Split a batch response into individual responses
    #[allow(dead_code)]
    fn split_response(
        response: GenerateResponse,
        original_requests: &[GenerateRequest],
    ) -> Vec<GenerateResponse> {
        let mut results = Vec::new();
        let mut image_index = 0;

        for request in original_requests {
            let n = request.n as usize;
            let images: Vec<GeneratedImage> = response
                .images
                .iter()
                .skip(image_index)
                .take(n)
                .cloned()
                .collect();

            results.push(GenerateResponse {
                images,
                model: response.model.clone(),
            });

            image_index += n;
        }

        results
    }
}

impl Default for Batcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Batch processor that runs as a background task
pub struct BatchProcessor {
    batcher: Arc<Batcher>,
    backend: Arc<dyn ImageBackend>,
}

impl BatchProcessor {
    /// Create a new batch processor
    pub fn new(batcher: Arc<Batcher>, backend: Arc<dyn ImageBackend>) -> Self {
        Self { batcher, backend }
    }

    /// Start the batch processing loop
    pub async fn run(&self) {
        let interval = Duration::from_millis(10); // Check every 10ms

        loop {
            if self.batcher.should_process().await {
                if let Err(e) = self.batcher.process_batch(self.backend.as_ref()).await {
                    tracing::error!(error = %e, "Batch processing failed");
                }
            }

            tokio::time::sleep(interval).await;
        }
    }
}

