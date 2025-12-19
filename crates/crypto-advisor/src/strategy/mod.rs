//! Investment Strategies
//!
//! Allocation algorithms for different investment approaches.

mod dca;
mod diversification;

pub use dca::DCAStrategy;
pub use diversification::{DiversificationStrategy, AllocationPlan};
