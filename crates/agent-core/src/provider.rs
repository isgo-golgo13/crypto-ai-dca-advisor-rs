//! LLM Provider Strategy Pattern
//!
//! Defines a common interface for all LLM providers (Ollama, OpenAI, Anthropic, etc.)
//! allowing the agent to work with any backend without code changes.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use agent_core::provider::{LlmProvider, ProviderConfig};
//!
//! // Create a provider
//! let provider = OllamaProvider::new(config);
//!
//! // Use through the trait
//! let response = provider.complete(messages, options).await?;
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use futures::Stream;

use crate::error::Result;
use crate::message::Message;

/// Configuration for LLM generation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenerationOptions {
    /// Model identifier (e.g., "llama3.2", "gpt-4", "claude-3-sonnet")
    pub model: String,
    
    /// Temperature for sampling (0.0 = deterministic, 1.0 = creative)
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    
    /// Maximum tokens to generate
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    
    /// Top-p nucleus sampling
    #[serde(default = "default_top_p")]
    pub top_p: f32,
    
    /// Stop sequences
    #[serde(default)]
    pub stop_sequences: Vec<String>,
    
    /// System prompt override (if provider supports it separately)
    #[serde(default)]
    pub system_prompt: Option<String>,
}

fn default_temperature() -> f32 { 0.7 }
fn default_max_tokens() -> u32 { 2048 }
fn default_top_p() -> f32 { 0.9 }

impl Default for GenerationOptions {
    fn default() -> Self {
        Self {
            model: "llama3.2".into(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            top_p: default_top_p(),
            stop_sequences: Vec::new(),
            system_prompt: None,
        }
    }
}

/// Response from an LLM completion
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Completion {
    /// The generated text
    pub content: String,
    
    /// Model that generated this response
    pub model: String,
    
    /// Token usage statistics (if available)
    pub usage: Option<TokenUsage>,
    
    /// Whether the response was truncated
    pub truncated: bool,
    
    /// Finish reason
    pub finish_reason: Option<FinishReason>,
}

/// Token usage statistics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Reason for completion finishing
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolUse,
    ContentFilter,
    Error,
}

/// A chunk from streaming completion
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamChunk {
    /// The text delta
    pub delta: String,
    
    /// Whether this is the final chunk
    pub done: bool,
    
    /// Token usage (typically only on final chunk)
    pub usage: Option<TokenUsage>,
}

/// Stream type for completion streaming
pub type CompletionStream = Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>;

/// Provider metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Provider name (e.g., "Ollama", "OpenAI")
    pub name: String,
    
    /// Provider version
    pub version: Option<String>,
    
    /// Available models
    pub models: Vec<ModelInfo>,
    
    /// Whether streaming is supported
    pub supports_streaming: bool,
    
    /// Whether tool/function calling is supported
    pub supports_tools: bool,
}

/// Information about a model
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub context_length: Option<u32>,
    pub supports_vision: bool,
}

/// Strategy trait for LLM providers
///
/// Implement this trait to add support for new LLM backends.
/// The agent works exclusively through this interface.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Get provider information and capabilities
    async fn info(&self) -> Result<ProviderInfo>;
    
    /// Check if the provider is available and configured correctly
    async fn health_check(&self) -> Result<bool>;
    
    /// Generate a completion from messages
    async fn complete(
        &self,
        messages: &[Message],
        options: &GenerationOptions,
    ) -> Result<Completion>;
    
    /// Generate a streaming completion
    async fn complete_stream(
        &self,
        messages: &[Message],
        options: &GenerationOptions,
    ) -> Result<CompletionStream>;
    
    /// List available models
    async fn list_models(&self) -> Result<Vec<ModelInfo>>;
    
    /// Estimate token count for text (provider-specific tokenization)
    fn estimate_tokens(&self, text: &str) -> u32 {
        // Default: rough estimate of ~4 chars per token
        (text.len() / 4) as u32
    }
}

/// Provider selection strategy
/// 
/// Enables automatic failover or load balancing across providers
#[derive(Clone, Debug)]
pub enum ProviderStrategy {
    /// Use a single provider
    Single,
    
    /// Failover to next provider on error
    Failover,
    
    /// Round-robin load balancing
    RoundRobin,
    
    /// Route based on model name
    ModelRouted,
}

/// Multi-provider wrapper with failover support
pub struct ProviderChain {
    providers: Vec<Box<dyn LlmProvider>>,
    strategy: ProviderStrategy,
    current_index: std::sync::atomic::AtomicUsize,
}

impl ProviderChain {
    pub fn new(providers: Vec<Box<dyn LlmProvider>>, strategy: ProviderStrategy) -> Self {
        Self {
            providers,
            strategy,
            current_index: std::sync::atomic::AtomicUsize::new(0),
        }
    }
    
    /// Get the next provider based on strategy
    pub fn next_provider(&self) -> Option<&dyn LlmProvider> {
        if self.providers.is_empty() {
            return None;
        }
        
        match self.strategy {
            ProviderStrategy::Single => self.providers.first().map(|p| p.as_ref()),
            ProviderStrategy::RoundRobin => {
                let idx = self.current_index.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let idx = idx % self.providers.len();
                Some(self.providers[idx].as_ref())
            }
            ProviderStrategy::Failover | ProviderStrategy::ModelRouted => {
                // Start from current, will advance on failure
                let idx = self.current_index.load(std::sync::atomic::Ordering::SeqCst);
                let idx = idx % self.providers.len();
                Some(self.providers[idx].as_ref())
            }
        }
    }
    
    /// Advance to next provider (for failover)
    pub fn advance(&self) {
        self.current_index.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generation_options_defaults() {
        let opts = GenerationOptions::default();
        assert_eq!(opts.temperature, 0.7);
        assert_eq!(opts.max_tokens, 2048);
        assert_eq!(opts.model, "llama3.2");
    }
}
