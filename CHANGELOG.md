# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Categorizer Engine (`categorizer` feature):** A rule-based transaction auto-mapping engine. It maps transaction descriptions to deterministic `(category, merchant)` paths based on configured keywords, and includes a feedback loop algorithm to automatically extract new rule patterns from manually labeled transactions using a TF-IDF-inspired token exclusivity heuristic.

### Fixed

- **CI:** bumped `actions/checkout`, `actions/setup-node`, and `actions/upload-artifact` to v7, and `actions/download-artifact` to v8, to move off the deprecated Node 20 runtime.
- **CI:** the macOS Intel wheel job targeted the `macos-13` runner, which
  GitHub has retired, so it sat queued forever and never got a machine. Since
  `pypi_publish` waits on every wheel job, this blocked the PyPI upload for
  0.0.2 while crates.io and npm published normally. Both macOS wheels now build
  on `macos-latest` (Apple Silicon), with x86_64 cross-compiled — Xcode ships
  both SDKs, so it needs nothing beyond the extra rustup target. Verified
  locally: the cross-compiled wheel contains a genuine `Mach-O 64-bit x86_64`
  extension.
- **CI:** the Linux wheel jobs pinned `ubuntu-22.04`, which will be retired the
  same way. Switched to `ubuntu-latest`; manylinux compatibility comes from
  maturin-action's container, not from the host image.

## [0.0.2] - 2026-08-28

### Changed

- **npm:** the package is now published as `xfingine` rather than
  `xfingine-wasm`, matching the crates.io and PyPI names. npm accepted the
  unsuffixed name. The crate remains `xfingine-wasm`, since Cargo refuses two
  packages named `xfingine` in one workspace and wasm-pack has no npm-name
  override; the publish workflow renames the generated manifest instead.

### Fixed

- **Python wheels:** the PyPI job built a single wheel for whichever platform
  and CPython version the runner happened to have, so only that exact
  combination could `pip install xfingine` without a Rust toolchain — 0.0.1
  shipped nothing but a macOS arm64 CPython 3.9 wheel. Fixed on three fronts,
  mirroring [sakthipriyan/xfina#58](https://github.com/sakthipriyan/xfina/issues/58):
  - The extension now builds against the **stable ABI** (`pyo3/abi3-py38`), so
    one wheel per OS/arch covers CPython 3.8+ instead of needing one per
    version — turning a ~25-build matrix into 5.
  - Wheels are built for **linux x86_64 / aarch64, macOS x86_64 / arm64, and
    windows x64** via `PyO3/maturin-action`, which cross-compiles properly. A
    plain `maturin build` only ever targets the runner's own platform, which
    was the root cause.
  - An **sdist** is built and uploaded as its own job, giving pip a source
    fallback on any platform without a prebuilt wheel.
- **Python metadata:** `pyproject.toml` advertised PyPy support that an abi3
  CPython extension cannot provide. Replaced with explicit CPython 3.8–3.13
  classifiers, so the metadata matches what is actually shipped.

### Notes

- First release published through GitHub Actions rather than from a laptop, and
  the first to use OIDC trusted publishing on all three registries. No
  long-lived API tokens are stored in the repository or in CI secrets.

## [0.0.1] - 2026-08-28

Initial release. Published from a local machine to claim the `xfingine` name on
crates.io, npm and PyPI — a prerequisite for configuring OIDC trusted
publishing, which each registry attaches to an already-existing package. The
contents are the full working library below rather than a placeholder stub.

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
