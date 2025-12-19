//! Stripe Webhook Handling
//!
//! Processes Stripe webhook events for subscription lifecycle management.

use serde::{Deserialize, Serialize};
use stripe::{Event, EventObject, EventType, Webhook};
use std::sync::Arc;

use crate::error::{PaymentError, Result};
use crate::license::{License, LicenseStore, Plan};

/// Parsed webhook event
#[derive(Clone, Debug)]
pub enum WebhookEvent {
    /// Checkout completed - create license
    CheckoutCompleted {
        session_id: String,
        subscription_id: String,
        customer_email: String,
        plan: Plan,
    },
    
    /// Subscription updated - might need to change plan
    SubscriptionUpdated {
        subscription_id: String,
        status: String,
        plan: Option<Plan>,
    },
    
    /// Subscription cancelled - deactivate license
    SubscriptionCancelled {
        subscription_id: String,
    },
    
    /// Payment failed - might need to notify/deactivate
    PaymentFailed {
        subscription_id: Option<String>,
        customer_email: Option<String>,
    },
    
    /// Unhandled event type
    Other {
        event_type: String,
    },
}

/// Webhook handler
pub struct WebhookHandler<S: LicenseStore> {
    license_store: Arc<S>,
}

impl<S: LicenseStore> WebhookHandler<S> {
    pub fn new(license_store: Arc<S>) -> Self {
        Self { license_store }
    }
    
    /// Verify webhook signature and parse event
    pub fn parse_event(&self, payload: &str, signature: &str, secret: &str) -> Result<Event> {
        Webhook::construct_event(payload, signature, secret)
            .map_err(|e| PaymentError::WebhookSignature(e.to_string()))
    }
    
    /// Process a webhook event
    pub async fn handle(&self, event: Event) -> Result<WebhookEvent> {
        tracing::info!(event_type = ?event.type_, "Processing Stripe webhook");
        
        let parsed = self.parse_webhook_event(&event)?;
        
        // Handle the event
        match &parsed {
            WebhookEvent::CheckoutCompleted {
                subscription_id,
                customer_email,
                plan,
                ..
            } => {
                let license = License::new(
                    subscription_id.clone(),
                    customer_email.clone(),
                    plan.clone(),
                );
                
                self.license_store.save(&license)?;
                
                tracing::info!(
                    license_key = %license.key,
                    email = %customer_email,
                    plan = ?plan,
                    "Created new license"
                );
                
                // TODO: Send license key to customer via email
            }
            
            WebhookEvent::SubscriptionCancelled { subscription_id } => {
                if let Some(mut license) = self.license_store.get_by_subscription(subscription_id)? {
                    license.deactivate();
                    self.license_store.save(&license)?;
                    
                    tracing::info!(
                        license_key = %license.key,
                        subscription_id = %subscription_id,
                        "Deactivated license"
                    );
                }
            }
            
            WebhookEvent::SubscriptionUpdated {
                subscription_id,
                status,
                plan,
            } => {
                if let Some(mut license) = self.license_store.get_by_subscription(subscription_id)? {
                    // Update active status based on subscription status
                    let is_active = matches!(status.as_str(), "active" | "trialing");
                    
                    if is_active {
                        license.reactivate();
                    } else {
                        license.deactivate();
                    }
                    
                    // Update plan if changed
                    if let Some(new_plan) = plan {
                        license.plan = new_plan.clone();
                    }
                    
                    self.license_store.save(&license)?;
                    
                    tracing::info!(
                        license_key = %license.key,
                        status = %status,
                        active = is_active,
                        "Updated license"
                    );
                }
            }
            
            WebhookEvent::PaymentFailed { subscription_id, customer_email } => {
                tracing::warn!(
                    subscription_id = ?subscription_id,
                    email = ?customer_email,
                    "Payment failed - may need to notify customer"
                );
                // Could send notification, implement grace period, etc.
            }
            
            WebhookEvent::Other { event_type } => {
                tracing::debug!(event_type = %event_type, "Unhandled webhook event");
            }
        }
        
        Ok(parsed)
    }
    
    /// Parse Stripe event into our event type
    fn parse_webhook_event(&self, event: &Event) -> Result<WebhookEvent> {
        match event.type_ {
            EventType::CheckoutSessionCompleted => {
                if let EventObject::CheckoutSession(session) = &event.data.object {
                    let plan = session.metadata
                        .as_ref()
                        .and_then(|m| m.get("plan"))
                        .map(|p| Plan::from_str(p))
                        .unwrap_or(Plan::Pro);
                    
                    Ok(WebhookEvent::CheckoutCompleted {
                        session_id: session.id.to_string(),
                        subscription_id: session.subscription
                            .as_ref()
                            .map(|s| s.id().to_string())
                            .unwrap_or_default(),
                        customer_email: session.customer_email.clone().unwrap_or_default(),
                        plan,
                    })
                } else {
                    Err(PaymentError::WebhookParse("Invalid checkout session data".into()))
                }
            }
            
            EventType::CustomerSubscriptionUpdated => {
                if let EventObject::Subscription(sub) = &event.data.object {
                    Ok(WebhookEvent::SubscriptionUpdated {
                        subscription_id: sub.id.to_string(),
                        status: sub.status.to_string(),
                        plan: None, // Would need to look at price/product to determine plan
                    })
                } else {
                    Err(PaymentError::WebhookParse("Invalid subscription data".into()))
                }
            }
            
            EventType::CustomerSubscriptionDeleted => {
                if let EventObject::Subscription(sub) = &event.data.object {
                    Ok(WebhookEvent::SubscriptionCancelled {
                        subscription_id: sub.id.to_string(),
                    })
                } else {
                    Err(PaymentError::WebhookParse("Invalid subscription data".into()))
                }
            }
            
            EventType::InvoicePaymentFailed => {
                if let EventObject::Invoice(invoice) = &event.data.object {
                    Ok(WebhookEvent::PaymentFailed {
                        subscription_id: invoice.subscription
                            .as_ref()
                            .map(|s| s.id().to_string()),
                        customer_email: invoice.customer_email.clone(),
                    })
                } else {
                    Err(PaymentError::WebhookParse("Invalid invoice data".into()))
                }
            }
            
            _ => Ok(WebhookEvent::Other {
                event_type: format!("{:?}", event.type_),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::license::MemoryLicenseStore;

    #[test]
    fn test_webhook_handler_creation() {
        let store = Arc::new(MemoryLicenseStore::new());
        let _handler = WebhookHandler::new(store);
    }
}
