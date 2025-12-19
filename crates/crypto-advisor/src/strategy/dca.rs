//! Dollar-Cost Averaging Strategy
//!
//! Spreads purchases over time to reduce timing risk.

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::model::RiskProfile;

/// DCA schedule configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DCAStrategy {
    /// Total amount to invest
    pub total_amount: Decimal,
    
    /// Number of periods to spread investment
    pub periods: u32,
    
    /// Interval between purchases (days)
    pub interval_days: u32,
    
    /// Amount per period
    pub amount_per_period: Decimal,
    
    /// Start date
    pub start_date: DateTime<Utc>,
    
    /// Scheduled purchases
    pub schedule: Vec<DCAScheduleEntry>,
}

/// A single DCA purchase entry
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DCAScheduleEntry {
    /// Scheduled date
    pub date: DateTime<Utc>,
    
    /// Amount to invest
    pub amount: Decimal,
    
    /// Whether this purchase has been executed
    pub executed: bool,
    
    /// Actual execution price (if executed)
    pub execution_price: Option<Decimal>,
}

impl DCAStrategy {
    /// Create a new DCA strategy
    pub fn new(total_amount: Decimal, periods: u32, interval_days: u32) -> Self {
        let amount_per_period = total_amount / Decimal::from(periods);
        let start_date = Utc::now();
        
        let schedule = (0..periods)
            .map(|i| {
                let days_offset = i * interval_days;
                DCAScheduleEntry {
                    date: start_date + Duration::days(days_offset as i64),
                    amount: amount_per_period,
                    executed: false,
                    execution_price: None,
                }
            })
            .collect();
        
        Self {
            total_amount,
            periods,
            interval_days,
            amount_per_period,
            start_date,
            schedule,
        }
    }
    
    /// Create from risk profile
    pub fn from_risk_profile(total_amount: Decimal, profile: &RiskProfile) -> Self {
        // More conservative = more periods (slower DCA)
        let periods = match profile.tolerance {
            1 => 12,     // Monthly for a year
            2 => 8,      // Every ~6 weeks for a year
            3 => 6,      // Bi-monthly for a year
            4 => 4,      // Quarterly
            _ => 2,      // Semi-annual (aggressive)
        };
        
        // Conservative = more frequent smaller purchases
        let interval_days = match profile.tolerance {
            1 => 30,     // Monthly
            2 => 45,
            3 => 60,     // Bi-monthly
            4 => 90,     // Quarterly
            _ => 180,
        };
        
        Self::new(total_amount, periods, interval_days)
    }
    
    /// Get next scheduled purchase
    pub fn next_purchase(&self) -> Option<&DCAScheduleEntry> {
        self.schedule.iter().find(|e| !e.executed)
    }
    
    /// Mark a purchase as executed
    pub fn execute_purchase(&mut self, index: usize, price: Decimal) {
        if let Some(entry) = self.schedule.get_mut(index) {
            entry.executed = true;
            entry.execution_price = Some(price);
        }
    }
    
    /// Calculate average execution price
    pub fn average_price(&self) -> Option<Decimal> {
        let executed: Vec<_> = self.schedule.iter()
            .filter(|e| e.executed && e.execution_price.is_some())
            .collect();
        
        if executed.is_empty() {
            return None;
        }
        
        let total_spent: Decimal = executed.iter().map(|e| e.amount).sum();
        let weighted_price: Decimal = executed.iter()
            .map(|e| e.amount * e.execution_price.unwrap())
            .sum();
        
        Some(weighted_price / total_spent)
    }
    
    /// Get completion percentage
    pub fn completion_percent(&self) -> Decimal {
        let executed = self.schedule.iter().filter(|e| e.executed).count();
        Decimal::from(executed * 100) / Decimal::from(self.periods)
    }
    
    /// Generate summary
    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("DCA Strategy: ${:.2} over {} periods\n", 
            self.total_amount, self.periods));
        s.push_str(&format!("Amount per period: ${:.2}\n", self.amount_per_period));
        s.push_str(&format!("Interval: {} days\n", self.interval_days));
        s.push_str(&format!("Progress: {:.0}%\n", self.completion_percent()));
        
        if let Some(avg) = self.average_price() {
            s.push_str(&format!("Average price: ${:.2}\n", avg));
        }
        
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dca_creation() {
        let dca = DCAStrategy::new(dec!(1000), 10, 7);
        assert_eq!(dca.periods, 10);
        assert_eq!(dca.amount_per_period, dec!(100));
        assert_eq!(dca.schedule.len(), 10);
    }

    #[test]
    fn test_dca_from_profile() {
        let conservative = RiskProfile::conservative();
        let dca = DCAStrategy::from_risk_profile(dec!(1000), &conservative);
        assert_eq!(dca.periods, 12); // Monthly
        assert_eq!(dca.interval_days, 30);
    }
}
