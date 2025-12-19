//! HTTP/WebSocket Handlers

use axum::{
    extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use agent_core::{
    message::Conversation,
    provider::GenerationOptions,
    reasoning::{Agent, AgentConfig},
};
use agent_payments::{
    CheckoutRequest as PaymentCheckoutRequest, LicenseKey, LicenseStore,
    LicenseVerification, Plan, WebhookHandler,
};

use crate::state::AppState;

// ============================================================================
// Response Types
// ============================================================================

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub ollama_connected: bool,
    pub stripe_configured: bool,
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    #[serde(default)]
    pub license_key: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub conversation_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub message: String,
    pub conversation_id: String,
    pub model: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutRequest {
    pub plan: String,
    pub email: String,
    pub success_url: String,
    pub cancel_url: String,
}

#[derive(Debug, Serialize)]
pub struct CheckoutResponse {
    pub checkout_url: String,
    pub session_id: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyLicenseRequest {
    pub license_key: String,
}

// ============================================================================
// Handlers
// ============================================================================

/// Health check endpoint
pub async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    let ollama_connected = state.provider.health_check().await.unwrap_or(false);
    
    Json(HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
        ollama_connected,
        stripe_configured: state.stripe.is_some(),
    })
}

/// Main chat endpoint (non-streaming)
pub async fn chat_handler(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Verify license if provided
    if let Some(ref key) = payload.license_key {
        let license_key = LicenseKey::from_string(key);
        match state.license_store.verify_and_use(&license_key) {
            Ok(verification) if !verification.valid => {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse {
                        error: verification.message.unwrap_or_else(|| "Invalid license".into()),
                        code: "INVALID_LICENSE".into(),
                    }),
                ));
            }
            Err(e) => {
                tracing::warn!("License verification error: {}", e);
            }
            _ => {}
        }
    }

    // Get model
    let model = payload.model.clone().unwrap_or_else(|| "llama3.2".into());
    
    // Create agent
    let config = AgentConfig {
        generation: GenerationOptions {
            model: model.clone(),
            ..Default::default()
        },
        ..Default::default()
    };
    
    let agent = Agent::new(
        state.provider.clone(),
        state.tools.clone(),
        config,
    );
    
    // Run agent
    let response = agent.ask(&payload.message).await.map_err(|e| {
        tracing::error!("Agent error: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.user_message(),
                code: "AGENT_ERROR".into(),
            }),
        )
    })?;

    let conversation_id = payload.conversation_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    Ok(Json(ChatResponse {
        message: response,
        conversation_id,
        model,
    }))
}

/// WebSocket streaming chat
pub async fn chat_stream_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(move |socket| handle_stream(socket, state))
}

async fn handle_stream(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    
    while let Some(msg) = receiver.next().await {
        let msg = match msg {
            Ok(Message::Text(text)) => text,
            Ok(Message::Close(_)) => break,
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            }
            _ => continue,
        };

        // Parse request
        let request: ChatRequest = match serde_json::from_str(&msg) {
            Ok(r) => r,
            Err(e) => {
                let error = serde_json::json!({"type": "error", "error": e.to_string()});
                let _ = sender.send(Message::Text(error.to_string().into())).await;
                continue;
            }
        };

        let model = request.model.unwrap_or_else(|| "llama3.2".into());
        let messages = vec![
            agent_core::Message::system("You are a helpful assistant."),
            agent_core::Message::user(request.message),
        ];

        let options = GenerationOptions {
            model: model.clone(),
            ..Default::default()
        };

        // Stream response
        match state.provider.complete_stream(&messages, &options).await {
            Ok(mut stream) => {
                while let Some(result) = stream.next().await {
                    match result {
                        Ok(chunk) => {
                            let response = serde_json::json!({
                                "type": "chunk",
                                "content": chunk.delta,
                                "done": chunk.done,
                            });
                            if sender.send(Message::Text(response.to_string().into())).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            let error = serde_json::json!({"type": "error", "error": e.to_string()});
                            let _ = sender.send(Message::Text(error.to_string().into())).await;
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                let error = serde_json::json!({"type": "error", "error": e.to_string()});
                let _ = sender.send(Message::Text(error.to_string().into())).await;
            }
        }
    }
}

/// Create Stripe checkout session
pub async fn create_checkout(
    State(state): State<AppState>,
    Json(payload): Json<CheckoutRequest>,
) -> Result<Json<CheckoutResponse>, (StatusCode, Json<ErrorResponse>)> {
    let stripe = state.stripe.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "Payments not configured".into(),
                code: "PAYMENTS_DISABLED".into(),
            }),
        )
    })?;

    let plan = Plan::from_str(&payload.plan);
    
    let request = PaymentCheckoutRequest {
        plan,
        customer_email: payload.email,
        success_url: payload.success_url,
        cancel_url: payload.cancel_url,
        user_id: None,
    };

    let session = stripe.create_checkout_session(request).await.map_err(|e| {
        tracing::error!("Checkout error: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.user_message().into(),
                code: "CHECKOUT_ERROR".into(),
            }),
        )
    })?;

    Ok(Json(CheckoutResponse {
        checkout_url: session.checkout_url,
        session_id: session.id,
    }))
}

/// Verify license key
pub async fn verify_license(
    State(state): State<AppState>,
    Json(payload): Json<VerifyLicenseRequest>,
) -> Json<LicenseVerification> {
    let key = LicenseKey::from_string(&payload.license_key);
    
    match state.license_store.get(&key) {
        Ok(Some(license)) if license.is_valid() => {
            let remaining = license.plan.rate_limit().saturating_sub(license.usage_today);
            Json(LicenseVerification::valid(license.plan, remaining))
        }
        _ => Json(LicenseVerification::invalid("License not found or invalid")),
    }
}

/// Stripe webhook handler
pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let stripe = state.stripe.as_ref().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ErrorResponse {
                error: "Payments not configured".into(),
                code: "PAYMENTS_DISABLED".into(),
            }),
        )
    })?;

    let signature = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Missing Stripe signature".into(),
                    code: "MISSING_SIGNATURE".into(),
                }),
            )
        })?;

    let handler = WebhookHandler::new(state.license_store.clone());
    
    let event = handler.parse_event(&body, signature, stripe.webhook_secret())
        .map_err(|e| {
            tracing::warn!("Webhook signature failed: {}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "Invalid signature".into(),
                    code: "INVALID_SIGNATURE".into(),
                }),
            )
        })?;

    handler.handle(event).await.map_err(|e| {
        tracing::error!("Webhook processing error: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: "Webhook processing failed".into(),
                code: "WEBHOOK_ERROR".into(),
            }),
        )
    })?;

    Ok(StatusCode::OK)
}
