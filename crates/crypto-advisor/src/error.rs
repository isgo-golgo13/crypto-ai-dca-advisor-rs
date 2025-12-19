//! Error Types for Crypto Advisor

use thiserror::Error;

pub type Result<T> = std::result::Result<T, AdvisorError>;

#[derive(Error, Debug)]
pub enum AdvisorError {
    #[error("Exchange error: {0}")]
    Exchange(String),
    
    #[error("Price unavailable for {0}")]
    PriceUnavailable(String),
    
    #[error("Invalid allocation: {0}")]
    InvalidAllocation(String),
    
    #[error("Insufficient funds: need {needed}, have {available}")]
    InsufficientFunds {
        needed: rust_decimal::Decimal,
        available: rust_decimal::Decimal,
    },
    
    #[error("Position limit exceeded: {asset} at {percent}% exceeds {limit}% max")]
    PositionLimitExceeded {
        asset: String,
        percent: rust_decimal::Decimal,
        limit: rust_decimal::Decimal,
    },
    
    #[error("Risk threshold exceeded: {0}")]
    RiskThresholdExceeded(String),
    
    #[error("Asset not supported: {0}")]
    UnsupportedAsset(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
