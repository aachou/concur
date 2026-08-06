# Agent guidance for this repo

## Commands

- `cargo test` — run all tests with standard Rust concurrency
- `cargo loom-test` — run tests under Loom exhaustive model checking (alias: `cargo test --features check-loom --release`, defined in `.cargo/config.toml`)
- Python (if needed) — installed via `uv`; run scripts with `uv run python`

## Loom gating

`src/lib.rs` conditionally re-exports `std::*` or `loom::*` based on the `check-loom` feature. `relaxed_memory_concurrency::model(f)` wraps `loom::model(f)` when `check-loom` is active, otherwise runs `f()` directly.

Tests import from `relaxed_memory_concurrency::{thread, sync::Arc, sync::atomic::{AtomicUsize, Ordering, fence}}` — the crate-level re-export in `src/lib.rs` makes these resolve to `std` or `loom` equivalents depending on the feature flag.

## Test layout

All integration tests live in `tests/promise_semantics.rs` with 4 modules: `multi_valued_memory`, `message_adjacency`, `views`, `promises`. The `promises` module tests store hoisting scenarios.

## Store hoisting tests

`store_hoisting_wo_dep` and `store_hoisting_syntactic_dep` use an external `reached` flag whose assertion is commented out — they always pass with `assert!(true)`. Do not uncomment blindly; Loom does not model store hoisting, so these executions are unreachable under Loom. `store_hoisting_w_dep_oota` and `store_hoisting_syntactic_dep_rw_coherence` assert the negative (that `r1=r2=1` / `r1=r2=r3=1` is impossible) and work correctly under Loom without the `reached` pattern.
