//! Asynchronous request queue for managing image generation requests

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Semaphore};
use tracing::debug;

use crate::backend::traits::{GenerateRequest, GenerateResponse};
use crate::error::{AppError, Result};
use crate::gateway::load_balancer::LoadBalancer;

/// Request with its response channel
struct QueuedRequest {
    request: GenerateRequest,
    backend_name: Option<String>,
    response_tx: oneshot::Sender<Result<GenerateResponse>>,
}

/// Configuration for the request queue
#[derive(Debug, Clone)]
pub struct QueueConfig {
    /// Maximum number of pending requests in the queue
    pub max_queue_size: usize,
    /// Maximum number of concurrent requests being processed
    pub max_concurrent: usize,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 1000,
            max_concurrent: 10,
            timeout_ms: 120000, // 2 minutes
        }
    }
}

/// Request queue for managing image generation requests
pub struct RequestQueue {
    #[allow(dead_code)]
    load_balancer: Arc<LoadBalancer>,
    request_tx: mpsc::Sender<QueuedRequest>,
    config: QueueConfig,
    pending_count: AtomicU64,
    processed_count: AtomicU64,
}

impl RequestQueue {
    /// Create a new request queue with default configuration
    pub fn new(load_balancer: Arc<LoadBalancer>) -> Self {
        Self::with_config(load_balancer, QueueConfig::default())
    }

    /// Create a new request queue with custom configuration
    pub fn with_config(load_balancer: Arc<LoadBalancer>, config: QueueConfig) -> Self {
        let (request_tx, request_rx) = mpsc::channel(config.max_queue_size);
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent));
        let lb = load_balancer.clone();
        let timeout_ms = config.timeout_ms;

        // Start the worker task
        tokio::spawn(async move {
            Self::process_requests(request_rx, lb, semaphore, timeout_ms).await;
        });

        Self {
            load_balancer,
            request_tx,
            config,
            pending_count: AtomicU64::new(0),
            processed_count: AtomicU64::new(0),
        }
    }

    /// Submit a request to the queue
    pub async fn submit(
        &self,
        request: GenerateRequest,
        backend_name: Option<&str>,
    ) -> Result<GenerateResponse> {
        // Check if queue is full
        let pending = self.pending_count.load(Ordering::Relaxed);
        if pending >= self.config.max_queue_size as u64 {
            return Err(AppError::Internal("Request queue is full".to_string()));
        }

        // Create response channel
        let (response_tx, response_rx) = oneshot::channel();

        let queued_request = QueuedRequest {
            request,
            backend_name: backend_name.map(String::from),
            response_tx,
        };

        // Increment pending count
        self.pending_count.fetch_add(1, Ordering::Relaxed);

        // Send to queue
        self.request_tx
            .send(queued_request)
            .await
            .map_err(|_| AppError::Internal("Failed to queue request".to_string()))?;

        debug!(pending = pending + 1, "Request queued");

        // Wait for response with timeout
        let timeout = Duration::from_millis(self.config.timeout_ms);
        match tokio::time::timeout(timeout, response_rx).await {
            Ok(Ok(result)) => {
                self.pending_count.fetch_sub(1, Ordering::Relaxed);
                result
            }
            Ok(Err(_)) => {
                self.pending_count.fetch_sub(1, Ordering::Relaxed);
                Err(AppError::Internal("Request processing was cancelled".to_string()))
            }
            Err(_) => {
                self.pending_count.fetch_sub(1, Ordering::Relaxed);
                Err(AppError::Timeout("Request timed out".to_string()))
            }
        }
    }

    /// Process requests from the queue
    async fn process_requests(
        mut request_rx: mpsc::Receiver<QueuedRequest>,
        load_balancer: Arc<LoadBalancer>,
        semaphore: Arc<Semaphore>,
        timeout_ms: u64,
    ) {
        while let Some(queued) = request_rx.recv().await {
            let lb = load_balancer.clone();
            let sem = semaphore.clone();
            let timeout = Duration::from_millis(timeout_ms);

            tokio::spawn(async move {
                // Acquire semaphore permit
                let _permit = match sem.acquire().await {
                    Ok(permit) => permit,
                    Err(_) => {
                        let _ = queued.response_tx.send(Err(AppError::Internal(
                            "Failed to acquire processing permit".to_string(),
                        )));
                        return;
                    }
                };

                // Select backend
                let backend = match lb
                    .select_backend(queued.backend_name.as_deref())
                    .await
                {
                    Ok(b) => b,
                    Err(e) => {
                        let _ = queued.response_tx.send(Err(e));
                        return;
                    }
                };

                debug!(backend = %backend.name(), "Processing request");

                // Generate images with timeout
                let result = tokio::time::timeout(timeout, backend.generate(queued.request)).await;

                let response = match result {
                    Ok(Ok(resp)) => Ok(resp),
                    Ok(Err(e)) => Err(e),
                    Err(_) => Err(AppError::Timeout(format!(
                        "Request to {} timed out",
                        backend.name()
                    ))),
                };

                // Send response
                let _ = queued.response_tx.send(response);
            });
        }
    }

    /// Get the number of pending requests
    pub fn pending_count(&self) -> u64 {
        self.pending_count.load(Ordering::Relaxed)
    }

    /// Get the number of processed requests
    pub fn processed_count(&self) -> u64 {
        self.processed_count.load(Ordering::Relaxed)
    }

    /// Get queue statistics
    pub fn stats(&self) -> QueueStats {
        QueueStats {
            pending: self.pending_count(),
            processed: self.processed_count(),
            max_queue_size: self.config.max_queue_size,
            max_concurrent: self.config.max_concurrent,
        }
    }
}

/// Queue statistics
#[derive(Debug, Clone)]
pub struct QueueStats {
    pub pending: u64,
    pub processed: u64,
    pub max_queue_size: usize,
    pub max_concurrent: usize,
}

