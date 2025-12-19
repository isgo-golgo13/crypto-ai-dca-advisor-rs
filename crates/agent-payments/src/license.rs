//! License Management
//!
//! Handles license key generation, storage, and verification.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

use crate::error::Result;

/// License key (formatted: XXXX-XXXX-XXXX-XXXX)
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LicenseKey(String);

impl LicenseKey {
    /// Generate a new license key
    pub fn generate() -> Self {
        let id = uuid::Uuid::new_v4();
        let hex = id.simple().to_string().to_uppercase();
        Self(format!(
            "{}-{}-{}-{}",
            &hex[0..4],
            &hex[4..8],
            &hex[8..12],
            &hex[12..16]
        ))
    }
    
    /// Parse from string
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into().to_uppercase())
    }
    
    /// Get the key as a string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for LicenseKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Subscription plan tiers
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Plan {
    Free,
    Pro,
    Team,
}

impl Plan {
    pub fn as_str(&self) -> &str {
        match self {
            Plan::Free => "free",
            Plan::Pro => "pro",
            Plan::Team => "team",
        }
    }
    
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pro" => Plan::Pro,
            "team" => Plan::Team,
            _ => Plan::Free,
        }
    }
    
    /// Get rate limit (requests per day)
    pub fn rate_limit(&self) -> u32 {
        match self {
            Plan::Free => 50,
            Plan::Pro => u32::MAX, // Unlimited
            Plan::Team => u32::MAX,
        }
    }
    
    /// Get seat count
    pub fn seats(&self) -> u32 {
        match self {
            Plan::Free => 1,
            Plan::Pro => 1,
            Plan::Team => 5,
        }
    }
}

impl Default for Plan {
    fn default() -> Self {
        Plan::Free
    }
}

/// A license record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct License {
    /// License key
    pub key: LicenseKey,
    
    /// Associated Stripe subscription ID
    pub subscription_id: String,
    
    /// Customer email
    pub email: String,
    
    /// Plan tier
    pub plan: Plan,
    
    /// Whether license is active
    pub active: bool,
    
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    
    /// Expiration (None = subscription-based, auto-renews)
    pub expires_at: Option<DateTime<Utc>>,
    
    /// Last verification timestamp
    pub last_verified: Option<DateTime<Utc>>,
    
    /// Usage count (for rate limiting)
    pub usage_today: u32,
    
    /// Last usage reset date
    pub usage_reset_date: Option<chrono::NaiveDate>,
}

impl License {
    /// Create a new license
    pub fn new(subscription_id: String, email: String, plan: Plan) -> Self {
        Self {
            key: LicenseKey::generate(),
            subscription_id,
            email,
            plan,
            active: true,
            created_at: Utc::now(),
            expires_at: None,
            last_verified: None,
            usage_today: 0,
            usage_reset_date: None,
        }
    }
    
    /// Check if license is valid (active and not expired)
    pub fn is_valid(&self) -> bool {
        if !self.active {
            return false;
        }
        
        if let Some(expires) = self.expires_at {
            if Utc::now() > expires {
                return false;
            }
        }
        
        true
    }
    
    /// Check rate limit and increment usage
    pub fn check_and_increment_usage(&mut self) -> bool {
        let today = Utc::now().date_naive();
        
        // Reset if new day
        if self.usage_reset_date != Some(today) {
            self.usage_today = 0;
            self.usage_reset_date = Some(today);
        }
        
        let limit = self.plan.rate_limit();
        if self.usage_today >= limit {
            return false;
        }
        
        self.usage_today += 1;
        true
    }
    
    /// Deactivate the license
    pub fn deactivate(&mut self) {
        self.active = false;
    }
    
    /// Reactivate the license
    pub fn reactivate(&mut self) {
        self.active = true;
    }
}

/// License verification result
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LicenseVerification {
    pub valid: bool,
    pub plan: Option<Plan>,
    pub remaining_requests: Option<u32>,
    pub message: Option<String>,
}

impl LicenseVerification {
    pub fn valid(plan: Plan, remaining: u32) -> Self {
        Self {
            valid: true,
            plan: Some(plan),
            remaining_requests: Some(remaining),
            message: None,
        }
    }
    
    pub fn invalid(message: impl Into<String>) -> Self {
        Self {
            valid: false,
            plan: None,
            remaining_requests: None,
            message: Some(message.into()),
        }
    }
}

/// License storage trait
pub trait LicenseStore: Send + Sync {
    /// Save or update a license
    fn save(&self, license: &License) -> Result<()>;
    
    /// Get license by key
    fn get(&self, key: &LicenseKey) -> Result<Option<License>>;
    
    /// Get license by subscription ID
    fn get_by_subscription(&self, subscription_id: &str) -> Result<Option<License>>;
    
    /// Delete a license
    fn delete(&self, key: &LicenseKey) -> Result<()>;
    
    /// Verify and use a license (atomic check + increment)
    fn verify_and_use(&self, key: &LicenseKey) -> Result<LicenseVerification>;
}

/// In-memory license store (for development)
pub struct MemoryLicenseStore {
    licenses: RwLock<HashMap<LicenseKey, License>>,
    by_subscription: RwLock<HashMap<String, LicenseKey>>,
}

impl Default for MemoryLicenseStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryLicenseStore {
    pub fn new() -> Self {
        Self {
            licenses: RwLock::new(HashMap::new()),
            by_subscription: RwLock::new(HashMap::new()),
        }
    }
}

impl LicenseStore for MemoryLicenseStore {
    fn save(&self, license: &License) -> Result<()> {
        let mut licenses = self.licenses.write().unwrap();
        let mut by_sub = self.by_subscription.write().unwrap();
        
        by_sub.insert(license.subscription_id.clone(), license.key.clone());
        licenses.insert(license.key.clone(), license.clone());
        
        Ok(())
    }
    
    fn get(&self, key: &LicenseKey) -> Result<Option<License>> {
        let licenses = self.licenses.read().unwrap();
        Ok(licenses.get(key).cloned())
    }
    
    fn get_by_subscription(&self, subscription_id: &str) -> Result<Option<License>> {
        let by_sub = self.by_subscription.read().unwrap();
        let licenses = self.licenses.read().unwrap();
        
        if let Some(key) = by_sub.get(subscription_id) {
            Ok(licenses.get(key).cloned())
        } else {
            Ok(None)
        }
    }
    
    fn delete(&self, key: &LicenseKey) -> Result<()> {
        let mut licenses = self.licenses.write().unwrap();
        let mut by_sub = self.by_subscription.write().unwrap();
        
        if let Some(license) = licenses.remove(key) {
            by_sub.remove(&license.subscription_id);
        }
        
        Ok(())
    }
    
    fn verify_and_use(&self, key: &LicenseKey) -> Result<LicenseVerification> {
        let mut licenses = self.licenses.write().unwrap();
        
        if let Some(license) = licenses.get_mut(key) {
            if !license.is_valid() {
                return Ok(LicenseVerification::invalid("License is not active"));
            }
            
            if !license.check_and_increment_usage() {
                return Ok(LicenseVerification::invalid("Rate limit exceeded"));
            }
            
            let remaining = license.plan.rate_limit().saturating_sub(license.usage_today);
            Ok(LicenseVerification::valid(license.plan.clone(), remaining))
        } else {
            Ok(LicenseVerification::invalid("License not found"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_license_key_generation() {
        let key = LicenseKey::generate();
        assert_eq!(key.as_str().len(), 19); // XXXX-XXXX-XXXX-XXXX
        assert_eq!(key.as_str().matches('-').count(), 3);
    }

    #[test]
    fn test_license_validity() {
        let license = License::new(
            "sub_123".into(),
            "test@example.com".into(),
            Plan::Pro,
        );
        assert!(license.is_valid());
    }

    #[test]
    fn test_rate_limiting() {
        let mut license = License::new(
            "sub_123".into(),
            "test@example.com".into(),
            Plan::Free,
        );
        
        // Should allow up to 50 requests
        for _ in 0..50 {
            assert!(license.check_and_increment_usage());
        }
        
        // 51st should fail
        assert!(!license.check_and_increment_usage());
    }
}
