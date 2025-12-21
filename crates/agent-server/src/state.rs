//! Application State

use std::sync::Arc;

use agent_core::{LlmProvider, ToolRegistry};
use agent_payments::{MemoryLicenseStore, StripeClient};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// LLM provider (Ollama, etc.)
    pub provider: Arc<dyn LlmProvider>,
    
    /// Tool registry with all available tools
    pub tools: Arc<ToolRegistry>,
    
    /// License store for subscription management
    pub license_store: Arc<MemoryLicenseStore>,
    
    /// Stripe client (optional - None if not configured)
    pub stripe: Option<Arc<StripeClient>>,
}
