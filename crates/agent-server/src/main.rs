//! rust-agent HTTP Server
//!
//! Axum-based server providing REST API and WebSocket endpoints.

mod handlers;
mod state;

use std::sync::Arc;

use axum::{routing::{get, post}, Router};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use agent_core::tool::{CalculatorTool, DateTimeTool, ToolRegistry};
use agent_payments::{MemoryLicenseStore, StripeClient};
use agent_runtime::OllamaProvider;

use crate::handlers::{
    chat_handler, chat_stream_handler, create_checkout, health_check, 
    stripe_webhook, verify_license,
};
use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load environment
    dotenvy::dotenv().ok();

    // Initialize LLM provider
    let provider = Arc::new(OllamaProvider::from_env());
    
    // Verify Ollama connection
    match provider.health_check().await {
        Ok(true) => tracing::info!("✓ Connected to Ollama"),
        Ok(false) | Err(_) => tracing::warn!("⚠ Ollama not available - agent will fail"),
    }

    // Initialize tools
    let mut tools = ToolRegistry::new();
    tools.register(DateTimeTool);
    tools.register(CalculatorTool);
    // Add more tools here
    
    tracing::info!("Registered {} tools: {:?}", tools.len(), tools.names());

    // Initialize payments
    let license_store = Arc::new(MemoryLicenseStore::new());
    let stripe = StripeClient::from_env().ok();
    
    if stripe.is_some() {
        tracing::info!("✓ Stripe configured");
    } else {
        tracing::warn!("⚠ Stripe not configured - payments disabled");
    }

    // Build application state
    let state = AppState {
        provider,
        tools: Arc::new(tools),
        license_store,
        stripe: stripe.map(Arc::new),
    };

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        // Health
        .route("/health", get(health_check))
        
        // Agent API
        .route("/api/chat", post(chat_handler))
        .route("/api/chat/stream", get(chat_stream_handler))
        
        // Payments
        .route("/api/checkout", post(create_checkout))
        .route("/api/license/verify", post(verify_license))
        .route("/webhook/stripe", post(stripe_webhook))
        
        // Static files (WASM frontend)
        .nest_service("/", tower_http::services::ServeDir::new("static"))
        
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".into());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    tracing::info!("🚀 rust-agent server running on http://{}", addr);
    
    axum::serve(listener, app).await?;
    
    Ok(())
}
