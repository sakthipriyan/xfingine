//! Input and output data for the RealValue EMI engine.
//!
//! Every type here serializes to `camelCase` JSON, which is the wire format
//! shared by the Rust, WASM and Python surfaces.

use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize, Serializer};

/// The longest tenure any engine will simulate: 50 years.
pub const MAX_LOAN_MONTHS: u32 = 50 * 12;

/// Which of the three loan variables the engine should solve for.
///
/// The other two are supplied in the request; the third is derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EmiMode {
    /// Given loan amount and tenure, solve for the monthly payment.
    Emi,
    /// Given loan amount and monthly payment, solve for the tenure.
    Tenure,
    /// Given monthly payment and tenure, solve for the loan amount.
    LoanAmount,
}

/// A calendar month, serialized as the string `"YYYY-MM"`.
///
/// This matches the value of an HTML `<input type="month">`, which is where the
/// web tool gets it from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YearMonth {
    /// Four-digit calendar year.
    pub year: i32,
    /// Month of the year, `1..=12`.
    pub month: u32,
}

impl YearMonth {
    /// Construct a month, returning `None` if `month` is outside `1..=12`.
    pub fn new(year: i32, month: u32) -> Option<Self> {
        (1..=12).contains(&month).then_some(Self { year, month })
    }

    /// The month `offset` months after this one.
    pub fn advance(self, offset: u32) -> Self {
        let total = (self.month - 1) + offset;
        Self {
            year: self.year + (total / 12) as i32,
            month: (total % 12) + 1,
        }
    }
}

impl Serialize for YearMonth {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{:04}-{:02}", self.year, self.month))
    }
}

impl<'de> Deserialize<'de> for YearMonth {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        let (year, month) = raw
            .split_once('-')
            .ok_or_else(|| de::Error::custom(format!("expected \"YYYY-MM\", got {raw:?}")))?;
        let year: i32 = year
            .parse()
            .map_err(|_| de::Error::custom(format!("invalid year in {raw:?}")))?;
        let month: u32 = month
            .parse()
            .map_err(|_| de::Error::custom(format!("invalid month in {raw:?}")))?;
        YearMonth::new(year, month)
            .ok_or_else(|| de::Error::custom(format!("month out of range in {raw:?}")))
    }
}

/// The input to [`crate::emi::compute`].
///
/// Which of `loan_amount`, `emi` and `months` are required depends on `mode`;
/// the field being solved for is ignored if supplied.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmiRequest {
    /// The variable to solve for.
    pub mode: EmiMode,

    /// Principal borrowed, in rupees. Required unless `mode` is
    /// [`EmiMode::LoanAmount`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loan_amount: Option<f64>,

    /// Monthly payment, in rupees. Required unless `mode` is [`EmiMode::Emi`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emi: Option<f64>,

    /// Tenure in months. Required unless `mode` is [`EmiMode::Tenure`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub months: Option<u32>,

    /// Annual nominal interest rate, as a percentage (e.g. `9.0` for 9%).
    pub interest_rate: f64,

    /// Annual inflation rate, as a percentage, used to discount each payment
    /// back to the value of money at the start of the loan. Defaults to `0`,
    /// which makes the real figures equal the nominal ones.
    #[serde(default)]
    pub inflation_rate: f64,

    /// The month the first payment falls in. Optional: supply it to get
    /// calendar dates on each row and a per-year breakdown. Without it the
    /// engine stays purely arithmetic and reads no clock.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<YearMonth>,
}

impl EmiRequest {
    /// A request that solves for the monthly payment.
    pub fn emi(loan_amount: f64, months: u32, interest_rate: f64) -> Self {
        Self {
            mode: EmiMode::Emi,
            loan_amount: Some(loan_amount),
            emi: None,
            months: Some(months),
            interest_rate,
            inflation_rate: 0.0,
            start: None,
        }
    }

    /// A request that solves for the tenure.
    pub fn tenure(loan_amount: f64, emi: f64, interest_rate: f64) -> Self {
        Self {
            mode: EmiMode::Tenure,
            loan_amount: Some(loan_amount),
            emi: Some(emi),
            months: None,
            interest_rate,
            inflation_rate: 0.0,
            start: None,
        }
    }

    /// A request that solves for the loan amount.
    pub fn loan_amount(emi: f64, months: u32, interest_rate: f64) -> Self {
        Self {
            mode: EmiMode::LoanAmount,
            loan_amount: None,
            emi: Some(emi),
            months: Some(months),
            interest_rate,
            inflation_rate: 0.0,
            start: None,
        }
    }

    /// Set the annual inflation rate used for the real-value columns.
    pub fn with_inflation(mut self, inflation_rate: f64) -> Self {
        self.inflation_rate = inflation_rate;
        self
    }

    /// Set the month of the first payment, enabling calendar dates and the
    /// per-year breakdown.
    pub fn with_start(mut self, start: YearMonth) -> Self {
        self.start = Some(start);
        self
    }
}

/// One month of the repayment schedule. All amounts are whole rupees.
///
/// `emi == principal + interest` and `real_emi == real_principal +
/// real_interest` hold exactly on every row, by construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmiMonth {
    /// Zero-based position in the schedule.
    pub index: u32,

    /// Calendar year of this payment. `None` unless the request had a `start`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,

    /// Calendar month of this payment, `1..=12`. `None` unless the request had
    /// a `start`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub month: Option<u32>,

    /// Amount actually debited this month. Equal to the scheduled payment for
    /// every month except the last, which is trimmed to clear the balance.
    pub emi: i64,
    /// The part of `emi` that reduces the balance.
    pub principal: i64,
    /// The part of `emi` that is interest.
    pub interest: i64,
    /// Balance remaining after this payment.
    pub outstanding: i64,

    /// `emi` discounted to the value of money at the start of the loan.
    pub real_emi: i64,
    /// `principal` discounted to the start of the loan.
    pub real_principal: i64,
    /// `interest` discounted to the start of the loan.
    pub real_interest: i64,

    /// Sum of the `emi` of every *later* month — what is still left to pay
    /// after this one.
    pub remaining_emi: i64,
    /// The same, in start-of-loan rupees.
    pub real_remaining_emi: i64,
}

/// A single month's payment inside a [`EmiYear`] row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmiYearMonth {
    /// The amount debited, in rupees of the day.
    pub nominal: i64,
    /// The same amount in start-of-loan rupees.
    pub real: i64,
}

/// One calendar year of the schedule, laid out as twelve month slots.
///
/// `months[0]` is January. Slots before the first payment and after the last
/// are `None`, so a partial first or final year lines up in a calendar grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmiYear {
    /// The calendar year.
    pub year: i32,
    /// Twelve slots, January to December.
    pub months: Vec<Option<EmiYearMonth>>,
    /// Total debited during this year.
    pub nominal_total: i64,
    /// The same total in start-of-loan rupees.
    pub real_total: i64,
    /// Balance remaining after this year's last payment.
    pub closing_outstanding: i64,
}

/// Headline totals for the whole loan.
///
/// The principal is not discounted: it is received on day one, so it is already
/// in start-of-loan rupees. Only the payments stretch into the future, which is
/// exactly where inflation bites — and why `real_interest` is so much smaller
/// than `nominal_interest` on a long loan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmiTotals {
    /// Sum of every payment, in rupees of the day.
    pub nominal_paid: i64,
    /// Sum of every payment, discounted to the start of the loan.
    pub real_paid: i64,
    /// The principal borrowed.
    pub nominal_principal: i64,
    /// `nominal_paid - nominal_principal`.
    pub nominal_interest: i64,
    /// The principal borrowed — undiscounted, see the type docs.
    pub real_principal: i64,
    /// `real_paid - real_principal`.
    pub real_interest: i64,
}

/// The output of [`crate::emi::compute`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmiResult {
    /// The variable that was solved for.
    pub mode: EmiMode,
    /// Principal borrowed, rounded to rupees.
    pub loan_amount: i64,
    /// The scheduled monthly payment, rounded to rupees the way a lender does.
    pub emi: i64,
    /// Number of rows in `schedule` — the tenure actually simulated, which can
    /// be one month shorter than requested when the final payment clears early.
    pub months: u32,
    /// Annual nominal interest rate, as a percentage.
    pub interest_rate: f64,
    /// Annual inflation rate, as a percentage.
    pub inflation_rate: f64,
    /// The month of the first payment, if one was supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<YearMonth>,
    /// Headline totals.
    pub totals: EmiTotals,
    /// The full month-by-month schedule.
    pub schedule: Vec<EmiMonth>,
    /// Per-calendar-year breakdown. Empty unless the request had a `start`.
    pub years: Vec<EmiYear>,
}
