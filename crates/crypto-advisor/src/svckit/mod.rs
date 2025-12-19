//! Service Kit - Agent Tools
//!
//! Domain-specific tools that implement `agent_core::Tool` for the crypto advisor.

mod price_lookup;
mod dca_calculator;
mod risk_analyzer;
mod portfolio_tracker;

pub use price_lookup::PriceLookupTool;
pub use dca_calculator::DCACalculatorTool;
pub use risk_analyzer::RiskAnalyzerTool;
pub use portfolio_tracker::PortfolioTrackerTool;
