//! Conversation Messages
//!
//! Standard message format used across the agent system.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Role of a message sender
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System prompt/instructions
    System,
    /// User input
    User,
    /// Assistant (LLM) response
    Assistant,
    /// Tool result (injected as context)
    Tool,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::Tool => write!(f, "tool"),
        }
    }
}

/// A single message in a conversation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    /// Message role
    pub role: Role,
    
    /// Text content
    pub content: String,
    
    /// Optional name (for multi-user scenarios)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    
    /// Timestamp
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,
    
    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<MessageMetadata>,
}

/// Additional message metadata
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// Token count (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<u32>,
    
    /// Tool call ID (for tool messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    
    /// Model that generated this (for assistant messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    
    /// Custom key-value pairs
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

impl Message {
    /// Create a new message
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            name: None,
            timestamp: Utc::now(),
            metadata: None,
        }
    }
    
    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content)
    }
    
    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }
    
    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
    }
    
    /// Create a tool result message
    pub fn tool(content: impl Into<String>, tool_call_id: Option<String>) -> Self {
        let mut msg = Self::new(Role::Tool, content);
        if tool_call_id.is_some() {
            msg.metadata = Some(MessageMetadata {
                tool_call_id,
                ..Default::default()
            });
        }
        msg
    }
    
    /// Add a name to the message
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    
    /// Estimate token count (rough approximation)
    pub fn estimate_tokens(&self) -> u32 {
        // ~4 characters per token is a rough estimate
        (self.content.len() / 4) as u32 + 4 // +4 for role overhead
    }
}

/// Conversation history with utility methods
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Conversation {
    messages: Vec<Message>,
    
    /// Maximum context length (in estimated tokens)
    #[serde(default = "default_max_context")]
    max_context_tokens: u32,
}

fn default_max_context() -> u32 {
    8192
}

impl Conversation {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_system_prompt(prompt: impl Into<String>) -> Self {
        let mut conv = Self::new();
        conv.push(Message::system(prompt));
        conv
    }
    
    /// Add a message
    pub fn push(&mut self, message: Message) {
        self.messages.push(message);
    }
    
    /// Get all messages
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }
    
    /// Get messages as mutable
    pub fn messages_mut(&mut self) -> &mut Vec<Message> {
        &mut self.messages
    }
    
    /// Get the last message
    pub fn last(&self) -> Option<&Message> {
        self.messages.last()
    }
    
    /// Clear all messages except system prompt
    pub fn clear_history(&mut self) {
        self.messages.retain(|m| m.role == Role::System);
    }
    
    /// Estimate total tokens in conversation
    pub fn estimate_tokens(&self) -> u32 {
        self.messages.iter().map(|m| m.estimate_tokens()).sum()
    }
    
    /// Truncate to fit within token limit, preserving system and recent messages
    pub fn truncate_to_fit(&mut self) {
        while self.estimate_tokens() > self.max_context_tokens && self.messages.len() > 2 {
            // Find first non-system message and remove it
            if let Some(pos) = self.messages.iter().position(|m| m.role != Role::System) {
                // Don't remove the very last message
                if pos < self.messages.len() - 1 {
                    self.messages.remove(pos);
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
    
    /// Number of messages
    pub fn len(&self) -> usize {
        self.messages.len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_conversation() {
        let mut conv = Conversation::with_system_prompt("You are helpful.");
        conv.push(Message::user("Hi"));
        conv.push(Message::assistant("Hello!"));
        
        assert_eq!(conv.len(), 3);
        assert!(conv.last().unwrap().role == Role::Assistant);
    }
}
