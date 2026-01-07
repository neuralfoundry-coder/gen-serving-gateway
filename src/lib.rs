//! Generative Image Serving Framework
//!
//! A Rust-based framework for serving multiple generative image model backends
//! through a unified gateway with load balancing, health checking, and more.

pub mod api;
pub mod backend;
pub mod config;
pub mod error;
pub mod gateway;
pub mod middleware;
pub mod queue;
pub mod response;

pub use error::{AppError, Result};

use std::sync::Arc;
use tokio::sync::RwLock;

use backend::registry::BackendRegistry;
use gateway::{health_check::HealthCheckManager, load_balancer::LoadBalancer};
use queue::request_queue::RequestQueue;

/// Application state shared across all handlers
pub struct AppState {
    pub settings: Arc<RwLock<config::Settings>>,
    pub backend_registry: Arc<BackendRegistry>,
    pub load_balancer: Arc<LoadBalancer>,
    pub health_manager: Arc<HealthCheckManager>,
    pub request_queue: Arc<RequestQueue>,
}

