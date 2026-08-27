# Contributing to Xfingine

Thanks for your interest in Xfingine. The most valuable contribution is a new
**engine** — a self-contained computation that takes data in and gives data
back.

## Getting Started

1. **Fork the repository.**
2. **Install Rust** via [rustup](https://rustup.rs/).
3. **Build and test:**
   ```bash
   cargo build
   cargo test --workspace --all-features
   ```

For the bindings you will also want
[`wasm-pack`](https://rustwasm.github.io/wasm-pack/) and
[`maturin`](https://www.maturin.rs/).

## Scope

Xfingine is a **pure library**: no UI, no CLI, no file I/O, no network, and no
clock. Anything that reaches outside the process belongs in the caller. Pull
requests that add such dependencies to the core crate will be asked to move
them out.

## Adding a New Engine

Engines live in `src/<engine>/`, split into `model.rs` (the serde types) and
`engine.rs` (the maths).

1. **Create the module.** Add `src/<engine>/mod.rs`, `model.rs` and `engine.rs`,
   and declare it in `src/lib.rs` behind `#[cfg(feature = "<engine>")]`.
2. **Model the data.** Put every type on the wire in `model.rs` with
   `#[serde(rename_all = "camelCase")]`. Provide builder-style constructors for
   the common cases, the way `EmiRequest::emi(..)` does.
3. **Return errors, never panic.** Everything fallible returns
   `Result<T, XfingineError>`. Validate inputs up front, and name the offending
   field using the `camelCase` name the caller passed.
4. **Add the feature flag** in `Cargo.toml` and include it in `all`.
5. **Wire up the targets** — both are one line each, thanks to the macros:
   - `wasm/src/lib.rs`: `bind_engine!(compute_x_json, compute_x, XRequest, xfingine::x::compute)`
   - `python/src/lib.rs`: `bind_engine!(compute_x, compute_x_json, XRequest, ::xfingine::x::compute)`
     and register both in the `#[pymodule]` block.
   - Mirror the feature flag in `wasm/Cargo.toml` and `python/Cargo.toml`.
6. **Document it** in `README.md`, `wasm/README.md` and `python/README.md`.

Keep the bindings thin. They deserialize, call one core function, and serialize.
Any logic there is logic the other two targets silently miss.

## Testing Requirements

Tests are mandatory. Because the engines are pure maths with no PII, all
fixtures are committed to this repository — there is no external test-data repo.

1. **Snapshot test.** Add inputs to `tests/data/<engine>_cases.json` and record
   the output:
   ```bash
   UPDATE_EXPECTED=1 cargo test
   ```
   Commit both files. Follow the pattern in `tests/emi_integration.rs`:
   ```rust
   if std::env::var("UPDATE_EXPECTED").as_deref() == Ok("1") {
       fs::write(EXPECTED, serde_json::to_string(&actual).unwrap()).unwrap();
       return;
   }
   let expected: Vec<XResult> = serde_json::from_str(&fs::read_to_string(EXPECTED)?)?;
   assert_eq!(expected, actual);
   ```
2. **Invariant tests.** Assert what must hold whatever the numbers are — totals
   reconcile with rows, balances land exactly on zero, monotonic relationships
   stay monotonic. These catch what snapshots cannot.
3. **Unit tests** in `engine.rs` for the formulas themselves, the zero-rate
   degenerate cases, and every error path.

### Porting from the JavaScript tools

If you are porting an engine from `sakthipriyan.com`'s existing `.js` tools,
verify it **differentially**: run the original JavaScript and your Rust over the
same inputs and compare every field of every row, plus a few hundred randomized
cases. Totals agreeing is not enough — rounding bugs hide in the final row.

Where you deliberately diverge from the original, record it in `CHANGELOG.md`
with the reasoning, so it is not "fixed" back later.

## Pull Request Process

1. `cargo test --workspace --all-features` passes.
2. `cargo fmt --check` is clean.
3. `cargo clippy --workspace --all-targets --all-features -- -D warnings` is clean.
4. **Update `CHANGELOG.md`** under `## [Unreleased]`. CI fails any PR touching
   `src/`, `wasm/`, `python/`, `tests/` or `Cargo.toml` without one.
5. Open the PR against `main`.

## License

By contributing, you agree that your contributions will be licensed under the
Apache 2.0 License.
