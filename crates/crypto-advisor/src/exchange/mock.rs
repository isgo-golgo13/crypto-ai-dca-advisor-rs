//! Mock Exchange Client
//!
//! For testing and demo purposes. Returns realistic static prices.

use async_trait::async_trait;
use chrono::Utc;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use super::ExchangeClient;
use crate::error::{AdvisorError, Result};
use crate::model::Asset;

/// Mock exchange client with static prices
pub struct MockExchangeClient {
    /// Add some variance to prices (for testing)
    variance_percent: f64,
}

impl Default for MockExchangeClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MockExchangeClient {
    pub fn new() -> Self {
        Self { variance_percent: 0.0 }
    }
    
    /// Create with price variance (for testing DCA over time)
    pub fn with_variance(variance_percent: f64) -> Self {
        Self { variance_percent }
    }
    
    /// Get base price for a symbol
    fn base_price(&self, symbol: &str) -> Option<(Decimal, &'static str, u8, Decimal)> {
        // (price, name, risk_tier, 24h_change)
        match symbol.to_uppercase().as_str() {
            "BTC" => Some((dec!(97500), "Bitcoin", 1, dec!(2.5))),
            "ETH" => Some((dec!(3450), "Ethereum", 1, dec!(1.8))),
            "SOL" => Some((dec!(195), "Solana", 2, dec!(4.2))),
            "ADA" => Some((dec!(0.95), "Cardano", 2, dec!(-1.2))),
            "DOT" => Some((dec!(7.20), "Polkadot", 2, dec!(0.8))),
            "LINK" => Some((dec!(24.50), "Chainlink", 3, dec!(3.1))),
            "AVAX" => Some((dec!(42.00), "Avalanche", 2, dec!(5.5))),
            "MATIC" => Some((dec!(0.52), "Polygon", 3, dec!(-0.5))),
            "ATOM" => Some((dec!(9.80), "Cosmos", 3, dec!(1.2))),
            "XRP" => Some((dec!(2.35), "Ripple", 3, dec!(0.9))),
            "DOGE" => Some((dec!(0.38), "Dogecoin", 5, dec!(12.0))),
            "SHIB" => Some((dec!(0.000022), "Shiba Inu", 5, dec!(-8.0))),
            "UNI" => Some((dec!(14.20), "Uniswap", 3, dec!(2.2))),
            "LTC" => Some((dec!(105), "Litecoin", 2, dec!(1.5))),
            "BCH" => Some((dec!(485), "Bitcoin Cash", 2, dec!(0.7))),
            _ => None,
        }
    }
}

#[async_trait]
impl ExchangeClient for MockExchangeClient {
    async fn get_price(&self, symbol: &str) -> Result<Asset> {
        let (base_price, name, risk_tier, change_24h) = self.base_price(symbol)
            .ok_or_else(|| AdvisorError::UnsupportedAsset(symbol.to_string()))?;
        
        // Apply variance if configured
        let price = if self.variance_percent > 0.0 {
            let factor = 1.0 + (rand_simple() - 0.5) * 2.0 * self.variance_percent / 100.0;
            base_price * Decimal::from_f64_retain(factor).unwrap_or(Decimal::ONE)
        } else {
            base_price
        };
        
        let mut asset = Asset::new(symbol, name, price);
        asset.risk_tier = risk_tier;
        asset.change_24h = change_24h;
        asset.updated_at = Utc::now();
        
        Ok(asset)
    }
    
    async fn get_volume(&self, symbol: &str) -> Result<Decimal> {
        // Return mock 24h volume in USD
        let volume = match symbol.to_uppercase().as_str() {
            "BTC" => dec!(25_000_000_000),
            "ETH" => dec!(15_000_000_000),
            "SOL" => dec!(3_000_000_000),
            _ => dec!(500_000_000),
        };
        Ok(volume)
    }
    
    async fn health_check(&self) -> bool {
        true // Mock always healthy
    }
    
    fn name(&self) -> &str {
        "MockExchange"
    }
}

/// Simple pseudo-random number (0.0 to 1.0)
/// For real use, use rand crate
fn rand_simple() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos % 1000) as f64 / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_exchange() {
        let exchange = MockExchangeClient::new();
        
        let btc = exchange.get_price("BTC").await.unwrap();
        assert_eq!(btc.symbol, "BTC");
        assert!(btc.price_usd > Decimal::ZERO);
        assert_eq!(btc.risk_tier, 1);
    }

    #[tokio::test]
    async fn test_unsupported_asset() {
        let exchange = MockExchangeClient::new();
        let result = exchange.get_price("NOTREAL").await;
        assert!(result.is_err());
    }
}
