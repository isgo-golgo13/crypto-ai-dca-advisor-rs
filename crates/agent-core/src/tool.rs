//! Tool System
//!
//! Extensible tool framework for agent capabilities.
//! Tools are registered at runtime and invoked by the reasoning loop.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{AgentError, Result};

/// Tool call request from the LLM
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool identifier
    pub name: String,
    
    /// Arguments as key-value pairs
    pub arguments: HashMap<String, serde_json::Value>,
    
    /// Optional call ID for tracking
    #[serde(default)]
    pub id: Option<String>,
}

/// Result from tool execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolResult {
    /// Tool that was called
    pub name: String,
    
    /// Call ID (if provided in request)
    pub id: Option<String>,
    
    /// Whether execution succeeded
    pub success: bool,
    
    /// Output (success message or error)
    pub output: String,
    
    /// Structured data (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl ToolResult {
    pub fn success(name: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            id: None,
            success: true,
            output: output.into(),
            data: None,
        }
    }
    
    pub fn failure(name: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            id: None,
            success: false,
            output: error.into(),
            data: None,
        }
    }
    
    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
    
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
}

/// Parameter definition for tool schema
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParameterSchema {
    /// Parameter name
    pub name: String,
    
    /// JSON Schema type (string, number, boolean, object, array)
    #[serde(rename = "type")]
    pub param_type: String,
    
    /// Human-readable description
    pub description: String,
    
    /// Whether this parameter is required
    #[serde(default)]
    pub required: bool,
    
    /// Default value if not provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    
    /// Enum of allowed values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<serde_json::Value>>,
}

/// Tool definition schema (for LLM function calling)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolSchema {
    /// Unique tool identifier
    pub name: String,
    
    /// Human-readable description (shown to LLM)
    pub description: String,
    
    /// Parameter definitions
    pub parameters: Vec<ParameterSchema>,
    
    /// Category for grouping
    #[serde(default)]
    pub category: Option<String>,
    
    /// Whether tool has side effects
    #[serde(default)]
    pub has_side_effects: bool,
}

/// Tool trait - implement to add new capabilities
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool's schema for LLM function calling
    fn schema(&self) -> ToolSchema;
    
    /// Execute the tool with given arguments
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult>;
    
    /// Validate arguments before execution (optional)
    fn validate(&self, call: &ToolCall) -> Result<()> {
        let schema = self.schema();
        
        for param in &schema.parameters {
            if param.required && !call.arguments.contains_key(&param.name) {
                return Err(AgentError::ToolValidation(format!(
                    "Missing required parameter: {}",
                    param.name
                )));
            }
        }
        
        Ok(())
    }
}

/// Registry for available tools
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }
    
    /// Register a new tool
    pub fn register<T: Tool + 'static>(&mut self, tool: T) {
        let schema = tool.schema();
        self.tools.insert(schema.name.clone(), Arc::new(tool));
    }
    
    /// Register a boxed tool
    pub fn register_boxed(&mut self, tool: Arc<dyn Tool>) {
        let schema = tool.schema();
        self.tools.insert(schema.name.clone(), tool);
    }
    
    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }
    
    /// Execute a tool call
    pub async fn execute(&self, call: &ToolCall) -> Result<ToolResult> {
        let tool = self.get(&call.name).ok_or_else(|| {
            AgentError::ToolNotFound(call.name.clone())
        })?;
        
        // Validate first
        tool.validate(call)?;
        
        // Execute
        tool.execute(call).await
    }
    
    /// Get all tool schemas (for system prompt generation)
    pub fn schemas(&self) -> Vec<ToolSchema> {
        self.tools.values().map(|t| t.schema()).collect()
    }
    
    /// Get tool names
    pub fn names(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }
    
    /// Number of registered tools
    pub fn len(&self) -> usize {
        self.tools.len()
    }
    
    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
    
    /// Generate system prompt section describing available tools
    pub fn generate_prompt_section(&self) -> String {
        let mut prompt = String::from("## Available Tools\n\n");
        prompt.push_str("You can use the following tools by responding with a JSON block:\n\n");
        prompt.push_str("```tool\n{\"tool\": \"tool_name\", \"arguments\": {\"arg\": \"value\"}}\n```\n\n");
        
        for schema in self.schemas() {
            prompt.push_str(&format!("### {}\n", schema.name));
            prompt.push_str(&format!("{}\n", schema.description));
            
            if !schema.parameters.is_empty() {
                prompt.push_str("**Parameters:**\n");
                for param in &schema.parameters {
                    let required = if param.required { " (required)" } else { "" };
                    prompt.push_str(&format!(
                        "- `{}` ({}){}: {}\n",
                        param.name, param.param_type, required, param.description
                    ));
                }
            }
            prompt.push('\n');
        }
        
        prompt
    }
}

// ============================================================================
// Built-in Tools
// ============================================================================

/// DateTime tool - returns current time
pub struct DateTimeTool;

#[async_trait]
impl Tool for DateTimeTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "datetime".into(),
            description: "Get the current date and time".into(),
            parameters: vec![
                ParameterSchema {
                    name: "format".into(),
                    param_type: "string".into(),
                    description: "Output format: 'iso', 'human', or 'unix'".into(),
                    required: false,
                    default: Some(serde_json::json!("human")),
                    enum_values: Some(vec![
                        serde_json::json!("iso"),
                        serde_json::json!("human"),
                        serde_json::json!("unix"),
                    ]),
                },
                ParameterSchema {
                    name: "timezone".into(),
                    param_type: "string".into(),
                    description: "Timezone (default: UTC)".into(),
                    required: false,
                    default: Some(serde_json::json!("UTC")),
                    enum_values: None,
                },
            ],
            category: Some("time".into()),
            has_side_effects: false,
        }
    }
    
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult> {
        let format = call.arguments
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("human");
        
        let now = chrono::Utc::now();
        
        let output = match format {
            "iso" => now.to_rfc3339(),
            "unix" => now.timestamp().to_string(),
            _ => now.format("%A, %B %d, %Y at %H:%M:%S UTC").to_string(),
        };
        
        Ok(ToolResult::success("datetime", output))
    }
}

/// Calculator tool - evaluates mathematical expressions
pub struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "calculate".into(),
            description: "Evaluate a mathematical expression".into(),
            parameters: vec![
                ParameterSchema {
                    name: "expression".into(),
                    param_type: "string".into(),
                    description: "Mathematical expression to evaluate (e.g., '2 + 2', '10 * 5')".into(),
                    required: true,
                    default: None,
                    enum_values: None,
                },
            ],
            category: Some("math".into()),
            has_side_effects: false,
        }
    }
    
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult> {
        let expr = call.arguments
            .get("expression")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::ToolValidation("Missing expression".into()))?;
        
        match evaluate_expression(expr) {
            Ok(result) => Ok(ToolResult::success("calculate", format!("{} = {}", expr, result))),
            Err(e) => Ok(ToolResult::failure("calculate", e)),
        }
    }
}

/// Simple expression evaluator (for production, use meval or fasteval)
fn evaluate_expression(expr: &str) -> std::result::Result<f64, String> {
    let expr = expr.replace(' ', "");
    
    // Handle parentheses recursively
    if let Some(start) = expr.rfind('(') {
        if let Some(end) = expr[start..].find(')') {
            let inner = &expr[start + 1..start + end];
            let inner_result = evaluate_expression(inner)?;
            let new_expr = format!(
                "{}{}{}",
                &expr[..start],
                inner_result,
                &expr[start + end + 1..]
            );
            return evaluate_expression(&new_expr);
        }
    }
    
    // Addition/subtraction (lowest precedence, evaluated last)
    for (i, c) in expr.char_indices().rev() {
        if i > 0 && (c == '+' || c == '-') {
            // Make sure it's not a unary minus
            let prev_char = expr.chars().nth(i - 1).unwrap_or(' ');
            if prev_char.is_ascii_digit() || prev_char == ')' {
                let left = evaluate_expression(&expr[..i])?;
                let right = evaluate_expression(&expr[i + 1..])?;
                return Ok(if c == '+' { left + right } else { left - right });
            }
        }
    }
    
    // Multiplication/division
    for (i, c) in expr.char_indices().rev() {
        if c == '*' || c == '/' {
            let left = evaluate_expression(&expr[..i])?;
            let right = evaluate_expression(&expr[i + 1..])?;
            if c == '/' && right == 0.0 {
                return Err("Division by zero".into());
            }
            return Ok(if c == '*' { left * right } else { left / right });
        }
    }
    
    // Power
    if let Some(i) = expr.find('^') {
        let left = evaluate_expression(&expr[..i])?;
        let right = evaluate_expression(&expr[i + 1..])?;
        return Ok(left.powf(right));
    }
    
    // Parse number
    expr.parse::<f64>().map_err(|e| format!("Parse error: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculator() {
        assert!((evaluate_expression("2 + 2").unwrap() - 4.0).abs() < f64::EPSILON);
        assert!((evaluate_expression("10 * 5").unwrap() - 50.0).abs() < f64::EPSILON);
        assert!((evaluate_expression("(2 + 3) * 4").unwrap() - 20.0).abs() < f64::EPSILON);
        assert!((evaluate_expression("2 ^ 8").unwrap() - 256.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_tool_registry() {
        let mut registry = ToolRegistry::new();
        registry.register(DateTimeTool);
        registry.register(CalculatorTool);
        
        assert_eq!(registry.len(), 2);
        assert!(registry.get("datetime").is_some());
        assert!(registry.get("calculate").is_some());
        assert!(registry.get("unknown").is_none());
    }
}
