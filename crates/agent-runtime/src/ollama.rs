//! Ollama LLM Provider
//!
//! Implementation of `LlmProvider` for local Ollama inference.

use std::pin::Pin;

use agent_core::{
    error::{AgentError, Result},
    message::{Message, Role},
    provider::{
        Completion, CompletionStream, FinishReason, GenerationOptions, LlmProvider,
        ModelInfo, ProviderInfo, StreamChunk, TokenUsage,
    },
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use ollama_rs::{
    generation::{
        chat::{ChatMessage, ChatMessageResponse, MessageRole, request::ChatMessageRequest},
        options::GenerationOptions as OllamaOptions,
    },
    Ollama,
};

/// Ollama provider configuration
#[derive(Clone, Debug)]
pub struct OllamaConfig {
    /// Ollama host URL
    pub host: String,
    
    /// Ollama port
    pub port: u16,
    
    /// Connection timeout in seconds
    pub timeout_secs: u64,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            host: "http://localhost".into(),
            port: 11434,
            timeout_secs: 120,
        }
    }
}

impl OllamaConfig {
    pub fn from_env() -> Self {
        let host = std::env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| "http://localhost".into());
        let port = std::env::var("OLLAMA_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(11434);
        
        Self {
            host,
            port,
            ..Default::default()
        }
    }
}

/// Ollama LLM provider
pub struct OllamaProvider {
    client: Ollama,
    config: OllamaConfig,
}

impl OllamaProvider {
    /// Create a new Ollama provider with custom host/port
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        let config = OllamaConfig {
            host: host.into(),
            port,
            ..Default::default()
        };
        
        Self {
            client: Ollama::new(&config.host, config.port),
            config,
        }
    }
    
    /// Create from configuration
    pub fn from_config(config: OllamaConfig) -> Self {
        Self {
            client: Ollama::new(&config.host, config.port),
            config,
        }
    }
    
    /// Create from environment variables
    pub fn from_env() -> Self {
        Self::from_config(OllamaConfig::from_env())
    }
    
    /// Create with default localhost settings
    pub fn localhost() -> Self {
        Self::from_config(OllamaConfig::default())
    }
    
    /// Convert agent messages to Ollama format
    fn convert_messages(messages: &[Message]) -> Vec<ChatMessage> {
        messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    Role::System => MessageRole::System,
                    Role::User => MessageRole::User,
                    Role::Assistant => MessageRole::Assistant,
                    Role::Tool => MessageRole::User, // Tools appear as user context
                };
                ChatMessage::new(role, m.content.clone())
            })
            .collect()
    }
    
    /// Convert Ollama response to agent completion
    fn convert_completion(response: ChatMessageResponse, model: &str) -> Completion {
        Completion {
            content: response.message.content,
            model: model.to_string(),
            usage: response.final_data.as_ref().map(|d| TokenUsage {
                prompt_tokens: d.prompt_eval_count.unwrap_or(0) as u32,
                completion_tokens: d.eval_count.unwrap_or(0) as u32,
                total_tokens: (d.prompt_eval_count.unwrap_or(0) + d.eval_count.unwrap_or(0)) as u32,
            }),
            truncated: false,
            finish_reason: Some(FinishReason::Stop),
        }
    }
    
    /// Build Ollama generation options
    fn build_options(opts: &GenerationOptions) -> OllamaOptions {
        OllamaOptions::default()
            .temperature(opts.temperature)
            .top_p(opts.top_p)
            .num_predict(opts.max_tokens as i32)
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn info(&self) -> Result<ProviderInfo> {
        let models = self.list_models().await.unwrap_or_default();
        
        Ok(ProviderInfo {
            name: "Ollama".into(),
            version: None, // Ollama API doesn't expose version
            models,
            supports_streaming: true,
            supports_tools: false, // Native tool calling not yet in ollama-rs
        })
    }
    
    async fn health_check(&self) -> Result<bool> {
        match self.client.list_local_models().await {
            Ok(_) => Ok(true),
            Err(e) => {
                tracing::warn!("Ollama health check failed: {}", e);
                Ok(false)
            }
        }
    }
    
    async fn complete(
        &self,
        messages: &[Message],
        options: &GenerationOptions,
    ) -> Result<Completion> {
        let ollama_messages = Self::convert_messages(messages);
        let ollama_options = Self::build_options(options);
        
        let request = ChatMessageRequest::new(
            options.model.clone(),
            ollama_messages,
        ).options(ollama_options);
        
        let response = self.client
            .send_chat_messages(request)
            .await
            .map_err(|e| AgentError::Provider(e.to_string()))?;
        
        Ok(Self::convert_completion(response, &options.model))
    }
    
    async fn complete_stream(
        &self,
        messages: &[Message],
        options: &GenerationOptions,
    ) -> Result<CompletionStream> {
        let ollama_messages = Self::convert_messages(messages);
        let ollama_options = Self::build_options(options);
        
        let request = ChatMessageRequest::new(
            options.model.clone(),
            ollama_messages,
        ).options(ollama_options);
        
        let stream = self.client
            .send_chat_messages_stream(request)
            .await
            .map_err(|e| AgentError::Provider(e.to_string()))?;
        
        // Transform the stream
        let mapped = stream.map(|result| {
            result
                .map(|chunk| StreamChunk {
                    delta: chunk.message.content,
                    done: chunk.done.unwrap_or(false),
                    usage: chunk.final_data.as_ref().map(|d| TokenUsage {
                        prompt_tokens: d.prompt_eval_count.unwrap_or(0) as u32,
                        completion_tokens: d.eval_count.unwrap_or(0) as u32,
                        total_tokens: (d.prompt_eval_count.unwrap_or(0) + d.eval_count.unwrap_or(0)) as u32,
                    }),
                })
                .map_err(|e| AgentError::Provider(e.to_string()))
        });
        
        Ok(Box::pin(mapped))
    }
    
    async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        let models = self.client
            .list_local_models()
            .await
            .map_err(|e| AgentError::ProviderUnavailable(e.to_string()))?;
        
        Ok(models
            .into_iter()
            .map(|m| ModelInfo {
                id: m.name.clone(),
                name: m.name,
                context_length: None, // Not exposed by Ollama API
                supports_vision: false, // Would need to check model details
            })
            .collect())
    }
    
    fn estimate_tokens(&self, text: &str) -> u32 {
        // Llama tokenizer is roughly 4 chars per token
        (text.len() / 4) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = OllamaConfig::default();
        assert_eq!(config.host, "http://localhost");
        assert_eq!(config.port, 11434);
    }

    #[test]
    fn test_message_conversion() {
        let messages = vec![
            Message::system("You are helpful."),
            Message::user("Hello"),
        ];
        
        let converted = OllamaProvider::convert_messages(&messages);
        assert_eq!(converted.len(), 2);
    }
}
