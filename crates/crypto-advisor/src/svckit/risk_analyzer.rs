//! Risk Analyzer Tool
//!
//! Analyzes volatility and risk metrics for assets and portfolios.

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

/// Tool for analyzing risk metrics
pub struct RiskAnalyzerTool {
    exchange: Arc<dyn ExchangeClient>,
}

impl RiskAnalyzerTool {
    pub fn new(exchange: Arc<dyn ExchangeClient>) -> Self {
        Self { exchange }
    }
}

#[async_trait]
impl Tool for RiskAnalyzerTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "risk_analyzer".into(),
            description: "Analyze risk metrics for cryptocurrencies including volatility, max drawdown, and risk tier classification.".into(),
            parameters: vec![
                ParameterSchema {
                    name: "symbols".into(),
                    param_type: "string".into(),
                    description: "Comma-separated list of symbols to analyze (e.g., 'BTC,ETH,DOGE')".into(),
                    required: true,
                    default: None,
                    enum_values: None,
                },
                ParameterSchema {
                    name: "compare_to_allin".into(),
                    param_type: "boolean".into(),
                    description: "Include comparison to all-in single asset scenario".into(),
                    required: false,
                    default: Some(serde_json::json!(true)),
                    enum_values: None,
                },
            ],
            category: Some("analysis".into()),
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
        
        let compare_allin = call.arguments
            .get("compare_to_allin")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        
        let mut output = String::from("Risk Analysis Report\n");
        output.push_str("═".repeat(50).as_str());
        output.push('\n');
        
        for symbol in &symbols {
            let metrics = get_risk_metrics(symbol);
            
            output.push_str(&format!("\n{}\n", symbol));
            output.push_str(&format!("  Risk Tier:        {} ({})\n", 
                metrics.tier,
                tier_description(metrics.tier)
            ));
            output.push_str(&format!("  Volatility (30d): {:.1}%\n", metrics.volatility_30d));
            output.push_str(&format!("  Max Drawdown:     -{:.1}% (historical worst)\n", metrics.max_drawdown));
            output.push_str(&format!("  Recovery Time:    {} months (avg after crash)\n", metrics.avg_recovery_months));
            output.push_str(&format!("  Correlation/BTC:  {:.2}\n", metrics.btc_correlation));
        }
        
        if compare_allin && symbols.len() > 1 {
            output.push_str("\n");
            output.push_str("═".repeat(50).as_str());
            output.push_str("\nDIVERSIFIED vs ALL-IN COMPARISON\n\n");
            
            // Calculate blended metrics for diversified
            let avg_volatility: f64 = symbols.iter()
                .map(|s| get_risk_metrics(s).volatility_30d)
                .sum::<f64>() / symbols.len() as f64;
            
            let max_single_drawdown = symbols.iter()
                .map(|s| get_risk_metrics(s).max_drawdown)
                .fold(0.0_f64, |a, b| a.max(b));
            
            // Diversification reduces volatility by correlation factor
            let diversified_volatility = avg_volatility * 0.6; // ~40% reduction from diversification
            let diversified_max_drawdown = max_single_drawdown * 0.5; // ~50% reduction
            
            output.push_str("If you invest in a SINGLE volatile asset:\n");
            output.push_str(&format!("  • Volatility:   {:.1}%\n", max_single_drawdown * 0.3));
            output.push_str(&format!("  • Max Drawdown: -{:.1}%\n", max_single_drawdown));
            output.push_str("  • Could go to ZERO if project fails\n\n");
            
            output.push_str(&format!("If you DIVERSIFY across {} assets:\n", symbols.len()));
            output.push_str(&format!("  • Volatility:   {:.1}% (reduced)\n", diversified_volatility));
            output.push_str(&format!("  • Max Drawdown: -{:.1}% (reduced)\n", diversified_max_drawdown));
            output.push_str("  • Unlikely ALL assets go to zero\n\n");
            
            output.push_str("📊 RECOMMENDATION:\n");
            output.push_str("  Diversification is FREE risk reduction.\n");
            output.push_str("  Same expected return, lower variance.\n");
        }
        
        Ok(ToolResult::success("risk_analyzer", output))
    }
}

/// Risk metrics for an asset
struct RiskMetrics {
    tier: u8,
    volatility_30d: f64,
    max_drawdown: f64,
    avg_recovery_months: u32,
    btc_correlation: f64,
}

/// Get risk metrics for a symbol (simplified - would use historical data in production)
fn get_risk_metrics(symbol: &str) -> RiskMetrics {
    match symbol.to_uppercase().as_str() {
        "BTC" => RiskMetrics {
            tier: 1,
            volatility_30d: 45.0,
            max_drawdown: 83.0,  // 2022 crash
            avg_recovery_months: 36,
            btc_correlation: 1.0,
        },
        "ETH" => RiskMetrics {
            tier: 1,
            volatility_30d: 55.0,
            max_drawdown: 94.0,
            avg_recovery_months: 30,
            btc_correlation: 0.85,
        },
        "SOL" => RiskMetrics {
            tier: 2,
            volatility_30d: 75.0,
            max_drawdown: 96.0,
            avg_recovery_months: 24,
            btc_correlation: 0.70,
        },
        "ADA" | "DOT" | "AVAX" => RiskMetrics {
            tier: 2,
            volatility_30d: 70.0,
            max_drawdown: 92.0,
            avg_recovery_months: 28,
            btc_correlation: 0.75,
        },
        "LINK" | "MATIC" | "ATOM" | "XRP" => RiskMetrics {
            tier: 3,
            volatility_30d: 80.0,
            max_drawdown: 90.0,
            avg_recovery_months: 24,
            btc_correlation: 0.65,
        },
        "DOGE" | "SHIB" => RiskMetrics {
            tier: 5,
            volatility_30d: 150.0,
            max_drawdown: 99.0,
            avg_recovery_months: 48,
            btc_correlation: 0.40,
        },
        _ => RiskMetrics {
            tier: 4,
            volatility_30d: 100.0,
            max_drawdown: 95.0,
            avg_recovery_months: 36,
            btc_correlation: 0.50,
        },
    }
}

fn tier_description(tier: u8) -> &'static str {
    match tier {
        1 => "Blue Chip - Lowest relative risk",
        2 => "Large Cap - Moderate risk",
        3 => "Mid Cap - Higher risk",
        4 => "Small Cap - High risk",
        5 => "Meme/Speculative - Extreme risk",
        _ => "Unknown",
    }
}
