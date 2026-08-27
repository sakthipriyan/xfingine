//! WebAssembly bindings for [`xfingine`].
//!
//! Every engine is exposed twice: once taking and returning a plain JavaScript
//! object, and once taking and returning a JSON string. The object form is what
//! you normally want; the JSON form avoids a round-trip when the caller already
//! holds a string.
//!
//! Both forms throw a readable string on invalid input.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

fn to_js_error<E: std::fmt::Display>(error: E) -> JsValue {
    JsValue::from_str(&error.to_string())
}

/// Decode a JavaScript value into a serde type, by way of JSON so that the
/// object and string entry points cannot drift apart.
fn from_js<T: serde::de::DeserializeOwned>(value: &JsValue) -> Result<T, JsValue> {
    let json = js_sys::JSON::stringify(value)
        .map_err(|_| JsValue::from_str("request could not be serialized to JSON"))?;
    let json: String = json
        .as_string()
        .ok_or_else(|| JsValue::from_str("request could not be serialized to JSON"))?;
    serde_json::from_str(&json).map_err(to_js_error)
}

/// Encode a serde type as a plain JavaScript object.
fn to_js<T: serde::Serialize>(value: &T) -> Result<JsValue, JsValue> {
    let json = serde_json::to_string(value).map_err(to_js_error)?;
    js_sys::JSON::parse(&json)
}

/// Generate the object and JSON entry points for one engine.
macro_rules! bind_engine {
    ($value_fn:ident, $json_fn:ident, $request:ty, $engine:path, $req_ts:ty, $res_ts:ty) => {
        /// Run the engine over a plain JavaScript object, returning a plain
        /// JavaScript object.
        #[wasm_bindgen]
        pub fn $value_fn(request: $req_ts) -> Result<$res_ts, JsValue> {
            let request: $request = from_js(request.as_ref())?;
            let result = $engine(&request).map_err(to_js_error)?;
            Ok(to_js(&result)?.unchecked_into())
        }

        /// Run the engine over a JSON request string, returning a JSON result
        /// string.
        #[wasm_bindgen]
        pub fn $json_fn(request_json: &str) -> Result<String, JsValue> {
            let request: $request = serde_json::from_str(request_json).map_err(to_js_error)?;
            let result = $engine(&request).map_err(to_js_error)?;
            serde_json::to_string(&result).map_err(to_js_error)
        }
    };
}

#[cfg(feature = "emi")]
mod emi {
    use super::*;

    /// Hand-written TypeScript declarations, so consumers get real types
    /// instead of `any`. These must be kept in step with `xfingine::emi`'s
    /// models — the snapshot tests pin the shape on the Rust side.
    #[wasm_bindgen(typescript_custom_section)]
    const EMI_TYPES: &'static str = r#"
/** Which of the three loan variables the engine should solve for. */
export type EmiMode = "emi" | "tenure" | "loanAmount";

export interface EmiRequest {
  /** The variable to solve for. The other two must be supplied. */
  mode: EmiMode;
  /** Principal borrowed, in rupees. Required unless mode is "loanAmount". */
  loanAmount?: number;
  /** Monthly payment, in rupees. Required unless mode is "emi". */
  emi?: number;
  /** Tenure in months, at most 600. Required unless mode is "tenure". */
  months?: number;
  /** Annual nominal interest rate as a percentage, e.g. 9 for 9%. */
  interestRate: number;
  /** Annual inflation rate as a percentage. Defaults to 0. */
  inflationRate?: number;
  /** Month of the first payment, "YYYY-MM". Omit for a dateless schedule. */
  start?: string;
}

/** One month of the schedule. All amounts are whole rupees. */
export interface EmiMonth {
  /** Zero-based position in the schedule. */
  index: number;
  /** Calendar year, present only when the request had a `start`. */
  year?: number;
  /** Calendar month 1-12, present only when the request had a `start`. */
  month?: number;
  /** Amount debited. Equals `principal + interest` exactly. */
  emi: number;
  principal: number;
  interest: number;
  /** Balance remaining after this payment. */
  outstanding: number;
  /** `emi` discounted to the value of money at the start of the loan. */
  realEmi: number;
  realPrincipal: number;
  realInterest: number;
  /** Sum of the `emi` of every later month. */
  remainingEmi: number;
  realRemainingEmi: number;
}

export interface EmiYearMonth {
  nominal: number;
  real: number;
}

/** One calendar year, laid out as twelve slots with January at index 0. */
export interface EmiYear {
  year: number;
  /** Twelve slots; `null` before the first payment and after the last. */
  months: (EmiYearMonth | null)[];
  nominalTotal: number;
  realTotal: number;
  closingOutstanding: number;
}

/**
 * Headline totals. The principal is not discounted — it is received on day
 * one, so it is already in start-of-loan rupees. Only the payments stretch
 * into the future, which is where inflation bites.
 */
export interface EmiTotals {
  nominalPaid: number;
  realPaid: number;
  nominalPrincipal: number;
  nominalInterest: number;
  realPrincipal: number;
  realInterest: number;
}

export interface EmiResult {
  mode: EmiMode;
  loanAmount: number;
  emi: number;
  /** Rows in `schedule`; can be one short of the requested tenure when the
   *  final payment clears the balance early. */
  months: number;
  interestRate: number;
  inflationRate: number;
  start?: string;
  totals: EmiTotals;
  schedule: EmiMonth[];
  /** Per-calendar-year breakdown. Empty unless the request had a `start`. */
  years: EmiYear[];
}
"#;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(typescript_type = "EmiRequest")]
        pub type JsEmiRequest;

        #[wasm_bindgen(typescript_type = "EmiResult")]
        pub type JsEmiResult;
    }

    bind_engine!(
        compute_emi,
        compute_emi_json,
        xfingine::emi::EmiRequest,
        xfingine::emi::compute,
        JsEmiRequest,
        JsEmiResult
    );
}

#[cfg(feature = "emi")]
pub use emi::{compute_emi, compute_emi_json};

/// The version of the `xfingine` engine this bundle was built from.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
