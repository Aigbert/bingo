# M116 — Every task has an owner

## Goal

Implement ADR-0068 §1–4: esc reaches a hung turn, command or input gate;
plugin awaits leave the actor mailbox, and blocking journal I/O leaves runtime threads.

## Bricks, in build order

1. **Contracts and failing tests first.** Paused-clock cancellation tests in
   `session/tests/owned.rs`, with the existing `ScriptedProvider` and `test_support/` fakes:
   hung before-tool hook, policy, compactor ignoring its token, holding command,
   and submit hook. Assert interruption, closed items, released queues and responsive
   queries/another session. The store's deterministic current-thread/barrier test
   checks its blocking-pool primitive; a separate fake cannot test JSONL's implementation.
2. **`ErrorCode::Interrupted`.** Add `INTERRUPTED` (the person stopped it; not a failure),
   pin its serde spelling and both RPC/plugin schemas. M113's fixtures are absent here.
3. **Supervise the turn.** Race `run_turn` with cancellation followed by the 2 s
   `INTERRUPT_GRACE`, beside `AFTER_TURN_DEADLINE`. `Ended::{Returned,Panicked,Dropped}`
   replaces `Result` in `TurnFinished`. On drop, the actor closes every transcript
   item still open for that turn as interrupted, records the interruption marker,
   publishes `TurnCompleted` with folded usage, and sends that turn's items to
   after-turn hooks. Unit-test `Transcript::open_items(turn)`. Shutdown's
   `Running::abandon` stays unchanged; every closed item previously arrived by mail.
4. **Own commands.** Use the actor's `TaskTracker`, a root token, a child per run and
   an intent-to-token map. `CommandContext.cancel` permits cooperative cleanup;
   otherwise cancellation, grace, then drop rejects the intent `INTERRUPTED`.
   Close cancels the root and joins through `tracker.wait()` under `AFTER_TURN_DEADLINE`.
   Update every context literal mechanically, including plugin test constructors.
5. **Own the input gate.** `session/gate.rs` keeps a FIFO with one owned task in flight.
   It resolves commands, gathers/runs submit hooks and performs redirects, then mails
   a prose, command, rejected or redirected verdict. `run_queued` uses find-only work.
   Keep `busy()` semantics; gate waits do not block queries, turn frames, esc or close.
   Interrupt priority: turn, holding command, gate head, then `NOT_READY`.
   Reject a cancelled gate head `INTERRUPTED` and proceed in order. Preserve older
   ready queue entries when a turn ends during a later input's gate await.
6. **Offload store I/O.** Keep append and summary writes in one `spawn_blocking`
   closure, preserving per-frame reopen and serial actor appends. Offload create,
   replay and list too; all remaining file/lock operations follow the same rule.
   Promote workspace `tokio` from dev to production for `spawn_blocking`; no new
   external crate or budget increase. Document the blocking-pool rule on `SessionStore`.
7. **Correct the tool doc.** Replace the nonexistent `ToolTraits::interrupt` reference:
   esc drops every call in flight.
8. Accept ADR-0068 §1–4 (§5 remains M117), and date amendment notes on ADR-0002 §2
   and ADR-0008 §2/§7. Keep Actor inherent implementations within the three-file limit.

## Files

- `crates/bingo-core/src/session.rs`, `session/{running,commands,inputs,mailbox,
  queue,spawn,tests,gate}.rs`, `session/tests/{owned,storage}.rs`, `test_support/**`
- `crates/bingo-sdk/src/{error,command,tool,store,transcript}.rs`, `transcript/tests.rs`
- `schema/{rpc,plugin}.json`, `crates/bingo-surface-rpc/tests/wire/main.rs`
- `crates/bingo-store-jsonl/{src/lib.rs,Cargo.toml}`, `Cargo.lock` if needed
- Every `CommandContext {` constructor (`rg -n 'CommandContext \{' crates`)
- `docs/adr/{0068,0002,0008}*.md`

## Exit criteria

- [x] Six ownership/runtime regressions fail before their implementation and pass after.
- [x] Production session `tokio::spawn` sites are only actor and turn supervisor.
- [x] `inputs.rs` awaits no command lookup, hook or redirect.
- [ ] PTY smoke passes; real-provider esc during permission and compaction finishes within 2 s
      (not run: protocol stop below; a provider key is configured).
- [ ] All gates, including Windows checks for every touched crate (PTY failure; stop below).

## Non-goals

Plugin deadlines (M117), a bounded mailbox, batching or keeping the journal open.
Cooperative turns retain their existing behavior; supervision catches unresponsive work.

## Risks

- Dropped plugin work can leave a process or socket; M117 bounds that work.
- Re-verified at `01789d66`: M112's transcript and named methods exist; M113 fixtures do not.
- User decision, 2026-10-13: amend ADR-0068 §2 to TaskTracker, child tokens and the
  turn → holding command → gate head interrupt order; the initial stop is resolved.
- Queue ordering needs both slow-head and turn-ending-during-submit regressions.
- Record the 1,000-frame append measurement without a timing budget.

- **Stopped under protocol step 6, 2026-10-13.** The final workspace run failed two
  PTY tests; the picture test failed again when rerun alone at `pty/main.rs:734`
  (`a terminator`). It waits only for the transmission header before parsing the
  complete Kitty payload. A partial-read race is the likely cause; that test file
  is outside this plan's Files. The rewind test also sampled an incomplete redraw.
- Options: authorize a scoped fix in `crates/bingo/tests/pty/main.rs` to await complete
  transmission/redraw, or keep this branch pending a separate PTY fix. No choice is
  made here. ADR acceptance and its amendment notes remain pending verification.

## Verified (2026-10-13)

Implementation is committed; this milestone is **not complete**. Evidence below is
trimmed command output. Detailed local logs are under `target/m116/`.

```text
cargo test -p bingo-core session::tests::owned --locked   # supervision absent
  test result: FAILED. 0 passed; 5 failed; 0 ignored
cargo test -p bingo-store-jsonl runtime_contract --locked # inline I/O primitive
  test result: FAILED. 0 passed; 1 failed; 0 ignored
cargo test -p bingo-core a_turn_ending_during_submit --locked # before queue fix
  test result: FAILED. 0 passed; 1 failed; 0 ignored
cargo test -p bingo-core --locked                       # after fixes
  test result: ok. 422 passed; 0 failed; 0 ignored
cargo test -p bingo-store-jsonl --locked
  test result: ok. 40 passed; 0 failed; 1 ignored
cargo test -p bingo-sdk --locked
  test result: ok. 98 passed; 0 failed; 0 ignored
cargo test -p bingo-surface-rpc an_interrupted_intent --locked
  test result: ok. 1 passed; 0 failed; 0 ignored
```

The store's ignored test is the requested manual measurement, run explicitly with
`cargo test -p bingo-store-jsonl append_thousand_frames --locked -- --ignored --nocapture`:
`1000 appends: 297.202459ms` before; `1000 appends: 212.744959ms` after. No timing assertion.

```text
cargo fmt --all -- --check: exit 0
cargo check --workspace --all-targets --locked:
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.49s
cargo clippy --workspace --all-targets --locked -- -D warnings:
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 12.36s
```

An earlier full workspace run passed 5,012 tests. The final run used
`cargo test --workspace --locked -- --test-threads=1` with the two known ACP token
cases below excluded for separate execution. It stopped at the PTY binary:

```text
FAILED: a_picture_an_answer_named_in_markdown_is_read_in_and_sent
FAILED: esc_twice_rewinds_the_turn_and_the_file_it_wrote
test result: FAILED. 14 passed; 2 failed; 0 ignored
cargo test -p bingo --test pty --locked a_picture_an_answer_named_in_markdown_is_read_in_and_sent -- --exact --test-threads=1
  panicked at crates/bingo/tests/pty/main.rs:734:10: a terminator
  test result: FAILED. 0 passed; 1 failed; 0 ignored
```

The earlier isolated runs both passed (`--locked`, `--exact --test-threads=1`):
`cargo test -p bingo --test acp_bridge a_proxy_with_a_token_this_run_never_minted_gets_nothing`
and `cargo test -p bingo-provider-acp --test bridge a_token_may_be_dialled_again_once_its_stream_has_closed`:
`test result: ok. 1 passed; 0 failed; 0 ignored` each. No silent skips.

Production `rg -n 'tokio::spawn' crates/bingo-core/src/session --glob '!**/tests/**'`:
`running.rs:69: let task = tokio::spawn(async move {`; `spawn.rs:112: tokio::spawn(actor.run());`.
Review of `rg -n '\.await' crates/bingo-core/src/session/inputs.rs` found only actor operations;
`rg -n 'commands.*find|hooks\.(at|gather)|on_submit|host\.deliver'` there: no matches.
Earlier discipline rerun: `discipline ok` after replacing extra Actor impls with module functions.
Dependency check: `dependencies (unique, normal): 345 (max 345)`; `budget ok`.
Final discipline/budget reruns, deny, Windows checks, PTY smoke and real-provider checks
remain unrun after the repeated PTY failure. No toolchain installed; no live user data touched.
