//! # agent-payments
//!
//! Payment processing and license management for rust-agent.
//!
//! ## Stripe Integration Strategies
//!
//! This crate supports two Stripe integration approaches:
//!
//! ### 1. Stripe Checkout (Hosted) - Recommended for simplicity
//!
//! **Flow:** Your site → Redirect to Stripe's hosted page → Redirect back
//!
//! ```text
//! ┌─────────────┐     ┌─────────────────┐     ┌─────────────┐
//! │  Your Site  │────▶│  Stripe Hosted  │────▶│  Your Site  │
//! │  (pricing)  │     │  Checkout Page  │     │  (success)  │
//! └─────────────┘     └─────────────────┘     └─────────────┘
//! ```
//!
//! **Pros:**
//! - Zero frontend payment code
//! - Stripe handles PCI compliance entirely
//! - Automatic localization, mobile optimization
//! - Built-in tax calculation, promo codes
//!
//! **Cons:**
//! - Users leave your site briefly
//! - Limited styling (can set colors/fonts via Stripe Dashboard)
//!
//! ### 2. Stripe Elements (Embedded) - For full UI control
//!
//! **Flow:** Payment form embedded in your page, never leave your site
//!
//! ```text
//! ┌────────────────────────────────────────┐
//! │           Your Checkout Page            │
//! │  ┌────────────────────────────────┐    │
//! │  │   Stripe Elements (iframe)     │    │
//! │  │   Card Number: ████ ████ ████  │    │
//! │  └────────────────────────────────┘    │
//! └────────────────────────────────────────┘
//! ```
//!
//! **Pros:**
//! - Complete styling control
//! - Users stay on your site
//! - Custom checkout experiences
//!
//! **Cons:**
//! - More frontend code (Stripe.js integration)
//! - You handle more of the flow
//! - Still PCI-compliant (Elements are iframes)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use agent_payments::{StripeClient, CheckoutRequest, Plan};
//!
//! let client = StripeClient::new("sk_test_xxx", "whsec_xxx");
//!
//! // Create a checkout session (Hosted approach)
//! let session = client.create_checkout_session(CheckoutRequest {
//!     plan: Plan::Pro,
//!     customer_email: "user@example.com".into(),
//!     success_url: "https://yoursite.com/success".into(),
//!     cancel_url: "https://yoursite.com/pricing".into(),
//! }).await?;
//!
//! // Redirect user to: session.checkout_url
//! ```

mod checkout;
mod license;
mod webhook;
mod error;

pub use checkout::{CheckoutRequest, CheckoutSession, StripeClient};
pub use license::{License, LicenseStore, MemoryLicenseStore, Plan};
pub use webhook::{WebhookEvent, WebhookHandler};
pub use error::{PaymentError, Result};
