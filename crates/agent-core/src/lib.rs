//! # agent-core
//!
//! Core agent logic with provider-agnostic LLM abstraction and extensible tool system.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                        Agent                                 │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
//! │  │  Reasoning  │  │    Tools    │  │   LlmProvider       │  │
//! │  │    Loop     │──│   Registry  │──│   (Strategy)        │  │
//! │  └─────────────┘  └─────────────┘  └─────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! The `LlmProvider` trait enables swapping between Ollama, OpenAI, Anthropic,
//! or any other provider without changing agent logic.

pub mod provider;
pub mod tool;
pub mod reasoning;
pub mod message;
pub mod error;
pub mod session;

pub use error::{AgentError, Result};
pub use message::{Message, Role};
pub use provider::LlmProvider;
pub use reasoning::Agent;
pub use session::Session;
pub use tool::{Tool, ToolCall, ToolResult, ToolRegistry};
