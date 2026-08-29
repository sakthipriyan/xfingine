//! Transaction auto-categorizer engine
//!
//! Maps raw transaction descriptions to configured `/Category/Merchant` paths,
//! and provides a utility to auto-derive rules from manually mapped data using TF-IDF.

mod engine;
mod model;

pub use engine::*;
pub use model::*;
