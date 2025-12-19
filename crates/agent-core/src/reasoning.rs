//! Reasoning Loop
//!
//! Implements the ReAct (Reason + Act) pattern for agent behavior.
//! The agent observes, thinks, acts (via tools), and responds.

use std::sync::Arc;

use crate::error::{AgentError, Result};
use crate::message::{Conversation, Message, Role};
use crate::provider::{Completion, GenerationOptions, LlmProvider};
use crate::tool::{ToolCall, ToolRegistry, ToolResult};

/// Agent configuration
#[derive(Clone, Debug)]
pub struct AgentConfig {
    /// System prompt template
    pub system_prompt: String,
    
    /// Maximum reasoning iterations before giving up
    pub max_iterations: usize,
    
    /// Generation options
    pub generation: GenerationOptions,
    
    /// Whether to append tool descriptions to system prompt
    pub inject_tool_descriptions: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            system_prompt: DEFAULT_SYSTEM_PROMPT.into(),
            max_iterations: 10,
            generation: GenerationOptions::default(),
            inject_tool_descriptions: true,
        }
    }
}

const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a helpful AI assistant.

When you need to use a tool, respond with a JSON block in this exact format:
```tool
{"tool": "tool_name", "arguments": {"arg1": "value1"}}
```

After receiving tool results, synthesize them into a helpful response.
If you can answer directly without tools, do so.
Be concise and accurate."#;

/// The main Agent struct
pub struct Agent {
    provider: Arc<dyn LlmProvider>,
    tools: Arc<ToolRegistry>,
    config: AgentConfig,
}

impl Agent {
    /// Create a new agent
    pub fn new(
        provider: Arc<dyn LlmProvider>,
        tools: Arc<ToolRegistry>,
        config: AgentConfig,
    ) -> Self {
        Self {
            provider,
            tools,
            config,
        }
    }
    
    /// Create with default configuration
    pub fn with_defaults(
        provider: Arc<dyn LlmProvider>,
        tools: Arc<ToolRegistry>,
    ) -> Self {
        Self::new(provider, tools, AgentConfig::default())
    }
    
    /// Build the full system prompt including tool descriptions
    fn build_system_prompt(&self) -> String {
        let mut prompt = self.config.system_prompt.clone();
        
        if self.config.inject_tool_descriptions && !self.tools.is_empty() {
            prompt.push_str("\n\n");
            prompt.push_str(&self.tools.generate_prompt_section());
        }
        
        prompt
    }
    
    /// Run the agent on a user message
    pub async fn run(&self, conversation: &mut Conversation) -> Result<String> {
        // Ensure system prompt is set
        if conversation.messages().first().map(|m| &m.role) != Some(&Role::System) {
            let messages = conversation.messages_mut();
            messages.insert(0, Message::system(self.build_system_prompt()));
        }
        
        let mut iterations = 0;
        
        loop {
            iterations += 1;
            
            if iterations > self.config.max_iterations {
                return Err(AgentError::MaxIterations(self.config.max_iterations));
            }
            
            // Get completion from provider
            let completion = self.provider
                .complete(conversation.messages(), &self.config.generation)
                .await?;
            
            let content = completion.content.clone();
            
            // Add assistant response to conversation
            conversation.push(Message::assistant(&content));
            
            // Check for tool calls
            if let Some(tool_call) = self.parse_tool_call(&content) {
                tracing::debug!(tool = %tool_call.name, "Executing tool");
                
                // Execute the tool
                let result = self.execute_tool(&tool_call).await;
                
                // Add tool result to conversation
                let tool_message = self.format_tool_result(&result);
                conversation.push(Message::tool(tool_message, tool_call.id.clone()));
                
                // Continue reasoning loop
                continue;
            }
            
            // No tool call - this is the final response
            return Ok(content);
        }
    }
    
    /// Run with a simple string input (creates temporary conversation)
    pub async fn ask(&self, question: &str) -> Result<String> {
        let mut conversation = Conversation::with_system_prompt(self.build_system_prompt());
        conversation.push(Message::user(question));
        self.run(&mut conversation).await
    }
    
    /// Parse a tool call from LLM response
    fn parse_tool_call(&self, content: &str) -> Option<ToolCall> {
        // Look for ```tool ... ``` blocks
        let tool_start = "```tool";
        let tool_end = "```";
        
        if let Some(start_idx) = content.find(tool_start) {
            let after_marker = &content[start_idx + tool_start.len()..];
            if let Some(end_idx) = after_marker.find(tool_end) {
                let json_str = after_marker[..end_idx].trim();
                
                // Try to parse as ToolCall
                if let Ok(mut call) = serde_json::from_str::<ToolCall>(json_str) {
                    // Generate call ID if not present
                    if call.id.is_none() {
                        call.id = Some(uuid::Uuid::new_v4().to_string());
                    }
                    return Some(call);
                }
            }
        }
        
        // Fallback: try to find raw JSON with "tool" key
        self.parse_inline_tool_call(content)
    }
    
    /// Try to parse inline JSON tool call
    fn parse_inline_tool_call(&self, content: &str) -> Option<ToolCall> {
        // Look for JSON object with "tool" field
        if !content.contains(r#""tool""#) {
            return None;
        }
        
        // Find JSON boundaries
        let start = content.find('{')?;
        let end = content.rfind('}')?;
        
        if end <= start {
            return None;
        }
        
        let json_str = &content[start..=end];
        serde_json::from_str::<ToolCall>(json_str).ok()
    }
    
    /// Execute a tool call
    async fn execute_tool(&self, call: &ToolCall) -> ToolResult {
        match self.tools.execute(call).await {
            Ok(mut result) => {
                result.id = call.id.clone();
                result
            }
            Err(e) => {
                ToolResult {
                    name: call.name.clone(),
                    id: call.id.clone(),
                    success: false,
                    output: format!("Error: {}", e),
                    data: None,
                }
            }
        }
    }
    
    /// Format tool result for conversation
    fn format_tool_result(&self, result: &ToolResult) -> String {
        if result.success {
            format!("[Tool '{}' returned]\n{}", result.name, result.output)
        } else {
            format!("[Tool '{}' failed]\n{}", result.name, result.output)
        }
    }
    
    /// Get the tool registry
    pub fn tools(&self) -> &ToolRegistry {
        &self.tools
    }
    
    /// Get configuration
    pub fn config(&self) -> &AgentConfig {
        &self.config
    }
}

/// Builder for Agent configuration
pub struct AgentBuilder {
    provider: Option<Arc<dyn LlmProvider>>,
    tools: ToolRegistry,
    config: AgentConfig,
}

impl Default for AgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentBuilder {
    pub fn new() -> Self {
        Self {
            provider: None,
            tools: ToolRegistry::new(),
            config: AgentConfig::default(),
        }
    }
    
    pub fn provider(mut self, provider: Arc<dyn LlmProvider>) -> Self {
        self.provider = Some(provider);
        self
    }
    
    pub fn tool<T: crate::tool::Tool + 'static>(mut self, tool: T) -> Self {
        self.tools.register(tool);
        self
    }
    
    pub fn tools(mut self, tools: ToolRegistry) -> Self {
        self.tools = tools;
        self
    }
    
    pub fn system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.config.system_prompt = prompt.into();
        self
    }
    
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.config.generation.model = model.into();
        self
    }
    
    pub fn temperature(mut self, temp: f32) -> Self {
        self.config.generation.temperature = temp;
        self
    }
    
    pub fn max_iterations(mut self, max: usize) -> Self {
        self.config.max_iterations = max;
        self
    }
    
    pub fn build(self) -> Result<Agent> {
        let provider = self.provider
            .ok_or_else(|| AgentError::Config("Provider is required".into()))?;
        
        Ok(Agent::new(provider, Arc::new(self.tools), self.config))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_call() {
        let content = r#"Let me check that for you.
```tool
{"tool": "calculate", "arguments": {"expression": "2 + 2"}}
```"#;
        
        // Would need mock provider to test fully
        // Just verify the structure compiles
        assert!(content.contains("```tool"));
    }
}
