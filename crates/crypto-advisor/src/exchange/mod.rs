//! Exchange Integration
//!
//! Abstractions and implementations for cryptocurrency exchanges.

mod mock;

pub use mock::MockExchangeClient;

use async_trait::async_trait;
use rust_decimal::Decimal;

use crate::error::Result;
use crate::model::Asset;

/// Exchange client trait (Strategy pattern)
///
/// Implement this for each exchange: Binance, Coinbase, Kraken, etc.
#[async_trait]
pub trait ExchangeClient: Send + Sync {
    /// Get current price for a symbol
    async fn get_price(&self, symbol: &str) -> Result<Asset>;
    
    /// Get prices for multiple symbols
    async fn get_prices(&self, symbols: &[&str]) -> Result<Vec<Asset>> {
        let mut assets = Vec::new();
        for symbol in symbols {
            if let Ok(asset) = self.get_price(symbol).await {
                assets.push(asset);
            }
        }
        Ok(assets)
    }
    
    /// Get 24h trading volume
    async fn get_volume(&self, symbol: &str) -> Result<Decimal>;
    
    /// Check if exchange is available
    async fn health_check(&self) -> bool;
    
    /// Exchange name
    fn name(&self) -> &str;
}
