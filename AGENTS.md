# Agent guidance for this repo

## Commands

- `cargo test` — run all tests with standard Rust concurrency
- `cargo loom-test` — run tests under Loom exhaustive model checking (alias: `cargo test --features check-loom --release`, defined in `.cargo/config.toml`)
- Python (if needed) — installed via `uv`; run scripts with `uv run python`

## Loom gating

`src/lib.rs` conditionally re-exports `std::*` or `loom::*` based on the `check-loom` feature. `relaxed_memory_concurrency::model(f)` wraps `loom::model(f)` when `check-loom` is active, otherwise runs `f()` directly.

Tests import from `relaxed_memory_concurrency::{thread, sync::Arc, sync::atomic::{AtomicUsize, Ordering, fence}}` — the crate-level re-export in `src/lib.rs` makes these resolve to `std` or `loom` equivalents depending on the feature flag.

## Test layout

Integration tests live in `tests/promise_semantics.rs` (4 modules: `multi_valued_memory`, `message_adjacency`, `views`, `promises`; the `promises` module tests store hoisting scenarios), plus `tests/lock.rs`, `tests/consensus.rs`, and `tests/universal.rs`. The consensus tests have a `basic` module (std-only, 20 threads) and a `correctness` module (std + Loom, n=2 via `C::default()`).

## Universal construction (`tests/universal.rs`, `src/universal.rs`)

- The replay loop in `LFUniversal::apply` must **not** downcast intermediate nodes' `Box<dyn Any>` results to the current call's `R` — callers may use different return types (e.g. `push` returns `()`, `pop` returns `Option<T>`). It invokes the stored closure and discards the result; only the final own-node result is downcast to `R`.
- Memory ordering: happens-before to all chain-node creators is **transitive** — every thread reads `head` with `Acquire` (`Node::max`'s comparison loads and its final load) and publishes its appended node with `Release`; consensus losers synchronize with the winner via the `cas.load(Ordering::Acquire)` fallback in `CasConsensus::decide`. This chain makes the non-atomic `invoc` reads safe and covers the `seq`/`next` accesses, so `next`/`seq`/`help.seq` may stay `Relaxed`. Do **not** weaken: the head/announce `Release` stores, the `Acquire` loads in `Node::max` (the final load also covers the case where the head entry changed between the comparison and the final read), or the consensus fallback `Acquire`.
- `WFUniversal`'s helping must **not** propose the sentinel: `announce[]` is initialized to the sentinel, whose `seq == 1` is indistinguishable from the "unchained" marker — guard with `help != self.tail` (without it, the sentinel gets chained into the chain, overwriting its own `seq`/`next` and corrupting the chain so replays miss nodes).
- The `correctness::{lf,wf}_universal` Loom tests are **intentionally minimal** (2 threads, one `push(0)` and one `pop` apply each; the `pop` asserts `item == 0` when `Some`, and the `None` empty-stack path is valid depending on chain order). The retry loops explode Loom's state space (default exploration is unbounded — `max_duration`/`max_permutations` are `None`); 2×2-apply tests never terminate, and even the minimal `wf_universal` takes ~80s. Do not enlarge without bounding Loom (e.g. `Builder`/`LOOM_MAX_DURATION`).

## Store hoisting tests

`store_hoisting_wo_dep` and `store_hoisting_syntactic_dep` use an external `reached` flag whose assertion is commented out — they always pass with `assert!(true)`. Do not uncomment blindly; Loom does not model store hoisting, so these executions are unreachable under Loom. `store_hoisting_w_dep_oota` and `store_hoisting_syntactic_dep_rw_coherence` assert the negative (that `r1=r2=1` / `r1=r2=r3=1` is impossible) and work correctly under Loom without the `reached` pattern.
