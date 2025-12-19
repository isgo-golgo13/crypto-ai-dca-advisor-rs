//! # crypto-advisor
//!
//! Conservative cryptocurrency investment advisor with dollar-cost averaging
//! and risk management strategies.
//!
//! ## Philosophy
//!
//! This advisor prioritizes capital preservation over aggressive gains:
//!
//! - **Diversification over concentration** - Never put all eggs in one basket
//! - **Dollar-cost averaging** - Spread purchases over time to reduce timing risk
//! - **Risk-adjusted allocation** - Higher allocation to stable assets, lower to volatile
//! - **Position limits** - No single asset exceeds configurable % of portfolio
//!
//! ## Example: $1000 Conservative Investment
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │  Conservative DCA: $1000 across 10 assets                   │
//! ├─────────────────────────────────────────────────────────────┤
//! │  BTC  ████████████████████  $200 (20%)  - Lowest risk       │
//! │  ETH  ████████████████████  $200 (20%)  - Low risk          │
//! │  SOL  ██████████            $100 (10%)  - Medium risk       │
//! │  ADA  ██████████            $100 (10%)  - Medium risk       │
//! │  DOT  ██████████            $100 (10%)  - Medium risk       │
//! │  LINK █████                 $50  (5%)   - Higher risk       │
//! │  AVAX █████                 $50  (5%)   - Higher risk       │
//! │  MATIC█████                 $50  (5%)   - Higher risk       │
//! │  ATOM █████                 $50  (5%)   - Higher risk       │
//! │  XRP  █████                 $50  (5%)   - Higher risk       │
//! └─────────────────────────────────────────────────────────────┘
//!
//! vs All-In Single Asset (HIGH RISK):
//! │  DOGE ████████████████████████████████████████  $1000 (100%)
//! │  → Could 10x OR go to zero
//! ```

pub mod svckit;
pub mod strategy;
pub mod exchange;
pub mod model;
pub mod error;

pub use error::{AdvisorError, Result};
pub use model::{Asset, Portfolio, Position, RiskProfile, Allocation};
pub use strategy::{DCAStrategy, DiversificationStrategy, AllocationPlan};

/// Re-export tools for easy registration
pub mod tools {
    pub use crate::svckit::{
        PriceLookupTool,
        DCACalculatorTool,
        RiskAnalyzerTool,
        PortfolioTrackerTool,
    };
}

/// System prompt for the crypto advisor agent
pub const CRYPTO_ADVISOR_PROMPT: &str = r#"You are a conservative cryptocurrency investment advisor focused on risk management and capital preservation.

## Investment Philosophy

1. **Diversification over concentration** - Always spread risk across multiple assets
2. **Dollar-cost averaging (DCA)** - Spread purchases over time, never lump-sum into volatile assets
3. **Risk-adjusted allocation** - More capital to stable assets (BTC, ETH), less to speculative altcoins
4. **Position limits** - No single asset should exceed 20% of portfolio without explicit user override

## When User Wants to Invest

For any investment request:

1. First use `risk_analyzer` to understand their risk tolerance
2. Use `price_lookup` to get current market prices
3. Use `dca_calculator` to compute allocation options:
   - Conservative: 10+ assets, max 20% per asset
   - Moderate: 5-7 assets, max 30% per asset  
   - Aggressive: 3-5 assets, max 40% per asset
4. Always present the RISK COMPARISON between diversified vs all-in approaches
5. Ask for confirmation before any "all-in" single-asset recommendation

## Risk Communication

Always explain:
- Volatility metrics (how much the asset typically moves)
- Correlation (do assets move together or independently)
- Worst-case scenario (what if this goes to zero?)
- Recovery time (how long to break even after a crash?)

## Tools Available

- `price_lookup` - Get current prices from exchanges
- `dca_calculator` - Compute diversified allocations
- `risk_analyzer` - Assess volatility and risk metrics
- `portfolio_tracker` - Track positions and P&L

Never make investment decisions without using these tools first."#;
