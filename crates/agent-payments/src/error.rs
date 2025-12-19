//! Payment Error Types

use thiserror::Error;

/// Result type alias
pub type Result<T> = std::result::Result<T, PaymentError>;

/// Payment-related errors
#[derive(Error, Debug)]
pub enum PaymentError {
    /// Stripe API error
    #[error("Stripe error: {0}")]
    Stripe(String),
    
    /// Webhook signature verification failed
    #[error("Webhook signature invalid: {0}")]
    WebhookSignature(String),
    
    /// Webhook payload parsing failed
    #[error("Webhook parse error: {0}")]
    WebhookParse(String),
    
    /// License not found
    #[error("License not found: {0}")]
    LicenseNotFound(String),
    
    /// License expired or invalid
    #[error("License invalid: {0}")]
    LicenseInvalid(String),
    
    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    RateLimited,
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),
    
    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),
}

impl PaymentError {
    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(self, PaymentError::Stripe(_) | PaymentError::Storage(_))
    }
    
    /// Get user-friendly message
    pub fn user_message(&self) -> &str {
        match self {
            PaymentError::Stripe(_) => "Payment processing failed. Please try again.",
            PaymentError::LicenseNotFound(_) => "License key not found.",
            PaymentError::LicenseInvalid(_) => "Your license is no longer valid.",
            PaymentError::RateLimited => "You've exceeded your usage limit.",
            PaymentError::Config(_) => "Service configuration error.",
            _ => "An error occurred processing your request.",
        }
    }
}
