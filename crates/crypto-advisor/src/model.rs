//! Domain Models
//!
//! Core data types for cryptocurrency portfolio management.
//! Uses `rust_decimal` for all monetary values - never use f64 for money!

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A cryptocurrency asset
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Asset {
    /// Ticker symbol (e.g., "BTC", "ETH")
    pub symbol: String,
    
    /// Full name (e.g., "Bitcoin", "Ethereum")
    pub name: String,
    
    /// Current price in USD
    pub price_usd: Decimal,
    
    /// 24-hour price change percentage
    pub change_24h: Decimal,
    
    /// Market capitalization
    pub market_cap: Option<Decimal>,
    
    /// Risk tier (1 = lowest, 5 = highest)
    pub risk_tier: u8,
    
    /// Last price update
    pub updated_at: DateTime<Utc>,
}

impl Asset {
    pub fn new(symbol: impl Into<String>, name: impl Into<String>, price_usd: Decimal) -> Self {
        Self {
            symbol: symbol.into().to_uppercase(),
            name: name.into(),
            price_usd,
            change_24h: Decimal::ZERO,
            market_cap: None,
            risk_tier: 3, // Default to medium
            updated_at: Utc::now(),
        }
    }
    
    /// Classify risk tier based on market cap and volatility
    pub fn classify_risk(&mut self) {
        self.risk_tier = match self.symbol.as_str() {
            "BTC" | "ETH" => 1,                    // Blue chips
            "SOL" | "ADA" | "DOT" | "AVAX" => 2,  // Large caps
            "LINK" | "MATIC" | "ATOM" | "XRP" => 3, // Mid caps
            _ => 4,                                 // Small caps / memes
        };
    }
}

/// A position in an asset
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    /// Asset symbol
    pub symbol: String,
    
    /// Quantity held
    pub quantity: Decimal,
    
    /// Average cost basis per unit
    pub cost_basis: Decimal,
    
    /// Current value (quantity * current price)
    pub current_value: Decimal,
    
    /// Unrealized P&L
    pub unrealized_pnl: Decimal,
    
    /// Unrealized P&L percentage
    pub unrealized_pnl_percent: Decimal,
    
    /// When position was opened
    pub opened_at: DateTime<Utc>,
    
    /// Last update
    pub updated_at: DateTime<Utc>,
}

impl Position {
    pub fn new(symbol: impl Into<String>, quantity: Decimal, cost_basis: Decimal) -> Self {
        let total_cost = quantity * cost_basis;
        Self {
            symbol: symbol.into().to_uppercase(),
            quantity,
            cost_basis,
            current_value: total_cost, // Initially same as cost
            unrealized_pnl: Decimal::ZERO,
            unrealized_pnl_percent: Decimal::ZERO,
            opened_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
    
    /// Update position with current price
    pub fn update_price(&mut self, current_price: Decimal) {
        self.current_value = self.quantity * current_price;
        let total_cost = self.quantity * self.cost_basis;
        self.unrealized_pnl = self.current_value - total_cost;
        
        if total_cost > Decimal::ZERO {
            self.unrealized_pnl_percent = (self.unrealized_pnl / total_cost) * Decimal::from(100);
        }
        
        self.updated_at = Utc::now();
    }
    
    /// Total cost of position
    pub fn total_cost(&self) -> Decimal {
        self.quantity * self.cost_basis
    }
}

/// A portfolio of positions
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Portfolio {
    /// All positions
    pub positions: HashMap<String, Position>,
    
    /// Available cash (USD)
    pub cash_balance: Decimal,
    
    /// Owner identifier
    pub owner_id: Option<String>,
    
    /// Portfolio name
    pub name: String,
    
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl Portfolio {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            positions: HashMap::new(),
            cash_balance: Decimal::ZERO,
            owner_id: None,
            name: name.into(),
            created_at: Utc::now(),
        }
    }
    
    /// Total portfolio value (positions + cash)
    pub fn total_value(&self) -> Decimal {
        let positions_value: Decimal = self.positions.values()
            .map(|p| p.current_value)
            .sum();
        positions_value + self.cash_balance
    }
    
    /// Total unrealized P&L
    pub fn total_pnl(&self) -> Decimal {
        self.positions.values()
            .map(|p| p.unrealized_pnl)
            .sum()
    }
    
    /// Get allocation percentages
    pub fn allocations(&self) -> HashMap<String, Decimal> {
        let total = self.total_value();
        if total == Decimal::ZERO {
            return HashMap::new();
        }
        
        let mut allocs = HashMap::new();
        for (symbol, position) in &self.positions {
            let percent = (position.current_value / total) * Decimal::from(100);
            allocs.insert(symbol.clone(), percent);
        }
        
        // Include cash
        if self.cash_balance > Decimal::ZERO {
            let cash_percent = (self.cash_balance / total) * Decimal::from(100);
            allocs.insert("CASH".into(), cash_percent);
        }
        
        allocs
    }
    
    /// Add or update a position
    pub fn add_position(&mut self, position: Position) {
        self.positions.insert(position.symbol.clone(), position);
    }
    
    /// Update all positions with current prices
    pub fn update_prices(&mut self, prices: &HashMap<String, Decimal>) {
        for (symbol, position) in &mut self.positions {
            if let Some(&price) = prices.get(symbol) {
                position.update_price(price);
            }
        }
    }
}

/// Risk profile for a user or portfolio
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RiskProfile {
    /// Risk tolerance (1-5, 1 = very conservative, 5 = aggressive)
    pub tolerance: u8,
    
    /// Maximum allocation to any single asset (percentage)
    pub max_single_allocation: Decimal,
    
    /// Minimum number of assets for diversification
    pub min_assets: u8,
    
    /// Maximum allocation to high-risk assets (percentage)
    pub max_high_risk_allocation: Decimal,
    
    /// Preferred investment horizon (months)
    pub investment_horizon_months: u32,
    
    /// Whether to allow margin/leverage
    pub allow_leverage: bool,
}

impl Default for RiskProfile {
    fn default() -> Self {
        Self::conservative()
    }
}

impl RiskProfile {
    /// Very conservative - prioritize capital preservation
    pub fn conservative() -> Self {
        Self {
            tolerance: 1,
            max_single_allocation: Decimal::from(20),
            min_assets: 10,
            max_high_risk_allocation: Decimal::from(10),
            investment_horizon_months: 36,
            allow_leverage: false,
        }
    }
    
    /// Moderate - balanced approach
    pub fn moderate() -> Self {
        Self {
            tolerance: 3,
            max_single_allocation: Decimal::from(30),
            min_assets: 5,
            max_high_risk_allocation: Decimal::from(25),
            investment_horizon_months: 24,
            allow_leverage: false,
        }
    }
    
    /// Aggressive - higher risk for higher potential returns
    pub fn aggressive() -> Self {
        Self {
            tolerance: 5,
            max_single_allocation: Decimal::from(50),
            min_assets: 3,
            max_high_risk_allocation: Decimal::from(50),
            investment_horizon_months: 12,
            allow_leverage: true,
        }
    }
    
    /// Create from tolerance level (1-5)
    pub fn from_tolerance(level: u8) -> Self {
        match level {
            1 => Self::conservative(),
            2 => Self {
                tolerance: 2,
                max_single_allocation: Decimal::from(25),
                min_assets: 7,
                max_high_risk_allocation: Decimal::from(15),
                investment_horizon_months: 30,
                allow_leverage: false,
            },
            3 => Self::moderate(),
            4 => Self {
                tolerance: 4,
                max_single_allocation: Decimal::from(40),
                min_assets: 4,
                max_high_risk_allocation: Decimal::from(35),
                investment_horizon_months: 18,
                allow_leverage: false,
            },
            _ => Self::aggressive(),
        }
    }
}

/// An allocation recommendation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Allocation {
    /// Asset symbol
    pub symbol: String,
    
    /// Allocation percentage
    pub percent: Decimal,
    
    /// Dollar amount
    pub amount_usd: Decimal,
    
    /// Quantity to purchase at current price
    pub quantity: Decimal,
    
    /// Risk tier of this asset
    pub risk_tier: u8,
    
    /// Rationale for this allocation
    pub rationale: String,
}

impl Allocation {
    pub fn new(
        symbol: impl Into<String>,
        percent: Decimal,
        amount_usd: Decimal,
        price: Decimal,
        risk_tier: u8,
    ) -> Self {
        let quantity = if price > Decimal::ZERO {
            amount_usd / price
        } else {
            Decimal::ZERO
        };
        
        Self {
            symbol: symbol.into(),
            percent,
            amount_usd,
            quantity,
            risk_tier,
            rationale: String::new(),
        }
    }
    
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = rationale.into();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_position_pnl() {
        let mut pos = Position::new("BTC", dec!(0.5), dec!(40000));
        assert_eq!(pos.total_cost(), dec!(20000));
        
        pos.update_price(dec!(50000));
        assert_eq!(pos.current_value, dec!(25000));
        assert_eq!(pos.unrealized_pnl, dec!(5000));
    }

    #[test]
    fn test_portfolio_allocations() {
        let mut portfolio = Portfolio::new("Test");
        portfolio.cash_balance = dec!(1000);
        
        let mut btc = Position::new("BTC", dec!(0.1), dec!(40000));
        btc.update_price(dec!(40000));
        portfolio.add_position(btc);
        
        let allocs = portfolio.allocations();
        assert!(allocs.contains_key("BTC"));
        assert!(allocs.contains_key("CASH"));
    }
}
