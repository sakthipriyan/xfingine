# Xfingine — Agent Context & Guidelines

A cheat sheet for AI agents working on Xfingine, so you can get oriented without
re-deriving the architecture.

## What this project is (and is not)

Xfingine is a **pure computation library**. Input data → computation → output
data. There is **no UI, no CLI, no file I/O, no network, and no clock** in the
core crate, and none should be added. If a change requires reaching outside the
process, it belongs in the caller, not here.

This is the sibling project to [Xfina](https://github.com/sakthipriyan/xfina),
which parses financial statements. Xfina has a web app; Xfingine deliberately
does not. Consumers bring their own UI — today that is the tools page on
sakthipriyan.com, which loads the WASM bundle.

## Architecture Overview

- **Core (`src/`):** Plain Rust. One module per engine (`src/emi/`), each split
  into `model.rs` (the serde types on the wire) and `engine.rs` (the maths).
  Shared helpers live in `src/num.rs`; all errors are `XfingineError`.
- **WASM (`wasm/`):** `wasm-bindgen` wrappers published to npm as
  `xfingine`. Every engine is exposed twice — object in/out, and a `_json`
  twin taking and returning strings.

  **Name gotcha:** the *crate* is `xfingine-wasm` but the *npm package* is
  plain `xfingine`. Cargo refuses two packages named `xfingine` in one
  workspace, and wasm-pack has no npm-name override — so the publish workflow
  builds with `--out-name xfingine` and then runs `npm pkg set name=xfingine`
  before publishing. Keep using `-p xfingine-wasm` for cargo commands.
- **Python (`python/`):** `pyo3` + `pythonize` wrappers published to PyPI as
  `xfingine`. Same dual shape: dict in/out, plus a `_json` twin.

  Built against the **stable ABI** (`pyo3/abi3-py38`), so one wheel per OS/arch
  covers CPython 3.8+. Never drop the `abi3-py38` feature to pick up a
  version-specific pyo3 API — doing so silently multiplies the release matrix
  by every supported Python version and leaves most users with no wheel. It
  also rules out PyPy, which is why `pyproject.toml` claims CPython only.
- **`xtask/`:** Release automation only. Not published.

The bindings must stay **thin**. They deserialize, call one core function, and
serialize. No maths, no validation, no defaulting in a binding — if you find
yourself writing logic there, it belongs in the core so all three targets get
it.

## Build & Test

```bash
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --target wasm32-unknown-unknown -p xfingine-wasm

cd wasm   && wasm-pack build --target web    # npm package → wasm/pkg
cd python && maturin develop                 # importable module (needs a venv)
```

`maturin develop` needs `VIRTUAL_ENV` set or a `.venv` in the tree.

## Testing Workflow

- **Snapshots:** `tests/data/emi_cases.json` holds inputs, `emi_expected.json`
  holds full recorded results. Unlike Xfina — whose fixtures contain PII and so
  live outside the repo in `../xfina-test-data/` — these are pure numbers, so
  they are **committed** and CI checks them directly.
- **Re-recording:** `UPDATE_EXPECTED=1 cargo test`. Never re-record to make a
  failing test pass without first understanding *why* the numbers moved, and
  say so in the changelog when they legitimately did.
- **Invariants:** `schedules_are_internally_consistent` asserts properties that
  hold regardless of the numbers (rows sum exactly, balance reaches zero, the
  per-year breakdown covers every payment once). Add to it when adding an
  engine — it catches classes of bug snapshots cannot.

## Technical Rules & Conventions

1. **Money out is `i64` whole rupees.** Engines compute in `f64` and round at
   the boundary, because that is what a lender debits. Never emit fractional
   currency; paise drift accumulates badly across 360 rows.
2. **Use `num::js_round`, not `f64::round`.** The engines are ported from
   JavaScript and must match it bit for bit. `Math.round` rounds half *up*;
   Rust's `f64::round` rounds half *away from zero*. They differ on negatives.
3. **Rates are percentages, not fractions.** `9.0` means 9%. Convert to a
   monthly fraction inside the engine, never at the API boundary.
4. **JSON is `camelCase`** on every type, via
   `#[serde(rename_all = "camelCase")]`, so Rust, WASM and Python share one wire
   format.
5. **No ambient clock.** Never call `SystemTime::now()` or equivalent. Dates are
   opt-in through an explicit `start` field; without it the engine emits no
   dates and an empty per-year breakdown. Determinism is the point.
6. **Errors, never panics.** Every fallible path returns `XfingineError`.
   Validate inputs up front and name the offending field in the message, using
   the `camelCase` name the caller actually passed (`loanAmount`, not
   `loan_amount`).
7. **Every engine sits behind its own Cargo feature,** included in `all`, so a
   WASM bundle only carries the maths it uses.
8. **`#![warn(missing_docs)]` is on.** Public items need doc comments. Say what
   a number *means*, not just its type — `real_interest` needs the sentence
   about discounting far more than it needs "the real interest".

## Porting an engine from the JavaScript tools

The engines originate as `.js` files in
`../sakthipriyan.github.io/static/js/`. When porting one:

1. Extract *only* the maths — the JS files interleave it with Vue templates and
   ECharts wiring.
2. Build a differential harness: run the extracted JS and the Rust side over the
   same inputs and compare **every field of every row**, not just the totals.
   Fuzz it with a few hundred random cases too; the interesting bugs live in
   final-row and zero-rate edges.
3. Where the Rust deliberately diverges from the JS — a rounding bug in the
   original, say — record it in `CHANGELOG.md` and explain why, so nobody
   "fixes" it back later.

## Release Process

Ideally, the code change and version bump should happen in the same feature branch, rather than via a separate release PR.

```bash
# 1. In your feature branch, before or while opening the PR:
cargo xtask prepare-release <major|minor|patch>   # bumps version, rolls changelog
# (If xtask creates a branch, just merge those changes back into your feature branch)

# 2. Push, open a PR, and stop. Merging is the maintainer's call.

# 3. Post squash merge, run from the main branch:
cargo xtask tag-release                          # tags main and pushes
```

The tag fires `.github/workflows/publish.yml` → crates.io, npm, PyPI in
parallel. PRs touching `src/`, `wasm/`, `python/`, `tests/` or `Cargo.toml`
**must** update `CHANGELOG.md`; CI fails the PR otherwise.
