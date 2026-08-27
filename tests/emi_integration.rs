//! Snapshot tests for the RealValue EMI engine.
//!
//! `tests/data/emi_cases.json` holds the inputs; `tests/data/emi_expected.json`
//! holds the full serialized result for each. Unlike Xfina — whose fixtures are
//! real statements containing PII and so live outside the repo — these are pure
//! numbers, so the snapshots are committed and CI checks them directly.
//!
//! The expected output was verified line by line against the original
//! JavaScript implementation of the tool (see `CHANGELOG.md` for the one
//! deliberate difference).
//!
//! To re-record after an intentional change:
//!
//! ```sh
//! UPDATE_EXPECTED=1 cargo test --test emi_integration
//! ```

use std::fs;

use xfingine::emi::{compute, EmiRequest, EmiResult};

const CASES: &str = "tests/data/emi_cases.json";
const EXPECTED: &str = "tests/data/emi_expected.json";

#[test]
fn matches_recorded_snapshots() {
    let raw = fs::read_to_string(CASES).expect("read cases");
    let requests: Vec<EmiRequest> = serde_json::from_str(&raw).expect("parse cases");
    let names: Vec<String> = serde_json::from_str::<Vec<serde_json::Value>>(&raw)
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap().to_string())
        .collect();

    let actual: Vec<EmiResult> = requests
        .iter()
        .map(|request| compute(request).expect("engine should succeed on every recorded case"))
        .collect();

    if std::env::var("UPDATE_EXPECTED").as_deref() == Ok("1") {
        let json = serde_json::to_string(&actual).unwrap();
        fs::write(EXPECTED, json).expect("write snapshots");
        return;
    }

    let expected: Vec<EmiResult> =
        serde_json::from_str(&fs::read_to_string(EXPECTED).expect("read snapshots"))
            .expect("parse snapshots");

    assert_eq!(
        expected.len(),
        actual.len(),
        "case count changed; re-record with UPDATE_EXPECTED=1"
    );

    for ((name, expected), actual) in names.iter().zip(&expected).zip(&actual) {
        assert_eq!(expected, actual, "snapshot mismatch for case {name:?}");
    }
}

/// Structural guarantees that must hold for every recorded case, whatever the
/// numbers happen to be.
#[test]
fn schedules_are_internally_consistent() {
    let raw = fs::read_to_string(CASES).expect("read cases");
    let requests: Vec<EmiRequest> = serde_json::from_str(&raw).expect("parse cases");

    for request in &requests {
        let result = compute(request).expect("engine should succeed");

        let last = result.schedule.last().expect("schedule is never empty");
        assert_eq!(last.outstanding, 0, "loan must be fully repaid");
        assert_eq!(
            last.remaining_emi, 0,
            "nothing left to pay after the last row"
        );

        let mut nominal = 0_i64;
        let mut real = 0_i64;
        for (index, row) in result.schedule.iter().enumerate() {
            assert_eq!(row.index, index as u32, "row indices are dense and ordered");
            assert_eq!(
                row.emi,
                row.principal + row.interest,
                "row {index}: emi must split exactly into principal and interest"
            );
            assert_eq!(
                row.real_emi,
                row.real_principal + row.real_interest,
                "row {index}: real emi must split exactly"
            );
            assert!(
                row.outstanding >= 0,
                "row {index}: balance never goes negative"
            );
            nominal += row.emi;
            real += row.real_emi;
        }

        assert_eq!(
            result.totals.nominal_paid, nominal,
            "totals match the schedule"
        );
        assert_eq!(
            result.totals.real_paid, real,
            "real totals match the schedule"
        );
        assert_eq!(
            result.totals.nominal_interest,
            result.totals.nominal_paid - result.totals.nominal_principal,
            "interest is what is paid above the principal"
        );

        // Every payment lands in exactly one year slot.
        let slotted: usize = result
            .years
            .iter()
            .map(|year| year.months.iter().filter(|m| m.is_some()).count())
            .sum();
        assert_eq!(
            slotted,
            result.schedule.len(),
            "per-year breakdown must cover every payment exactly once"
        );

        let year_total: i64 = result.years.iter().map(|y| y.nominal_total).sum();
        assert_eq!(
            year_total, nominal,
            "per-year totals reconcile with the schedule"
        );
    }
}

/// Inflation must only ever reduce the value of a future payment, never raise
/// it, and must be a no-op at zero.
#[test]
fn inflation_discounts_monotonically() {
    let base = EmiRequest::emi(5_000_000.0, 240, 9.0);

    let none = compute(&base.clone().with_inflation(0.0)).unwrap();
    assert_eq!(
        none.totals.real_paid, none.totals.nominal_paid,
        "zero inflation leaves real equal to nominal"
    );

    let mut previous = none.totals.real_paid;
    for rate in [2.0, 4.0, 6.0, 8.0, 10.0] {
        let result = compute(&base.clone().with_inflation(rate)).unwrap();
        assert!(
            result.totals.real_paid < previous,
            "higher inflation must lower the real cost ({rate}%)"
        );
        assert!(
            result.totals.real_paid <= result.totals.nominal_paid,
            "real cost never exceeds nominal"
        );
        previous = result.totals.real_paid;
    }
}
