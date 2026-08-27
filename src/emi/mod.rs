//! **RealValue EMI Engine** — amortization with the inflation taken out.
//!
//! A standard EMI calculator tells you that a ₹50L loan at 9% over 20 years
//! costs ₹1.08 crore. That number is misleading: the payment in year 20 is made
//! with rupees worth far less than the payment in year 1. This engine reports
//! both — the rupees actually debited, and the same payments discounted back to
//! what money is worth on the day the loan starts.
//!
//! # Example
//!
//! ```
//! use xfingine::emi::{compute, EmiRequest, YearMonth};
//!
//! let request = EmiRequest::emi(5_000_000.0, 240, 9.0)
//!     .with_inflation(6.0)
//!     .with_start(YearMonth::new(2026, 1).unwrap());
//!
//! let result = compute(&request)?;
//!
//! assert_eq!(result.emi, 44_986);
//! assert_eq!(result.schedule.len(), 240);
//! assert_eq!(result.years.len(), 20);
//!
//! // Inflation does most of the work that prepayment gets credit for.
//! assert!(result.totals.real_paid < result.totals.nominal_paid);
//! # Ok::<(), xfingine::error::XfingineError>(())
//! ```

mod engine;
mod model;

pub use engine::{compute, months_for, payment_for, principal_for};
pub use model::{
    EmiMode, EmiMonth, EmiRequest, EmiResult, EmiTotals, EmiYear, EmiYearMonth, YearMonth,
    MAX_LOAN_MONTHS,
};
