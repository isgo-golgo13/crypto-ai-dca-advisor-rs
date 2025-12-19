//! # agent-runtime
//!
//! Runtime providers for the rust-agent system.
//!
//! ## Providers
//!
//! - **Ollama** (default): Local LLM inference via Ollama
//! - **OpenAI** (coming soon): OpenAI API integration
//! - **Anthropic** (coming soon): Claude API integration
//!
//! ## Usage
//!
//! ```rust,ignore
//! use agent_runtime::ollama::OllamaProvider;
//!
//! let provider = OllamaProvider::new("http://localhost", 11434);
//! let agent = AgentBuilder::new()
//!     .provider(Arc::new(provider))
//!     .build()?;
//! ```

#[cfg(feature = "ollama")]
pub mod ollama;

#[cfg(feature = "ollama")]
pub use ollama::OllamaProvider;

// Re-export core types for convenience
pub use agent_core::{
    Agent, AgentError, LlmProvider, Message, Result, Role, Session, Tool, ToolRegistry,
};
