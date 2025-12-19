//! Portfolio Tracker Tool
//!
//! Tracks positions, calculates P&L, and monitors allocations.

use std::sync::Arc;
use async_trait::async_trait;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tokio::sync::RwLock;

use agent_core::{
    Tool, ToolSchema, ToolCall, ToolResult,
    tool::ParameterSchema,
    Result as CoreResult,
};

use crate::exchange::ExchangeClient;
use crate::model::{Portfolio, Position};

/// Tool for tracking portfolio positions
pub struct PortfolioTrackerTool {
    exchange: Arc<dyn ExchangeClient>,
    portfolios: Arc<RwLock<std::collections::HashMap<String, Portfolio>>>,
}

impl PortfolioTrackerTool {
    pub fn new(exchange: Arc<dyn ExchangeClient>) -> Self {
        Self {
            exchange,
            portfolios: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
    
    /// Create with existing portfolios
    pub fn with_portfolios(
        exchange: Arc<dyn ExchangeClient>,
        portfolios: Arc<RwLock<std::collections::HashMap<String, Portfolio>>>,
    ) -> Self {
        Self { exchange, portfolios }
    }
}

#[async_trait]
impl Tool for PortfolioTrackerTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "portfolio_tracker".into(),
            description: "Track cryptocurrency portfolio positions, P&L, and allocation percentages.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "action".into(),
                    param_type: "string".into(),
                    description: "Action: 'view', 'add', 'remove', or 'update'".into(),
                    required: true,
                    default: None,
                    enum_values: Some(vec![
                        serde_json::json!("view"),
                        serde_json::json!("add"),
                        serde_json::json!("remove"),
                        serde_json::json!("update"),
                    ]),
                },
                ParameterSchema {
                    name: "portfolio_id".into(),
                    param_type: "string".into(),
                    description: "Portfolio identifier (default: 'default')".into(),
                    required: false,
                    default: Some(serde_json::json!("default")),
                    enum_values: None,
                },
                ParameterSchema {
                    name: "symbol".into(),
                    param_type: "string".into(),
                    description: "Asset symbol (for add/remove actions)".into(),
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ParameterSchema {
                    name: "quantity".into(),
                    param_type: "number".into(),
                    description: "Quantity (for add action)".into(),
                    required: false,
                    default: None,
                    enum_values: None,
                },
                ParameterSchema {
                    name: "cost_basis".into(),
                    param_type: "number".into(),
                    description: "Cost basis per unit in USD (for add action)".into(),
                    required: false,
                    default: None,
                    enum_values: None,
                },
            ],
            category: Some("tracking".into()),
            has_side_effects: true,
        }
    }
    
    async fn execute(&self, call: &ToolCall) -> CoreResult<ToolResult> {
        let action = call.arguments
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("view");
        
        let portfolio_id = call.arguments
            .get("portfolio_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
        
        match action {
            "view" => self.view_portfolio(&portfolio_id).await,
            "add" => {
                let symbol = call.arguments
                    .get("symbol")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| agent_core::AgentError::ToolValidation(
                        "Symbol required for add".into()
                    ))?;
                
                let quantity = call.arguments
                    .get("quantity")
                    .and_then(|v| v.as_f64())
                    .map(|f| Decimal::from_f64_retain(f).unwrap_or(Decimal::ZERO))
                    .unwrap_or(Decimal::ZERO);
                
                let cost_basis = call.arguments
                    .get("cost_basis")
                    .and_then(|v| v.as_f64())
                    .map(|f| Decimal::from_f64_retain(f).unwrap_or(Decimal::ZERO))
                    .unwrap_or(Decimal::ZERO);
                
                self.add_position(&portfolio_id, symbol, quantity, cost_basis).await
            }
            "remove" => {
                let symbol = call.arguments
                    .get("symbol")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| agent_core::AgentError::ToolValidation(
                        "Symbol required for remove".into()
                    ))?;
                
                self.remove_position(&portfolio_id, symbol).await
            }
            "update" => self.update_prices(&portfolio_id).await,
            _ => Ok(ToolResult::failure("portfolio_tracker", "Invalid action")),
        }
    }
}

impl PortfolioTrackerTool {
    async fn view_portfolio(&self, portfolio_id: &str) -> CoreResult<ToolResult> {
        let portfolios = self.portfolios.read().await;
        
        let portfolio = match portfolios.get(portfolio_id) {
            Some(p) => p,
            None => return Ok(ToolResult::success(
                "portfolio_tracker",
                format!("Portfolio '{}' not found. Use 'add' to create positions.", portfolio_id)
            )),
        };
        
        if portfolio.positions.is_empty() {
            return Ok(ToolResult::success(
                "portfolio_tracker",
                format!("Portfolio '{}' is empty.", portfolio_id)
            ));
        }
        
        let mut output = format!("Portfolio: {}\n", portfolio.name);
        output.push_str("═".repeat(60).as_str());
        output.push('\n');
        
        let mut total_cost = Decimal::ZERO;
        let mut total_value = Decimal::ZERO;
        
        for (symbol, pos) in &portfolio.positions {
            let pnl_sign = if pos.unrealized_pnl >= Decimal::ZERO { "+" } else { "" };
            output.push_str(&format!(
                "{:<6} {:>12.6} units @ ${:.2} = ${:.2} ({}${:.2} / {}{:.1}%)\n",
                symbol,
                pos.quantity,
                pos.cost_basis,
                pos.current_value,
                pnl_sign, pos.unrealized_pnl,
                pnl_sign, pos.unrealized_pnl_percent
            ));
            total_cost += pos.total_cost();
            total_value += pos.current_value;
        }
        
        output.push_str("─".repeat(60).as_str());
        output.push('\n');
        
        let total_pnl = total_value - total_cost;
        let total_pnl_pct = if total_cost > Decimal::ZERO {
            (total_pnl / total_cost) * dec!(100)
        } else {
            Decimal::ZERO
        };
        let pnl_sign = if total_pnl >= Decimal::ZERO { "+" } else { "" };
        
        output.push_str(&format!("Total Cost:  ${:.2}\n", total_cost));
        output.push_str(&format!("Total Value: ${:.2}\n", total_value));
        output.push_str(&format!("Total P&L:   {}${:.2} ({}{:.1}%)\n", 
            pnl_sign, total_pnl, pnl_sign, total_pnl_pct));
        
        // Show allocations
        output.push_str("\nAllocations:\n");
        let allocations = portfolio.allocations();
        let mut allocs: Vec<_> = allocations.iter().collect();
        allocs.sort_by(|a, b| b.1.cmp(a.1));
        
        for (symbol, percent) in allocs {
            let bar_len = (percent.to_string().parse::<f64>().unwrap_or(0.0) / 5.0) as usize;
            let bar = "█".repeat(bar_len.min(20));
            output.push_str(&format!("  {:<6} {:>5.1}% {}\n", symbol, percent, bar));
        }
        
        Ok(ToolResult::success("portfolio_tracker", output))
    }
    
    async fn add_position(
        &self,
        portfolio_id: &str,
        symbol: &str,
        quantity: Decimal,
        cost_basis: Decimal,
    ) -> CoreResult<ToolResult> {
        if quantity <= Decimal::ZERO || cost_basis <= Decimal::ZERO {
            return Ok(ToolResult::failure(
                "portfolio_tracker",
                "Quantity and cost_basis must be positive"
            ));
        }
        
        let mut portfolios = self.portfolios.write().await;
        
        let portfolio = portfolios
            .entry(portfolio_id.to_string())
            .or_insert_with(|| Portfolio::new(portfolio_id));
        
        // Get current price
        let current_price = match self.exchange.get_price(symbol).await {
            Ok(asset) => asset.price_usd,
            Err(_) => cost_basis, // Fall back to cost basis
        };
        
        let mut position = Position::new(symbol, quantity, cost_basis);
        position.update_price(current_price);
        
        portfolio.add_position(position);
        
        let total_cost = quantity * cost_basis;
        Ok(ToolResult::success(
            "portfolio_tracker",
            format!(
                "Added {} {} at ${:.2}/unit (${:.2} total) to portfolio '{}'",
                quantity, symbol.to_uppercase(), cost_basis, total_cost, portfolio_id
            )
        ))
    }
    
    async fn remove_position(&self, portfolio_id: &str, symbol: &str) -> CoreResult<ToolResult> {
        let mut portfolios = self.portfolios.write().await;
        
        if let Some(portfolio) = portfolios.get_mut(portfolio_id) {
            if portfolio.positions.remove(&symbol.to_uppercase()).is_some() {
                return Ok(ToolResult::success(
                    "portfolio_tracker",
                    format!("Removed {} from portfolio '{}'", symbol.to_uppercase(), portfolio_id)
                ));
            }
        }
        
        Ok(ToolResult::failure(
            "portfolio_tracker",
            format!("Position {} not found in portfolio '{}'", symbol, portfolio_id)
        ))
    }
    
    async fn update_prices(&self, portfolio_id: &str) -> CoreResult<ToolResult> {
        let mut portfolios = self.portfolios.write().await;
        
        let portfolio = match portfolios.get_mut(portfolio_id) {
            Some(p) => p,
            None => return Ok(ToolResult::failure(
                "portfolio_tracker",
                format!("Portfolio '{}' not found", portfolio_id)
            )),
        };
        
        let mut updated = 0;
        let symbols: Vec<String> = portfolio.positions.keys().cloned().collect();
        
        for symbol in symbols {
            if let Ok(asset) = self.exchange.get_price(&symbol).await {
                if let Some(pos) = portfolio.positions.get_mut(&symbol) {
                    pos.update_price(asset.price_usd);
                    updated += 1;
                }
            }
        }
        
        Ok(ToolResult::success(
            "portfolio_tracker",
            format!("Updated prices for {} positions in '{}'", updated, portfolio_id)
        ))
    }
}
