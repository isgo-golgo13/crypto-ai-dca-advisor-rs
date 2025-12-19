//! Diversification Strategy
//!
//! Allocates capital across multiple assets based on risk profile.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::model::{Allocation, Asset, RiskProfile};

/// Diversification strategy for multi-asset allocation
pub struct DiversificationStrategy {
    profile: RiskProfile,
}

impl DiversificationStrategy {
    pub fn new(profile: RiskProfile) -> Self {
        Self { profile }
    }
    
    /// Allocate capital across assets
    pub fn allocate(&self, total_amount: Decimal, assets: &[Asset]) -> Vec<Allocation> {
        if assets.is_empty() || total_amount <= Decimal::ZERO {
            return Vec::new();
        }
        
        // Sort assets by risk tier (lower = safer)
        let mut sorted_assets: Vec<_> = assets.iter().collect();
        sorted_assets.sort_by_key(|a| a.risk_tier);
        
        // Calculate weights based on risk profile
        let weights = self.calculate_weights(&sorted_assets);
        
        // Create allocations
        let mut allocations = Vec::new();
        let mut remaining = total_amount;
        
        for (asset, weight) in sorted_assets.iter().zip(weights.iter()) {
            // Ensure we don't exceed max single allocation
            let capped_weight = (*weight).min(self.profile.max_single_allocation / dec!(100));
            let amount = (total_amount * capped_weight).min(remaining);
            
            if amount > Decimal::ZERO {
                let percent = (amount / total_amount) * dec!(100);
                let mut alloc = Allocation::new(
                    &asset.symbol,
                    percent,
                    amount,
                    asset.price_usd,
                    asset.risk_tier,
                );
                alloc.rationale = self.rationale_for_asset(asset, percent);
                allocations.push(alloc);
                remaining -= amount;
            }
        }
        
        allocations
    }
    
    /// Calculate weights for assets
    fn calculate_weights(&self, assets: &[&Asset]) -> Vec<Decimal> {
        let n = assets.len();
        if n == 0 {
            return Vec::new();
        }
        
        // Base strategy: risk-inverse weighting
        // Lower risk tier = higher weight
        let max_tier = assets.iter().map(|a| a.risk_tier).max().unwrap_or(5) as i32;
        
        let raw_weights: Vec<Decimal> = assets.iter()
            .map(|a| {
                let inverse_risk = (max_tier - a.risk_tier as i32 + 1) as u32;
                // Adjust by risk tolerance
                let base = Decimal::from(inverse_risk);
                match self.profile.tolerance {
                    1 => base * dec!(2.0),      // Double weight to safe assets
                    2 => base * dec!(1.5),
                    3 => base,                  // Neutral
                    4 => base * dec!(0.75),
                    _ => base * dec!(0.5),      // Reduce safe asset bias
                }
            })
            .collect();
        
        // Normalize to sum to 1
        let total: Decimal = raw_weights.iter().sum();
        if total == Decimal::ZERO {
            return vec![Decimal::ONE / Decimal::from(n as u32); n];
        }
        
        raw_weights.iter()
            .map(|w| w / total)
            .collect()
    }
    
    /// Generate rationale for an allocation
    fn rationale_for_asset(&self, asset: &Asset, percent: Decimal) -> String {
        let tier_desc = match asset.risk_tier {
            1 => "Blue chip, lowest relative risk",
            2 => "Large cap, established project",
            3 => "Mid cap, moderate risk",
            4 => "Small cap, higher risk",
            _ => "Speculative, high risk",
        };
        
        format!("{:.1}% to {} - {}", percent, asset.symbol, tier_desc)
    }
}

/// A complete allocation plan
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AllocationPlan {
    /// Plan name
    pub name: String,
    
    /// Risk profile used
    pub risk_level: String,
    
    /// Total investment amount
    pub total_amount: Decimal,
    
    /// Individual allocations
    pub allocations: Vec<Allocation>,
    
    /// Total in low-risk assets
    pub low_risk_amount: Decimal,
    
    /// Total in medium-risk assets
    pub medium_risk_amount: Decimal,
    
    /// Total in high-risk assets
    pub high_risk_amount: Decimal,
}

impl AllocationPlan {
    /// Create from allocations
    pub fn new(
        name: impl Into<String>,
        risk_level: impl Into<String>,
        total_amount: Decimal,
        allocations: Vec<Allocation>,
    ) -> Self {
        let mut low_risk = Decimal::ZERO;
        let mut med_risk = Decimal::ZERO;
        let mut high_risk = Decimal::ZERO;
        
        for alloc in &allocations {
            match alloc.risk_tier {
                1 => low_risk += alloc.amount_usd,
                2 => med_risk += alloc.amount_usd,
                _ => high_risk += alloc.amount_usd,
            }
        }
        
        Self {
            name: name.into(),
            risk_level: risk_level.into(),
            total_amount,
            allocations,
            low_risk_amount: low_risk,
            medium_risk_amount: med_risk,
            high_risk_amount: high_risk,
        }
    }
    
    /// Get risk distribution percentages
    pub fn risk_distribution(&self) -> (Decimal, Decimal, Decimal) {
        if self.total_amount == Decimal::ZERO {
            return (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO);
        }
        
        (
            (self.low_risk_amount / self.total_amount) * dec!(100),
            (self.medium_risk_amount / self.total_amount) * dec!(100),
            (self.high_risk_amount / self.total_amount) * dec!(100),
        )
    }
    
    /// Compare to all-in scenario
    pub fn vs_all_in_analysis(&self) -> String {
        let (low, med, high) = self.risk_distribution();
        
        let mut s = String::new();
        s.push_str("═══ DIVERSIFIED vs ALL-IN ═══\n\n");
        
        s.push_str("Your Diversified Plan:\n");
        s.push_str(&format!("  Low risk:    {:.1}% (${:.2})\n", low, self.low_risk_amount));
        s.push_str(&format!("  Medium risk: {:.1}% (${:.2})\n", med, self.medium_risk_amount));
        s.push_str(&format!("  High risk:   {:.1}% (${:.2})\n", high, self.high_risk_amount));
        s.push_str(&format!("  Across {} assets\n\n", self.allocations.len()));
        
        s.push_str("If All-In Single Asset:\n");
        s.push_str(&format!("  100% in ONE asset (${:.2})\n", self.total_amount));
        s.push_str("  If it 10x: You make $");
        s.push_str(&format!("{:.0}\n", self.total_amount * dec!(9)));
        s.push_str("  If it -90%: You lose $");
        s.push_str(&format!("{:.0}\n", self.total_amount * dec!(0.9)));
        s.push_str("  If it fails: You lose EVERYTHING\n\n");
        
        s.push_str("Why Diversification Wins:\n");
        s.push_str("  ✓ One asset failing doesn't wipe you out\n");
        s.push_str("  ✓ Reduced volatility, similar expected returns\n");
        s.push_str("  ✓ Easier to sleep at night\n");
        
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diversification() {
        let profile = RiskProfile::conservative();
        let strategy = DiversificationStrategy::new(profile);
        
        let assets = vec![
            Asset::new("BTC", "Bitcoin", dec!(50000)),
            Asset::new("ETH", "Ethereum", dec!(3000)),
            Asset::new("SOL", "Solana", dec!(100)),
        ];
        
        let allocations = strategy.allocate(dec!(1000), &assets);
        
        // Should have allocations for all assets
        assert!(!allocations.is_empty());
        
        // Total should roughly equal 1000
        let total: Decimal = allocations.iter().map(|a| a.amount_usd).sum();
        assert!(total <= dec!(1000));
    }
}
