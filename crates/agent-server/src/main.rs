//! rust-agent HTTP Server
//!
//! Axum-based server providing REST API and WebSocket endpoints.
//! 
//! This version includes crypto-advisor tools for cryptocurrency
//! investment guidance with DCA and risk management.

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

// Import crypto-advisor tools
use crypto_advisor::{
    tools::{PriceLookupTool, DCACalculatorTool, RiskAnalyzerTool, PortfolioTrackerTool},
    exchange::MockExchangeClient,
};

use crate::handlers::{
    chat_handler, chat_stream_handler, create_checkout, health_check, 
    stripe_webhook, verify_license, list_models,
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
        Ok(true) => {
            tracing::info!("✓ Connected to Ollama");
            // List available models
            if let Ok(models) = provider.list_models().await {
                for model in models {
                    tracing::info!("  Model: {}", model.id);
                }
            }
        }
        Ok(false) | Err(_) => {
            tracing::warn!("⚠ Ollama not available - agent will fail");
            tracing::warn!("  Make sure Ollama is running: ollama serve");
        }
    }

    // Initialize exchange client for crypto tools
    let exchange: Arc<dyn crypto_advisor::exchange::ExchangeClient> = 
        Arc::new(MockExchangeClient::new());

    // Initialize tools
    let mut tools = ToolRegistry::new();
    
    // Core tools
    tools.register(DateTimeTool);
    tools.register(CalculatorTool);
    
    // Crypto advisor tools
    tools.register(PriceLookupTool::new(exchange.clone()));
    tools.register(DCACalculatorTool::new(exchange.clone()));
    tools.register(RiskAnalyzerTool::new(exchange.clone()));
    tools.register(PortfolioTrackerTool::new(exchange.clone()));
    
    tracing::info!("Registered {} tools:", tools.len());
    for name in tools.names() {
        tracing::info!("  • {}", name);
    }

    // Initialize payments
    let license_store = Arc::new(MemoryLicenseStore::new());
    let stripe = StripeClient::from_env().ok();
    
    if stripe.is_some() {
        tracing::info!("✓ Stripe configured");
    } else {
        tracing::warn!("⚠ Stripe not configured - payments disabled");
        tracing::warn!("  Set STRIPE_SECRET_KEY and STRIPE_WEBHOOK_SECRET in .env");
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
        // Health & info
        .route("/health", get(health_check))
        .route("/api/models", get(list_models))
        
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
    
    tracing::info!("══════════════════════════════════════════════════");
    tracing::info!("🚀 rust-agent server running on http://{}", addr);
    tracing::info!("══════════════════════════════════════════════════");
    tracing::info!("");
    tracing::info!("Endpoints:");
    tracing::info!("  GET  /health          - Health check");
    tracing::info!("  GET  /api/models      - List available models");
    tracing::info!("  POST /api/chat        - Send message");
    tracing::info!("  GET  /api/chat/stream - WebSocket streaming");
    tracing::info!("  POST /api/checkout    - Create Stripe checkout");
    tracing::info!("  POST /api/license/verify - Verify license key");
    tracing::info!("");
    
    axum::serve(listener, app).await?;
    
    Ok(())
}
