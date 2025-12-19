//! Application State

use std::sync::Arc;

use agent_core::{LlmProvider, ToolRegistry};
use agent_payments::{LicenseStore, MemoryLicenseStore, StripeClient};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    /// LLM provider (Ollama, etc.)
    pub provider: Arc<dyn LlmProvider>,
    
    /// Tool registry
    pub tools: Arc<ToolRegistry>,
    
    /// License store
    pub license_store: Arc<MemoryLicenseStore>,
    
    /// Stripe client (optional)
    pub stripe: Option<Arc<StripeClient>>,
}
