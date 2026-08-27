//! Error type shared by every engine in the crate.

use thiserror::Error;

/// Everything that can go wrong while running an engine.
///
/// Engines never panic on bad input — they return one of these instead.
#[derive(Debug, Error)]
pub enum XfingineError {
    /// An input field was missing, out of range, or contradictory.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// The EMI is at or below the first month's interest, so the outstanding
    /// balance would never shrink and the loan would never be repaid.
    #[error(
        "monthly payment of {emi:.2} does not cover the first month's interest of {minimum:.2}; \
         the loan would never be repaid"
    )]
    PaymentBelowInterest {
        /// The payment that was supplied.
        emi: f64,
        /// The first month's interest, which the payment must exceed.
        minimum: f64,
    },

    /// The computed or supplied tenure is longer than the engine will simulate.
    #[error("tenure of {months} months exceeds the maximum supported {max} months")]
    TenureTooLong {
        /// The tenure that was asked for.
        months: u64,
        /// The ceiling the engine enforces.
        max: u32,
    },

    /// JSON could not be decoded into a request, or a result could not be encoded.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, XfingineError>;
