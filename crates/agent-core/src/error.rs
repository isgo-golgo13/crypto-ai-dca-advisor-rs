//! Error Types

use thiserror::Error;

/// Result type alias for agent operations
pub type Result<T> = std::result::Result<T, AgentError>;

/// Agent error types
#[derive(Error, Debug)]
pub enum AgentError {
    /// LLM provider error
    #[error("Provider error: {0}")]
    Provider(String),
    
    /// Provider unavailable or not responding
    #[error("Provider unavailable: {0}")]
    ProviderUnavailable(String),
    
    /// Tool not found in registry
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    
    /// Tool validation failed
    #[error("Tool validation error: {0}")]
    ToolValidation(String),
    
    /// Tool execution failed
    #[error("Tool execution error: {0}")]
    ToolExecution(String),
    
    /// Maximum iterations reached in reasoning loop
    #[error("Maximum iterations ({0}) reached")]
    MaxIterations(usize),
    
    /// Context length exceeded
    #[error("Context length exceeded: {used} tokens (max: {max})")]
    ContextOverflow { used: u32, max: u32 },
    
    /// Parse error (e.g., tool call parsing)
    #[error("Parse error: {0}")]
    Parse(String),
    
    /// Session error
    #[error("Session error: {0}")]
    Session(String),
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
    
    /// Rate limited
    #[error("Rate limited: {0}")]
    RateLimited(String),
    
    /// Authentication failed
    #[error("Authentication failed: {0}")]
    Auth(String),
    
    /// Generic IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    /// Other/unknown error
    #[error("{0}")]
    Other(String),
}

impl AgentError {
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            AgentError::ProviderUnavailable(_)
                | AgentError::RateLimited(_)
                | AgentError::Io(_)
        )
    }
    
    /// Convert to a user-friendly message
    pub fn user_message(&self) -> String {
        match self {
            AgentError::Provider(msg) => format!("The AI service encountered an error: {}", msg),
            AgentError::ProviderUnavailable(_) => "The AI service is currently unavailable. Please try again.".into(),
            AgentError::ToolNotFound(name) => format!("The tool '{}' is not available.", name),
            AgentError::ToolValidation(msg) => format!("Invalid tool input: {}", msg),
            AgentError::ToolExecution(msg) => format!("Tool error: {}", msg),
            AgentError::MaxIterations(_) => "The request took too long to process. Please try a simpler query.".into(),
            AgentError::ContextOverflow { .. } => "The conversation is too long. Please start a new session.".into(),
            AgentError::RateLimited(_) => "You've made too many requests. Please wait a moment.".into(),
            AgentError::Auth(_) => "Authentication failed. Please check your credentials.".into(),
            _ => "An unexpected error occurred.".into(),
        }
    }
}

impl From<anyhow::Error> for AgentError {
    fn from(err: anyhow::Error) -> Self {
        AgentError::Other(err.to_string())
    }
}
