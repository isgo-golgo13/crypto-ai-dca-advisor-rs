//! Price Lookup Tool
//!
//! Fetches current cryptocurrency prices from exchanges.

use std::sync::Arc;
use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use agent_core::{
    Tool, ToolSchema, ToolCall, ToolResult,
    tool::ParameterSchema,
    Result as CoreResult,
};

use crate::exchange::ExchangeClient;
use crate::model::Asset;

/// Tool for looking up cryptocurrency prices
pub struct PriceLookupTool {
    exchange: Arc<dyn ExchangeClient>,
}

impl PriceLookupTool {
    pub fn new(exchange: Arc<dyn ExchangeClient>) -> Self {
        Self { exchange }
    }
}

#[async_trait]
impl Tool for PriceLookupTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "price_lookup".into(),
            description: "Get current cryptocurrency prices from exchanges. Returns price, 24h change, and market cap.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "symbols".into(),
                    param_type: "string".into(),
                    description: "Comma-separated list of symbols (e.g., 'BTC,ETH,SOL')".into(),
                    required: true,
                    default: None,
                    enum_values: None,
                },
            ],
            category: Some("market_data".into()),
            has_side_effects: false,
        }
    }
    
    async fn execute(&self, call: &ToolCall) -> CoreResult<ToolResult> {
        let symbols_str = call.arguments
            .get("symbols")
            .and_then(|v| v.as_str())
            .unwrap_or("BTC");
        
        let symbols: Vec<&str> = symbols_str
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        
        let mut results = Vec::new();
        let mut errors = Vec::new();
        
        for symbol in symbols {
            match self.exchange.get_price(symbol).await {
                Ok(asset) => {
                    results.push(format!(
                        "{}: ${:.2} ({:+.2}% 24h) - Risk tier: {}",
                        asset.symbol,
                        asset.price_usd,
                        asset.change_24h,
                        asset.risk_tier
                    ));
                }
                Err(e) => {
                    errors.push(format!("{}: {}", symbol, e));
                }
            }
        }
        
        let mut output = String::new();
        
        if !results.is_empty() {
            output.push_str("Current Prices:\n");
            for result in &results {
                output.push_str(&format!("  {}\n", result));
            }
        }
        
        if !errors.is_empty() {
            output.push_str("\nUnavailable:\n");
            for error in &errors {
                output.push_str(&format!("  {}\n", error));
            }
        }
        
        Ok(ToolResult::success("price_lookup", output.trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Tests would use mock exchange client
}
