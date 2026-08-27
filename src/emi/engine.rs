//! The RealValue EMI computation.

use crate::error::{Result, XfingineError};
use crate::num::{js_round, to_rupees};

use super::model::{
    EmiMode, EmiMonth, EmiRequest, EmiResult, EmiTotals, EmiYear, EmiYearMonth, MAX_LOAN_MONTHS,
};

/// The scheduled payment that amortizes `principal` over `months` at
/// `monthly_rate` (a per-month fraction, not a percentage).
///
/// This is the standard annuity formula; a zero rate degenerates to an even
/// split of the principal.
pub fn payment_for(principal: f64, monthly_rate: f64, months: u32) -> f64 {
    if monthly_rate == 0.0 {
        return principal / months as f64;
    }
    let compound = (1.0 + monthly_rate).powi(months as i32);
    principal * monthly_rate * compound / (compound - 1.0)
}

/// The principal that a payment of `emi` amortizes over `months` — the annuity
/// formula solved the other way round.
pub fn principal_for(emi: f64, monthly_rate: f64, months: u32) -> f64 {
    if monthly_rate == 0.0 {
        return emi * months as f64;
    }
    let compound = (1.0 + monthly_rate).powi(months as i32);
    emi * (compound - 1.0) / (monthly_rate * compound)
}

/// How many months a payment of `emi` needs to clear `principal`, rounded up.
///
/// Fails with [`XfingineError::PaymentBelowInterest`] when the payment does not
/// even cover the first month's interest, since the balance would then grow
/// without bound.
pub fn months_for(principal: f64, emi: f64, monthly_rate: f64) -> Result<u32> {
    if emi <= 0.0 {
        return Err(XfingineError::InvalidInput(
            "emi must be greater than zero".into(),
        ));
    }

    let months = if monthly_rate == 0.0 {
        (principal / emi).ceil()
    } else {
        let minimum = principal * monthly_rate;
        if emi <= minimum {
            return Err(XfingineError::PaymentBelowInterest { emi, minimum });
        }
        (emi / (emi - principal * monthly_rate)).ln() / (1.0 + monthly_rate).ln()
    }
    .ceil();

    if !months.is_finite() || months < 1.0 {
        return Err(XfingineError::InvalidInput(
            "could not derive a valid tenure from the supplied loan amount and emi".into(),
        ));
    }
    if months > MAX_LOAN_MONTHS as f64 {
        return Err(XfingineError::TenureTooLong {
            months: months as u64,
            max: MAX_LOAN_MONTHS,
        });
    }
    Ok(months as u32)
}

/// Run the EMI engine.
///
/// Solves for whichever variable [`EmiRequest::mode`] names, then simulates the
/// loan month by month, tracking both the rupees actually debited and the same
/// payments discounted back to the value of money at the start of the loan.
///
/// # Errors
///
/// Returns [`XfingineError::InvalidInput`] for missing or out-of-range fields,
/// [`XfingineError::PaymentBelowInterest`] when a payment cannot cover the
/// interest, and [`XfingineError::TenureTooLong`] beyond
/// [`MAX_LOAN_MONTHS`].
pub fn compute(request: &EmiRequest) -> Result<EmiResult> {
    validate(request)?;

    let monthly_rate = request.interest_rate / 100.0 / 12.0;

    // Solve for the missing variable.
    let (loan_amount, scheduled_emi, months) = match request.mode {
        EmiMode::Emi => {
            let loan_amount = required(request.loan_amount, "loanAmount", request.mode)?;
            let months = required_months(request.months, request.mode)?;
            (
                loan_amount,
                payment_for(loan_amount, monthly_rate, months),
                months,
            )
        }
        EmiMode::LoanAmount => {
            let emi = required(request.emi, "emi", request.mode)?;
            let months = required_months(request.months, request.mode)?;
            (principal_for(emi, monthly_rate, months), emi, months)
        }
        EmiMode::Tenure => {
            let loan_amount = required(request.loan_amount, "loanAmount", request.mode)?;
            let emi = required(request.emi, "emi", request.mode)?;
            let months = months_for(loan_amount, emi, monthly_rate)?;
            (loan_amount, emi, months)
        }
    };

    let mut schedule = simulate(
        loan_amount,
        scheduled_emi,
        months,
        monthly_rate,
        request.inflation_rate,
    );

    fill_remaining(&mut schedule);
    if let Some(start) = request.start {
        for row in schedule.iter_mut() {
            let at = start.advance(row.index);
            row.year = Some(at.year);
            row.month = Some(at.month);
        }
    }

    let nominal_paid: i64 = schedule.iter().map(|row| row.emi).sum();
    let real_paid: i64 = schedule.iter().map(|row| row.real_emi).sum();
    let principal = to_rupees(loan_amount);

    let years = if request.start.is_some() {
        group_by_year(&schedule)
    } else {
        Vec::new()
    };

    Ok(EmiResult {
        mode: request.mode,
        loan_amount: principal,
        emi: to_rupees(scheduled_emi),
        months: schedule.len() as u32,
        interest_rate: request.interest_rate,
        inflation_rate: request.inflation_rate,
        start: request.start,
        totals: EmiTotals {
            nominal_paid,
            real_paid,
            nominal_principal: principal,
            nominal_interest: nominal_paid - principal,
            real_principal: principal,
            real_interest: real_paid - principal,
        },
        schedule,
        years,
    })
}

/// Walk the loan month by month.
///
/// The payment is rounded to a whole rupee up front, the way a lender quotes
/// it, and each month's split is rounded so that `principal + interest` equals
/// the debit exactly — no stray paise that would otherwise accumulate over
/// hundreds of rows. The final payment is trimmed to whatever is left, which is
/// why the last row rarely matches the quoted EMI.
fn simulate(
    loan_amount: f64,
    scheduled_emi: f64,
    months: u32,
    monthly_rate: f64,
    inflation_rate: f64,
) -> Vec<EmiMonth> {
    // Monthly equivalent of the annual inflation rate, compounded.
    let inflation_step = (1.0 + inflation_rate / 100.0).powf(1.0 / 12.0);
    let rounded_emi = js_round(scheduled_emi);

    let mut schedule = Vec::with_capacity(months as usize);
    let mut outstanding = loan_amount;
    let mut inflation_factor = 1.0_f64;

    for index in 0..months {
        let interest_exact = outstanding * monthly_rate;
        let is_last = index == months - 1 || outstanding <= rounded_emi;

        let (emi, principal, interest) = if is_last {
            // Clear the balance exactly rather than leaving a rupee behind.
            let principal = js_round(outstanding);
            let interest = js_round(interest_exact);
            outstanding = 0.0;
            (principal + interest, principal, interest)
        } else {
            let principal = js_round(rounded_emi - interest_exact);
            // Derive interest from the debit so the row always sums exactly.
            let interest = rounded_emi - principal;
            outstanding = js_round(outstanding - (rounded_emi - interest_exact));
            (rounded_emi, principal, interest)
        };

        let real_principal = js_round(principal / inflation_factor);
        let real_interest = js_round(interest / inflation_factor);

        schedule.push(EmiMonth {
            index,
            year: None,
            month: None,
            emi: emi as i64,
            principal: principal as i64,
            interest: interest as i64,
            outstanding: outstanding as i64,
            real_emi: (real_principal + real_interest) as i64,
            real_principal: real_principal as i64,
            real_interest: real_interest as i64,
            remaining_emi: 0,
            real_remaining_emi: 0,
        });

        if outstanding == 0.0 {
            break;
        }

        inflation_factor *= inflation_step;
    }

    schedule
}

/// Fill in what is still owed after each row, in one backwards pass.
fn fill_remaining(schedule: &mut [EmiMonth]) {
    let mut nominal = 0_i64;
    let mut real = 0_i64;

    for row in schedule.iter_mut().rev() {
        row.remaining_emi = nominal;
        row.real_remaining_emi = real;
        nominal += row.emi;
        real += row.real_emi;
    }
}

/// Collapse a dated schedule into calendar-year rows.
fn group_by_year(schedule: &[EmiMonth]) -> Vec<EmiYear> {
    let mut years: Vec<EmiYear> = Vec::new();

    for row in schedule {
        let (Some(year), Some(month)) = (row.year, row.month) else {
            continue;
        };

        let entry = match years.last_mut() {
            Some(last) if last.year == year => last,
            _ => {
                years.push(EmiYear {
                    year,
                    months: vec![None; 12],
                    nominal_total: 0,
                    real_total: 0,
                    closing_outstanding: 0,
                });
                years.last_mut().expect("just pushed")
            }
        };

        entry.months[(month - 1) as usize] = Some(EmiYearMonth {
            nominal: row.emi,
            real: row.real_emi,
        });
        entry.nominal_total += row.emi;
        entry.real_total += row.real_emi;
        entry.closing_outstanding = row.outstanding;
    }

    years
}

fn validate(request: &EmiRequest) -> Result<()> {
    if !request.interest_rate.is_finite() || request.interest_rate < 0.0 {
        return Err(XfingineError::InvalidInput(
            "interestRate must be zero or greater".into(),
        ));
    }
    if !request.inflation_rate.is_finite() || request.inflation_rate < 0.0 {
        return Err(XfingineError::InvalidInput(
            "inflationRate must be zero or greater".into(),
        ));
    }
    Ok(())
}

fn required(value: Option<f64>, field: &str, mode: EmiMode) -> Result<f64> {
    let value = value.ok_or_else(|| {
        XfingineError::InvalidInput(format!("{field} is required when solving for {mode:?}"))
    })?;
    if !value.is_finite() || value <= 0.0 {
        return Err(XfingineError::InvalidInput(format!(
            "{field} must be greater than zero"
        )));
    }
    Ok(value)
}

fn required_months(months: Option<u32>, mode: EmiMode) -> Result<u32> {
    let months = months.ok_or_else(|| {
        XfingineError::InvalidInput(format!("months is required when solving for {mode:?}"))
    })?;
    if months == 0 {
        return Err(XfingineError::InvalidInput(
            "months must be greater than zero".into(),
        ));
    }
    if months > MAX_LOAN_MONTHS {
        return Err(XfingineError::TenureTooLong {
            months: months as u64,
            max: MAX_LOAN_MONTHS,
        });
    }
    Ok(months)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::emi::model::YearMonth;

    /// The textbook EMI on ₹50L at 9% over 20 years is ₹44,986.
    #[test]
    fn payment_matches_the_annuity_formula() {
        let emi = payment_for(5_000_000.0, 9.0 / 100.0 / 12.0, 240);
        assert_eq!(js_round(emi), 44_986.0);
    }

    /// Solving for the loan amount must invert solving for the payment.
    #[test]
    fn payment_and_principal_are_inverses() {
        let rate = 9.0 / 100.0 / 12.0;
        let emi = payment_for(5_000_000.0, rate, 240);
        let back = principal_for(emi, rate, 240);
        assert!((back - 5_000_000.0).abs() < 1e-6, "round-tripped to {back}");
    }

    /// A zero rate is not a special-cased approximation — it splits evenly.
    #[test]
    fn zero_rate_splits_the_principal_evenly() {
        assert_eq!(payment_for(120_000.0, 0.0, 12), 10_000.0);
        assert_eq!(principal_for(10_000.0, 0.0, 12), 120_000.0);
        assert_eq!(months_for(120_000.0, 10_000.0, 0.0).unwrap(), 12);
    }

    /// A payment that cannot cover the first month's interest is rejected
    /// rather than looping forever.
    #[test]
    fn payment_below_interest_is_rejected() {
        // ₹50L at 12% accrues ₹50,000 of interest in month one.
        let error = months_for(5_000_000.0, 50_000.0, 12.0 / 100.0 / 12.0).unwrap_err();
        assert!(
            matches!(error, XfingineError::PaymentBelowInterest { .. }),
            "got {error:?}"
        );
    }

    #[test]
    fn tenure_beyond_the_cap_is_rejected() {
        let request = EmiRequest::emi(5_000_000.0, MAX_LOAN_MONTHS + 1, 9.0);
        assert!(matches!(
            compute(&request).unwrap_err(),
            XfingineError::TenureTooLong { .. }
        ));
    }

    #[test]
    fn missing_fields_are_reported_by_name() {
        let request = EmiRequest {
            mode: EmiMode::Emi,
            loan_amount: None,
            emi: None,
            months: Some(240),
            interest_rate: 9.0,
            inflation_rate: 0.0,
            start: None,
        };
        let message = compute(&request).unwrap_err().to_string();
        assert!(message.contains("loanAmount"), "got {message}");
    }

    #[test]
    fn negative_rates_are_rejected() {
        let mut request = EmiRequest::emi(5_000_000.0, 240, -1.0);
        assert!(compute(&request).is_err());

        request = EmiRequest::emi(5_000_000.0, 240, 9.0).with_inflation(-1.0);
        assert!(compute(&request).is_err());
    }

    /// The last row clears the balance exactly, in whole rupees.
    #[test]
    fn the_final_payment_clears_the_balance() {
        let request = EmiRequest::emi(1_234_567.0, 137, 8.75).with_inflation(5.5);
        let result = compute(&request).unwrap();
        let last = result.schedule.last().unwrap();

        assert_eq!(last.outstanding, 0);
        assert_eq!(last.emi, last.principal + last.interest);

        let repaid: i64 = result.schedule.iter().map(|row| row.principal).sum();
        assert_eq!(
            repaid, result.loan_amount,
            "principal repaid equals borrowed"
        );
    }

    /// Without a start month the engine reads no clock and emits no dates.
    #[test]
    fn dates_are_opt_in() {
        let undated = compute(&EmiRequest::emi(1_000_000.0, 60, 9.0)).unwrap();
        assert!(undated.years.is_empty());
        assert!(undated.schedule.iter().all(|row| row.year.is_none()));

        let dated = compute(
            &EmiRequest::emi(1_000_000.0, 60, 9.0).with_start(YearMonth::new(2026, 11).unwrap()),
        )
        .unwrap();
        assert_eq!(dated.schedule[0].year, Some(2026));
        assert_eq!(dated.schedule[0].month, Some(11));
        // Nov 2026 + 59 months = Oct 2031.
        assert_eq!(dated.schedule[59].year, Some(2031));
        assert_eq!(dated.schedule[59].month, Some(10));
        // Six calendar years touched: 2026 through 2031.
        assert_eq!(dated.years.len(), 6);
        assert_eq!(
            dated.years[0].months.iter().filter(|m| m.is_some()).count(),
            2
        );
    }

    /// `remaining_emi` is what is still owed *after* each row.
    #[test]
    fn remaining_counts_only_later_rows() {
        let result = compute(&EmiRequest::emi(1_000_000.0, 12, 9.0)).unwrap();
        let total: i64 = result.schedule.iter().map(|row| row.emi).sum();

        assert_eq!(
            result.schedule[0].remaining_emi,
            total - result.schedule[0].emi
        );
        assert_eq!(result.schedule.last().unwrap().remaining_emi, 0);
    }

    #[test]
    fn year_month_round_trips_through_json() {
        let ym = YearMonth::new(2026, 3).unwrap();
        assert_eq!(serde_json::to_string(&ym).unwrap(), "\"2026-03\"");
        assert_eq!(
            serde_json::from_str::<YearMonth>("\"2026-03\"").unwrap(),
            ym
        );
        assert!(serde_json::from_str::<YearMonth>("\"2026-13\"").is_err());
        assert!(YearMonth::new(2026, 0).is_none());
    }
}
