# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-08-27

Initial release.

### Added

- **Core:** The `xfingine` crate — a pure computation layer for personal finance
  planning. Data in, arithmetic, data out: no UI, no I/O, no network, no clock.
- **RealValue EMI Engine (`emi` feature):** Loan amortization reported in both
  nominal rupees and rupees discounted for inflation. Solves for any one of
  monthly payment, tenure, or loan amount given the other two, and returns
  headline totals, the full month-by-month schedule, and an optional
  per-calendar-year breakdown. Ported from `realvalue-emi-engine.js` on
  sakthipriyan.com.
- **WASM bindings (`xfingine` on npm):** Each engine exposed as both an
  object-in/object-out function and a `_json` string variant.
- **Python bindings (`xfingine` on PyPI):** Each engine exposed as both a
  dict-in/dict-out function and a `_json` string variant, via `pyo3`.
- **Cargo features:** One feature per engine, all enabled by `default`, so a
  WASM bundle carries only the maths it uses.
- **Tests:** Committed snapshots over 16 scenarios, structural invariant checks,
  and unit coverage of the formulas, degenerate zero-rate cases, and every error
  path.
- **CI/CD:** PR checks gating on formatting, clippy, tests and a changelog
  entry; tag-triggered publishing to crates.io, npm and PyPI.
- **`xtask`:** `prepare-release` and `tag-release` for cutting versions.

### Notes on the port

The EMI engine was verified differentially against the original JavaScript, not
merely tested in isolation: both implementations were run over the same inputs
and compared field by field across every schedule row — 16 hand-picked scenarios
(2,691 rows) and 600 randomized cases (133,686 rows). Output is bit-identical
apart from one deliberate difference:

- **Fixed:** the JavaScript rounds every payment to whole rupees *except the
  final one*, where it assigns the raw floating-point balance as the closing
  principal. On loans whose principal is not itself a round number this made the
  last row — and therefore the nominal total — carry fractional paise, and could
  report a *real* final payment marginally larger than the nominal one, which is
  incoherent. Xfingine rounds the closing principal like every other row, so
  `emi == principal + interest` holds in whole rupees on all rows and the
  schedule sums exactly to the totals. The difference only surfaces on the final
  row of a loan with a non-round principal.

## [0.0.1] - 2026-08-28

Name reservation release, published to claim `xfingine` on all three registries
ahead of the real 0.1.0. Contents are the same working library described under
0.1.0 rather than a stub, so the version is usable rather than merely a
placeholder. The npm package is plain `xfingine`, not `xfingine-wasm` — npm
accepted the unsuffixed name, so it matches the crates.io and PyPI names.
