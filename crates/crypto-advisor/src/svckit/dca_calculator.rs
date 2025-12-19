//! DCA Calculator Tool
//!
//! Computes dollar-cost averaging allocations across multiple assets.

use std::sync::Arc;
use async_trait::async_trait;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use agent_core::{
    Tool, ToolSchema, ToolCall, ToolResult,
    tool::ParameterSchema,
    Result as CoreResult,
};

use crate::exchange::ExchangeClient;
use crate::model::{Allocation, RiskProfile};
use crate::strategy::DiversificationStrategy;

/// Tool for calculating DCA allocations
pub struct DCACalculatorTool {
    exchange: Arc<dyn ExchangeClient>,
}

impl DCACalculatorTool {
    pub fn new(exchange: Arc<dyn ExchangeClient>) -> Self {
        Self { exchange }
    }
}

#[async_trait]
impl Tool for DCACalculatorTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "dca_calculator".into(),
            description: "Calculate dollar-cost averaging allocations. Spreads investment across multiple assets based on risk profile.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "amount".into(),
                    param_type: "number".into(),
                    description: "Total USD amount to invest".into(),
                    required: true,
                    default: None,
                    enum_values: None,
                },
                ParameterSchema {
                    name: "risk_level".into(),
                    param_type: "string".into(),
                    description: "Risk tolerance: 'conservative', 'moderate', or 'aggressive'".into(),
                    required: false,
                    default: Some(serde_json::json!("conservative")),
                    enum_values: Some(vec![
                        serde_json::json!("conservative"),
                        serde_json::json!("moderate"),
                        serde_json::json!("aggressive"),
                    ]),
                },
                ParameterSchema {
                    name: "exclude".into(),
                    param_type: "string".into(),
                    description: "Comma-separated symbols to exclude from allocation".into(),
                    required: false,
                    default: None,
                    enum_values: None,
                },
            ],
            category: Some("planning".into()),
            has_side_effects: false,
        }
    }
    
    async fn execute(&self, call: &ToolCall) -> CoreResult<ToolResult> {
        // Parse amount
        let amount: Decimal = call.arguments
            .get("amount")
            .and_then(|v| v.as_f64())
            .map(|f| Decimal::from_f64_retain(f).unwrap_or(Decimal::ZERO))
            .unwrap_or(Decimal::ZERO);
        
        if amount <= Decimal::ZERO {
            return Ok(ToolResult::failure("dca_calculator", "Amount must be positive"));
        }
        
        // Parse risk level
        let risk_level = call.arguments
            .get("risk_level")
            .and_then(|v| v.as_str())
            .unwrap_or("conservative");
        
        let profile = match risk_level {
            "aggressive" => RiskProfile::aggressive(),
            "moderate" => RiskProfile::moderate(),
            _ => RiskProfile::conservative(),
        };
        
        // Parse exclusions
        let exclude: Vec<String> = call.arguments
            .get("exclude")
            .and_then(|v| v.as_str())
            .map(|s| s.split(',').map(|x| x.trim().to_uppercase()).collect())
            .unwrap_or_default();
        
        // Get current prices for standard portfolio
        let symbols = get_standard_portfolio(&profile, &exclude);
        let mut assets = Vec::new();
        
        for symbol in &symbols {
            if let Ok(asset) = self.exchange.get_price(symbol).await {
                assets.push(asset);
            }
        }
        
        if assets.is_empty() {
            return Ok(ToolResult::failure(
                "dca_calculator",
                "Could not fetch prices. Exchange may be unavailable."
            ));
        }
        
        // Calculate allocations
        let strategy = DiversificationStrategy::new(profile.clone());
        let allocations = strategy.allocate(amount, &assets);
        
        // Format output
        let mut output = format!(
            "DCA Allocation for ${:.2} ({} strategy)\n",
            amount, risk_level
        );
        output.push_str(&format!(
            "Diversified across {} assets (max {}% per asset)\n\n",
            allocations.len(),
            profile.max_single_allocation
        ));
        
        // Risk breakdown
        let mut low_risk_total = Decimal::ZERO;
        let mut med_risk_total = Decimal::ZERO;
        let mut high_risk_total = Decimal::ZERO;
        
        for alloc in &allocations {
            let line = format!(
                "  {} {:>6.1}%  ${:>8.2}  ({:.6} units)\n",
                format!("{:<6}", alloc.symbol),
                alloc.percent,
                alloc.amount_usd,
                alloc.quantity
            );
            output.push_str(&line);
            
            match alloc.risk_tier {
                1 => low_risk_total += alloc.amount_usd,
                2 => med_risk_total += alloc.amount_usd,
                _ => high_risk_total += alloc.amount_usd,
            }
        }
        
        output.push_str("\nRisk Distribution:\n");
        output.push_str(&format!(
            "  Low risk (BTC/ETH):  ${:.2} ({:.1}%)\n",
            low_risk_total,
            (low_risk_total / amount) * dec!(100)
        ));
        output.push_str(&format!(
            "  Medium risk:         ${:.2} ({:.1}%)\n",
            med_risk_total,
            (med_risk_total / amount) * dec!(100)
        ));
        output.push_str(&format!(
            "  Higher risk:         ${:.2} ({:.1}%)\n",
            high_risk_total,
            (high_risk_total / amount) * dec!(100)
        ));
        
        // Compare to all-in risk
        output.push_str("\n⚠️  ALL-IN COMPARISON:\n");
        output.push_str("  If you put $");
        output.push_str(&format!("{:.2}", amount));
        output.push_str(" into a single volatile asset:\n");
        output.push_str("  - Could gain 100%+ in a bull run\n");
        output.push_str("  - Could lose 80-100% in a crash\n");
        output.push_str("  - Recovery could take years or never happen\n");
        output.push_str("\n  Diversified approach reduces max drawdown by ~50%\n");
        
        Ok(ToolResult::success("dca_calculator", output))
    }
}

/// Get standard portfolio assets based on risk profile
fn get_standard_portfolio(profile: &RiskProfile, exclude: &[String]) -> Vec<String> {
    let all_assets = match profile.tolerance {
        1 => vec![
            "BTC", "ETH", "SOL", "ADA", "DOT",
            "LINK", "AVAX", "MATIC", "ATOM", "XRP",
        ],
        2 | 3 => vec![
            "BTC", "ETH", "SOL", "ADA", "DOT", "LINK", "AVAX",
        ],
        _ => vec![
            "BTC", "ETH", "SOL", "ADA", "LINK",
        ],
    };
    
    all_assets
        .into_iter()
        .map(String::from)
        .filter(|s| !exclude.contains(s))
        .collect()
}
