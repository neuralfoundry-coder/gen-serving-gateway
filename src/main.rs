//! Main entry point for the Generative Image Serving Framework

use generative_img_serving::{
    api,
    backend::registry::BackendRegistry,
    config::Settings,
    gateway::{health_check::HealthCheckManager, load_balancer::LoadBalancer},
    queue::request_queue::RequestQueue,
    AppState,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().json())
        .init();

    info!("Starting Generative Image Serving Framework");

    // Load configuration
    let settings = Settings::load()?;
    info!(
        "Loaded configuration: server={}:{}",
        settings.server.host, settings.server.port
    );

    let settings = Arc::new(RwLock::new(settings));
    
    // Initialize backend registry
    let backend_registry = Arc::new(BackendRegistry::new());
    
    // Register backends from configuration
    {
        let config = settings.read().await;
        backend_registry.initialize_from_config(&config.backends).await?;
    }
    
    // Initialize load balancer
    let load_balancer = Arc::new(LoadBalancer::new(backend_registry.clone()));
    
    // Initialize health check manager
    let health_manager = Arc::new(HealthCheckManager::new(backend_registry.clone()));
    
    // Start health check background task
    {
        let config = settings.read().await;
        health_manager.start(config.backends.iter()
            .map(|b| b.health_check_interval_secs)
            .min()
            .unwrap_or(30))
            .await;
    }
    
    // Initialize request queue
    let request_queue = Arc::new(RequestQueue::new(load_balancer.clone()));
    
    // Create application state
    let app_state = Arc::new(AppState {
        settings: settings.clone(),
        backend_registry,
        load_balancer,
        health_manager,
        request_queue,
    });

    // Build the router
    let app = api::routes::create_router(app_state.clone()).await;

    // Get server address
    let addr = {
        let config = settings.read().await;
        format!("{}:{}", config.server.host, config.server.port)
    };
    
    info!("Server listening on {}", addr);
    
    // Start the server
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

