//! Stripe Checkout Integration
//!
//! Implements the "Stripe Checkout (Hosted)" approach for payment processing.

use serde::{Deserialize, Serialize};
use stripe::{
    CheckoutSession as StripeCheckoutSession, CheckoutSessionMode, Client,
    CreateCheckoutSession, CreateCheckoutSessionLineItems,
    CreateCheckoutSessionLineItemsPriceData,
    CreateCheckoutSessionLineItemsPriceDataProductData,
    CreateCheckoutSessionLineItemsPriceDataRecurring,
    CreateCheckoutSessionLineItemsPriceDataRecurringInterval,
    Currency,
};

use crate::error::{PaymentError, Result};
use crate::license::Plan;

/// Stripe client wrapper
pub struct StripeClient {
    client: Client,
    webhook_secret: String,
}

impl StripeClient {
    /// Create a new Stripe client
    pub fn new(secret_key: &str, webhook_secret: &str) -> Self {
        Self {
            client: Client::new(secret_key),
            webhook_secret: webhook_secret.to_string(),
        }
    }
    
    /// Create from environment variables
    pub fn from_env() -> Result<Self> {
        let secret_key = std::env::var("STRIPE_SECRET_KEY")
            .map_err(|_| PaymentError::Config("STRIPE_SECRET_KEY not set".into()))?;
        let webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET")
            .map_err(|_| PaymentError::Config("STRIPE_WEBHOOK_SECRET not set".into()))?;
        
        Ok(Self::new(&secret_key, &webhook_secret))
    }
    
    /// Get the webhook secret
    pub fn webhook_secret(&self) -> &str {
        &self.webhook_secret
    }
    
    /// Create a Stripe Checkout session (Hosted approach)
    ///
    /// Returns a URL to redirect the user to Stripe's hosted checkout page.
    pub async fn create_checkout_session(&self, request: CheckoutRequest) -> Result<CheckoutSession> {
        let pricing = request.plan.pricing();
        
        let mut params = CreateCheckoutSession::new();
        params.customer_email = Some(&request.customer_email);
        params.success_url = Some(&request.success_url);
        params.cancel_url = Some(&request.cancel_url);
        params.mode = Some(CheckoutSessionMode::Subscription);
        
        // Add metadata for tracking
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("plan".to_string(), request.plan.as_str().to_string());
        if let Some(ref user_id) = request.user_id {
            metadata.insert("user_id".to_string(), user_id.clone());
        }
        params.metadata = Some(metadata);
        
        // Line items
        params.line_items = Some(vec![CreateCheckoutSessionLineItems {
            quantity: Some(1),
            price_data: Some(CreateCheckoutSessionLineItemsPriceData {
                currency: Currency::USD,
                unit_amount: Some(pricing.cents),
                product_data: Some(CreateCheckoutSessionLineItemsPriceDataProductData {
                    name: pricing.name.clone(),
                    description: Some(pricing.description.clone()),
                    ..Default::default()
                }),
                recurring: Some(CreateCheckoutSessionLineItemsPriceDataRecurring {
                    interval: match pricing.interval {
                        BillingInterval::Monthly => CreateCheckoutSessionLineItemsPriceDataRecurringInterval::Month,
                        BillingInterval::Yearly => CreateCheckoutSessionLineItemsPriceDataRecurringInterval::Year,
                    },
                    interval_count: Some(1),
                }),
                ..Default::default()
            }),
            ..Default::default()
        }]);

        let session = StripeCheckoutSession::create(&self.client, params)
            .await
            .map_err(|e| PaymentError::Stripe(e.to_string()))?;
        
        let checkout_url = session.url.ok_or_else(|| {
            PaymentError::Stripe("No checkout URL returned".into())
        })?;
        
        Ok(CheckoutSession {
            id: session.id.to_string(),
            checkout_url,
            plan: request.plan,
            customer_email: request.customer_email,
        })
    }
    
    /// Get the underlying Stripe client
    pub fn inner(&self) -> &Client {
        &self.client
    }
}

/// Request to create a checkout session
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckoutRequest {
    /// Plan to purchase
    pub plan: Plan,
    
    /// Customer email
    pub customer_email: String,
    
    /// URL to redirect after successful payment
    pub success_url: String,
    
    /// URL to redirect if checkout is cancelled
    pub cancel_url: String,
    
    /// Optional user ID for tracking
    #[serde(default)]
    pub user_id: Option<String>,
}

/// Result of creating a checkout session
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckoutSession {
    /// Stripe session ID
    pub id: String,
    
    /// URL to redirect user to
    pub checkout_url: String,
    
    /// Plan being purchased
    pub plan: Plan,
    
    /// Customer email
    pub customer_email: String,
}

/// Billing interval
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BillingInterval {
    Monthly,
    Yearly,
}

/// Pricing information
#[derive(Clone, Debug)]
pub struct PlanPricing {
    pub name: String,
    pub description: String,
    pub cents: i64,
    pub interval: BillingInterval,
}

impl Plan {
    /// Get pricing for this plan
    pub fn pricing(&self) -> PlanPricing {
        match self {
            Plan::Free => PlanPricing {
                name: "Free".into(),
                description: "Basic access with limits".into(),
                cents: 0,
                interval: BillingInterval::Monthly,
            },
            Plan::Pro => PlanPricing {
                name: "Agent Pro".into(),
                description: "Unlimited local inference, priority support".into(),
                cents: 2900, // $29/month
                interval: BillingInterval::Monthly,
            },
            Plan::Team => PlanPricing {
                name: "Agent Team".into(),
                description: "5 seats, API access, custom integrations".into(),
                cents: 9900, // $99/month
                interval: BillingInterval::Monthly,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_pricing() {
        let pricing = Plan::Pro.pricing();
        assert_eq!(pricing.cents, 2900);
        assert_eq!(pricing.interval, BillingInterval::Monthly);
    }
}
