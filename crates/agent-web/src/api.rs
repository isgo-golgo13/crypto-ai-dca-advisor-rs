//! API Client

use serde::{Deserialize, Serialize};

/// Chat message for display
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Send a chat message to the backend
pub async fn send_chat(message: &str, license_key: Option<&str>) -> Result<String, String> {
    let client = reqwest::Client::new();
    
    let mut body = serde_json::json!({
        "message": message,
    });
    
    if let Some(key) = license_key {
        body["license_key"] = serde_json::json!(key);
    }

    let response = client
        .post("/api/chat")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        Ok(data["message"].as_str().unwrap_or("No response").to_string())
    } else {
        let data: serde_json::Value = response.json().await.unwrap_or_default();
        Err(data["error"].as_str().unwrap_or("Request failed").to_string())
    }
}

/// Create a Stripe checkout session
pub async fn create_checkout(plan: &str) -> Result<String, String> {
    let client = reqwest::Client::new();
    
    let origin = web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_else(|| "http://localhost:3000".into());

    let body = serde_json::json!({
        "plan": plan,
        "email": "", // Would get from form
        "success_url": format!("{}/chat?success=true", origin),
        "cancel_url": format!("{}/pricing", origin),
    });

    let response = client
        .post("/api/checkout")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        Ok(data["checkout_url"].as_str().unwrap_or("").to_string())
    } else {
        Err("Failed to create checkout".into())
    }
}
