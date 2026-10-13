# M112 — One transcript fold

## Goal

ADR-0064 §1–4. One sdk `Transcript` owns the client items and model order.
`SessionState` embeds it; `ContextView` renders its model order; the turn
folds its own events through it. Starting a turn neither clones the journal
nor replays it. Model input and client wire bytes stay unchanged.

## Bricks, in build order

1. **Contract, failing.** Stub `Transcript::{new, apply, items, get,
   generation, model_items}`; property tests generate item start/update/
   completion, retry, rewind and known/unknown compaction boundaries with
   reused ids. Compare every prefix with scratch replay, the frozen client
   reducer and today's frame-based `ContextView::items`. No dropped id
   survives. Keep that model oracle under `cfg(test)` through M112.
   Add `proptest` as an sdk dev edge: already in the workspace and lockfile,
   no new external crate. Pin existing `SessionState` JSON bytes first.
2. **Transcript.** `Vec<Arc<Item>>`, an id/index map, generation, model ids
   in order and their membership index. Apply each compaction once with
   the existing splice; unknown model boundaries make no cut. Retry and
   rewind remove from both orders. Authoritative updates upsert by id.
   Serialize existing client fields; deserialize rebuilds client indexes.
3. **SessionState.** Flatten `transcript`, remove its own items/generation,
   delegate item events, and migrate every reader and fixture. Preserve the
   RPC schema and byte-equal snapshot round trips.
4. **Model view.** `ContextView::model_view(&Transcript)` renders model
   items, retaining existing external-tool filtering. Move the production
   splice into sdk; keep the independent test oracle and its splice.
   Slice-based hook/contributor/compactor APIs receive items on demand.
5. **Turn.** `TurnRun` takes a transcript snapshot. One `emit_and_fold`
   applies each event before `host.emit`; remove local upsert/splice/retain
   and redundant generation. `Running::spawn` clones no frames.
6. **Measure.** Exercise the same snapshot constructor as spawn with
   10,000 items. It accepts no journal/frames; pointer equality and strong
   counts prove shared, unchanged bodies. No timing assertion.
7. Keep ADR-0064 proposed for M120; date §1 as landed in M112
   and add the dated amendment note to ADR-0002 §3.

## Files

- `crates/bingo-sdk/src/{transcript.rs,transcript/*.rs,state.rs,lib.rs}`
- `crates/bingo-sdk/src/host.rs` (removed model-fold API doc reference)
- `crates/bingo-sdk/Cargo.toml`, workspace `Cargo.toml` and `Cargo.lock`
- `crates/bingo-core/src/{context.rs,turn.rs,turn/*.rs,rewind.rs}`
- `crates/bingo-core/src/turn/tests.rs`, `turn/tests/*.rs`
- `crates/bingo-core/src/session.rs`, `session/{running.rs,spawn.rs,title.rs}`
- Every reader of `SessionState::items` or generation, including plugin
  queries, surface fixtures, and integration tests (`rg -n '\.items\b' crates`)
- `docs/adr/0064-*.md`, `docs/adr/0002-event-stream.md`

## Exit criteria

- [x] Client/replay and model-oracle properties pass with ≥1,000 cases.
- [x] `rg -n 'ContextView::items|history: actor.journal' crates`: test code only.
- [x] `rg -n 'fn upsert|fn splice_compaction' crates`: one production rule
      each, in sdk; the frozen test oracle is the only other splice.
- [x] Existing `SessionState`/RPC fixture JSON is byte-equal; schema unchanged.
- [x] M108 retry and resume tests pass with their assertions unchanged.
- [x] All gates, including TUI smoke; Windows aws-lc-sys graphs are left
      to Windows CI under the final protocol step 7 decision (details below).

## Non-goals

Tree fold (M120), checkpoints/windowed memory or removing `Actor::journal` (M114), plugin API changes, and changing model input.

## Risks

- Client snapshots preserve the wire and do not seed a model: the model
  order comes from event replay. Keep copy-on-write deltas snapshot-safe.
- Re-verified against `72036be6` (M108): indexed retry removal is present;
  plugin APIs still take `&[Item]`. Adapt their call sites, not their traits.
- Option A amended ADR-0064 in `e28969f3` before code: two cuts require model order `[s2, b]`, not `[s2, b, s1]`; the named regression passes.
- Decision, 2026-10-13: leave Windows aws-lc-sys graphs to Windows CI; install no toolchain and continue local checks for the others.

## Verified (2026-10-13)

Branch `m112-one-transcript-fold`; no branch switch, push, merge or PR.
Commits: contract `8bc741aa`, fold `78fc2a6b`, integration `3932a35a`, fixes `648516e6`/`e3719b8f`.
Full workspace: 4,967 passed, 0 failed, 3 existing ignores; ACP tests ran
serially (`RUST_TEST_THREADS=1`). The PTY redraw race passed alone and in
that successful workspace run. Subsequent fixture-only discipline cleanup
passed the focused screen/RPC tests below. Logs are in `target/m112-*.log`.
Windows inverse graphs contain aws-lc-sys for `bingo-channels`,
`bingo-surface-print`, `bingo-surface-tui`, and `bingo`: left to Windows CI.
Channels previously failed on missing Windows C headers; no toolchain installed.
The other seven touched crates pass locally. Smoke uses a private tmux server
and HOME with SHELL=/bin/sh; the first default-shell attempt timed out on a
mise trust prompt before bingo started. The isolated rerun passed all 18 steps.

```text
$ cargo test -p bingo-sdk --offline transcript::tests -- --nocapture
# Stub: empty transcript instead of inserted items; snapshot length 0, not 10000.
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 91 filtered out; finished in 0.02s
$ cargo test -p bingo-core --locked context::tests::two_compactions_do_not_resurrect_the_first_summary -- --exact
# Stub model view is empty; the oracle contains s2, b.
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 411 filtered out; finished in 0.02s
$ RUST_TEST_THREADS=1 PROPTEST_CASES=1000 cargo test --workspace --locked
 test context::tests::transcript_model_matches_frame_oracle ... ok
 test context::tests::two_compactions_do_not_resurrect_the_first_summary ... ok
 test session::tests::retry::retry_withdrawal_survives_resume ... ok
 test session::tests::retry::retry_withdrawal_survives_the_next_turn ... ok
 test turn::tests::turn_start_shares_ten_thousand_bodies_without_a_journal ... ok
 test transcript::tests::transcript_matches_client_and_replay ... ok
 test state::tests::transcript_snapshot_keeps_the_wire_bytes ... ok
 test host::session_open_fixture_keeps_its_wire_bytes ... ok
 test result: ok. 413 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.66s
 test result: ok. 96 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.77s
$ cargo test -p bingo --test pty --locked esc_twice_rewinds_the_turn_and_the_file_it_wrote -- --exact --test-threads=1
 test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 15 filtered out; finished in 3.00s
$ cargo test -p bingo-surface-tui --locked screens::the_rewind_picker -- --exact
 test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1149 filtered out; finished in 0.08s
$ cargo test -p bingo --test rpc --locked
 test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.10s
$ cargo fmt --all -- --check
(no output; exit 0)
$ cargo check --workspace --all-targets --locked
Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.16s
$ cargo clippy --workspace --all-targets --locked -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.23s
$ scripts/check_discipline.sh
 discipline ok
$ scripts/budget.sh
 dependencies (unique, normal): 342 (max  342)
 warm cargo check -p bingo-core: 0s (max  20s)
 budget ok
$ cargo deny check
 advisories ok, bans ok, licenses ok, sources ok
$ cargo check -p bingo-sdk --all-targets --target x86_64-pc-windows-msvc
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.98s
$ cargo check -p bingo-core --all-targets --target x86_64-pc-windows-msvc
Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.11s
$ cargo check -p bingo-agents --all-targets --target x86_64-pc-windows-msvc
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.46s
$ cargo check -p bingo-checkpoints --all-targets --target x86_64-pc-windows-msvc --locked
Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.27s
$ cargo check -p bingo-context --all-targets --target x86_64-pc-windows-msvc --locked
Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.03s
$ cargo check -p bingo-rooms --all-targets --target x86_64-pc-windows-msvc --locked
Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.36s
$ cargo check -p bingo-surface-rpc --all-targets --target x86_64-pc-windows-msvc --locked
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.60s
$ cargo tree -p bingo-channels -i aws-lc-sys --target x86_64-pc-windows-msvc --locked
aws-lc-sys v0.44.0
# Same root for print, tui, bingo; the other seven report no matching package.
$ scripts/tui-smoke.sh
  a button on a pinned board fires its command and the table changes
tui-smoke ok
```
