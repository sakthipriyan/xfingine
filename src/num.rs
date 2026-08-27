//! Numeric helpers shared by the engines.
//!
//! The engines deliberately mirror the arithmetic of the JavaScript tools they
//! were ported from, so that the same inputs produce byte-identical output on
//! the site, in Python, and in Rust. That means IEEE-754 `f64` throughout and a
//! rounding function that matches JavaScript's `Math.round` exactly.

/// Round half **up** (towards positive infinity), the way JavaScript's
/// `Math.round` does.
///
/// This differs from Rust's [`f64::round`], which rounds half *away from zero*.
/// The two agree for every non-negative value, but loan arithmetic can produce
/// small negative intermediates, so the distinction is kept explicit.
///
/// The correction handles the one case where `(x + 0.5).floor()` is not enough:
/// values such as `0.499_999_999_999_999_94`, where adding `0.5` rounds *up* in
/// floating point and would otherwise yield `1` instead of `0`. Comparing
/// `rounded - 0.5` against the original input catches it exactly, because an
/// integer minus a half is always representable at these magnitudes.
///
/// ```
/// use xfingine::num::js_round;
/// assert_eq!(js_round(2.5), 3.0);
/// assert_eq!(js_round(-2.5), -2.0);
/// assert_eq!(js_round(0.499_999_999_999_999_94), 0.0);
/// ```
#[inline]
pub fn js_round(x: f64) -> f64 {
    let rounded = (x + 0.5).floor();
    if rounded - 0.5 > x {
        rounded - 1.0
    } else {
        rounded
    }
}

/// Round to the nearest whole rupee and narrow to an integer.
///
/// Amounts leave the engines as `i64` rupees because that is what a lender
/// actually debits — no fractional paise drift across a 360-month schedule.
#[inline]
pub fn to_rupees(x: f64) -> i64 {
    js_round(x) as i64
}
