# Agent guidance for this repo

Rust learning repo (edition 2024): concurrency primitives (locks, consensus, universal construction) and memory-model experiments, each verified under both `std` and Loom (Tokio's exhaustive relaxed-memory model checker). Tests import concurrency types from `concur::{thread, sync::Arc, sync::atomic::{AtomicUsize, Ordering, fence}}` so the same code compiles against `std` or `loom` depending on the feature.

## Commands

- `cargo test` — all tests with standard Rust concurrency
- `cargo loom-test` — same tests under Loom exhaustive model checking (alias: `cargo test --features check-loom --release`, defined in `.cargo/config.toml`)
- CI (`.github/workflows/ci.yml`) additionally runs `cargo check`, `cargo build`, and `cargo test --release`

## Loom gating

- `src/lib.rs` re-exports `std::*` or `loom::*` based on the `check-loom` feature; `concur::model(f)` wraps `loom::model(f)` when the feature is active, otherwise runs `f()` directly.
- `src/lib.rs` also exposes `cell_read`/`cell_write` helpers that hide the `UnsafeCell` access differences between std and loom — use them instead of touching `.get()` when a `UnsafeCell` field must be readable in both modes.

## Test layout

- `tests/promising.rs` — 4 modules: `multi_valued_memory`, `message_adjacency`, `views`, `promises` (the `promises` module tests store hoisting scenarios).
- `tests/lock.rs`, `tests/consensus.rs`, `tests/universal.rs` — each follows the same split: a std-only `basic` module gated with `#[cfg(not(feature = "check-loom"))]` (heavy: up to 20 threads × 500 ops) and an ungated `correctness` module that runs under both std and Loom with small n (2 threads, `C::default()`).

## Universal construction (`src/universal.rs`, `tests/universal.rs`)

- The replay loop in `LFUniversal::apply` must **not** downcast intermediate nodes' `Box<dyn Any>` results to the current call's `R` — callers may use different return types (e.g. `push` returns `()`, `pop` returns `Option<T>`). It invokes the stored closure and discards the result; only the final own-node result is downcast to `R`.
- Memory ordering: happens-before to all chain-node creators is **transitive** — every thread reads `head` with `Acquire` (`Node::max`'s comparison loads and its final load) and publishes its appended node with `Release`; consensus losers synchronize with the winner via the `cas.load(Ordering::Acquire)` fallback in `CasConsensus::decide` (`src/consensus/cas.rs:42`). This chain makes the non-atomic `invoc` reads safe and covers the `next` links, so `next`/`help.seq` may stay `Relaxed`. Do **not** weaken: the `head`/`announce` `Release` stores, the `Acquire` loads in `Node::max` (the final load also covers the case where the head entry changed between the comparison and the final read), or the consensus fallback `Acquire`. Note `WFUniversal` additionally uses `Acquire` on its own `new.seq` load and `Release` on `after.seq` store as part of its synchronization — keep those too.
- `WFUniversal`'s helping must **not** propose the sentinel: `announce[]` is initialized to the sentinel, whose `seq == 1` is indistinguishable from the "unchained" marker — guard with `help != self.tail` (without it, the sentinel gets chained into the chain, overwriting its own `seq`/`next` and corrupting the chain so replays miss nodes).
- The `correctness::{lf,wf}_universal` Loom tests are **intentionally minimal** (2 threads, one `push(0)` and one `pop` apply each; the `pop` asserts `item == 0` when `Some`, and the `None` empty-stack path is valid depending on chain order). The retry loops explode Loom's state space (default exploration is unbounded — `max_duration`/`max_permutations` are `None`); 2×2-apply tests never terminate, and even the minimal `wf_universal` takes ~80s. Do not enlarge without bounding Loom (e.g. `Builder`/`LOOM_MAX_DURATION`).

## Store hoisting tests

`store_hoisting_wo_dep` and `store_hoisting_syntactic_dep` record into an external `reached` flag but their `assert!(reached...)` is commented out — they always pass with `assert!(true)`. Do not uncomment blindly; Loom does not model store hoisting, so these executions are unreachable under Loom. `store_hoisting_w_dep_oota` and `store_hoisting_syntactic_dep_rw_coherence` assert the negative (that `r1=r2=1` / `r1=r2=r3=1` is impossible) and work correctly under Loom without the `reached` pattern.

Do not confuse the two commented-out tests with `load_hoisting` (`multi_valued_memory`) and `relaxed_no_sync` (`views`): those use the same `reached` flag but **do** assert it under `check-loom` (they affirm that a relaxed execution can reach `r1=r2=0` / both-read-1), which Loom can model.
