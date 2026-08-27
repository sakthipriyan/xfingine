//! # Xfingine
//!
//! **Xfingine** is the computation layer behind the personal-finance tools on
//! [sakthipriyan.com](https://sakthipriyan.com/building-wealth/tools/). It is a
//! pure library: data in, arithmetic, data out. No UI, no I/O, no clock, no
//! network — the same inputs always produce the same outputs, on every target.
//!
//! One core written in Rust, shipped three ways:
//!
//! | Ecosystem | Package | Install |
//! |---|---|---|
//! | Rust | [`xfingine`](https://crates.io/crates/xfingine) | `cargo add xfingine` |
//! | JavaScript | [`xfingine-wasm`](https://www.npmjs.com/package/xfingine-wasm) | `npm i xfingine-wasm` |
//! | Python | [`xfingine`](https://pypi.org/project/xfingine/) | `pip install xfingine` |
//!
//! ## Engines
//!
//! - [`emi`] — the **RealValue EMI Engine**: amortization schedules that report
//!   both nominal rupees and rupees discounted for inflation.
//!
//! ## Example
//!
//! ```
//! use xfingine::emi::{compute, EmiRequest};
//!
//! // ₹50L at 9% over 20 years, against 6% inflation.
//! let result = compute(&EmiRequest::emi(5_000_000.0, 240, 9.0).with_inflation(6.0))?;
//!
//! println!("EMI          ₹{}", result.emi);
//! println!("Nominal cost ₹{}", result.totals.nominal_paid);
//! println!("Real cost    ₹{}", result.totals.real_paid);
//! # Ok::<(), xfingine::error::XfingineError>(())
//! ```
//!
//! ## Cargo features
//!
//! Every engine sits behind its own feature so a WASM bundle only carries the
//! maths it actually uses. `default = ["all"]` turns them all on.
//!
//! ```toml
//! # just the EMI engine
//! xfingine = { version = "0.1", default-features = false, features = ["emi"] }
//! ```
//!
//! ## Conventions
//!
//! - **Money is `i64` rupees on the way out.** Engines work in `f64` and round
//!   to whole rupees at the boundary, because that is what a lender debits.
//! - **Rates are percentages**, not fractions: `9.0` means 9%.
//! - **JSON is `camelCase`** across Rust, WASM and Python.

#![warn(missing_docs)]

#[cfg(feature = "emi")]
pub mod emi;

pub mod error;
pub mod num;
