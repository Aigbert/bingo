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
- [x] Fake-provider paused-time tests prove esc completes within 2 s during hung policy/compaction.
- [x] PTY smoke passes in a harness PTY.
- [ ] Real-provider esc during permission and compaction finishes within 2 s.
      User will drive it: needs credentials and spends money; not run by instruction.
- [x] Gates run; isolated PTY reruns resolve the failures; Windows header failures go to CI.

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

- The PTY stop is resolved: picture tests decoded `a=T` before its terminator on
  a loaded machine. User-authorized cherry-pick `27e55202` → `bf499d45` waits for
  whole transmissions (`crates/bingo/tests/pty/main.rs`); user reports 3/3 isolated passes.
- Real-provider esc needs credentials and spends money; the user will drive it.
  Verify the 2 s bound using the fake provider and paused Tokio time instead.

## Verified (2026-10-13)

Implementation is committed; real-provider esc remains pending by user instruction.
Trimmed output follows; detailed local logs are under `target/m116/`.

```text
cargo test -p bingo-core session::tests::owned --locked # supervision absent
  FAILED. 0 passed; 5 failed
cargo test -p bingo-store-jsonl runtime_contract --locked # inline I/O
  FAILED. 0 passed; 1 failed
cargo test -p bingo-core a_turn_ending_during_submit --locked # before queue fix
  FAILED. 0 passed; 1 failed
cargo test -p bingo-core --locked: 422 passed; 0 failed
cargo test -p bingo-store-jsonl --locked: 40 passed; 0 failed; 1 ignored
cargo test -p bingo-sdk --locked: 98 passed; 0 failed
cargo test -p bingo-surface-rpc an_interrupted_intent --locked: 1 passed; 0 failed
cargo test -p bingo-core session::tests::owned --locked: 9 passed; 0 failed
cargo fmt --all -- --check: exit 0
cargo check --workspace --all-targets --locked: Finished `dev` profile in 4.49s
cargo clippy --workspace --all-targets --locked -- -D warnings: Finished in 12.36s
scripts/check_discipline.sh: discipline ok
scripts/budget.sh: dependencies (unique, normal): 345 (max 345); budget ok
cargo deny check: advisories ok, bans ok, licenses ok, sources ok
```
The hung-hook, policy and compactor tests assert paused Tokio time from esc to
`TurnCompleted` is ≤2 s, without advancing time before starting the bounded wait.
The store's ignored manual measurement was explicitly run:
`cargo test -p bingo-store-jsonl append_thousand_frames --locked -- --ignored --nocapture`:
`1000 appends: 297.202459ms` before; `212.744959ms` after. No timing assertion.

An earlier full workspace run passed 5,012 tests. The final
`cargo test --workspace --locked -- --test-threads=1` (two ACP cases isolated below)
stopped at PTY: `FAILED. 14 passed; 2 failed` (picture terminator, partial rewind redraw).
Picture failed again alone (`a terminator`), triggering the recorded stop.
User identified the partial-transmission race, supplied `27e55202` → `bf499d45`,
and reports the picture test passed 3/3 alone on this branch.
The isolated ACP runs passed (`--locked -- --exact --test-threads=1`):
`cargo test -p bingo --test acp_bridge a_proxy_with_a_token_this_run_never_minted_gets_nothing`
and `cargo test -p bingo-provider-acp --test bridge a_token_may_be_dialled_again_once_its_stream_has_closed`:
`test result: ok. 1 passed; 0 failed` each. No silent skips.

Production `rg -n 'tokio::spawn' crates/bingo-core/src/session --glob '!**/tests/**'`:
`running.rs:69` (turn supervisor); `spawn.rs:112` (actor).
Review of `rg -n '\.await' crates/bingo-core/src/session/inputs.rs`: only actor operations;
`rg -n 'commands.*find|hooks\.(at|gather)|on_submit|host\.deliver'` there: no matches.

`cargo check -p <crate> --all-targets --locked --target x86_64-pc-windows-msvc`:
PASS: bingo-{agents,checkpoints,context,core,demo-ui,experience,rooms,schedule,sdk,
skills,store-jsonl,surface-rpc,tasks,tool-bash} (14 crates).
Windows CI: bingo, bingo-mcp, bingo-plugin-rpc → aws-lc-sys; bingo-permissions → tree-sitter.
Only missing Windows C headers blocked those four checks; no Rust errors, no toolchain installed.

`cargo test -p bingo --test pty --locked esc_twice_rewinds_the_turn_and_the_file_it_wrote -- --exact --test-threads=1`:
`test result: ok. 1 passed; 0 failed` (the earlier redraw failure passes alone).
`PATH="$PWD/target/m116/bin:$PATH" scripts/tui-smoke.sh` in a harness PTY: `tui-smoke ok`.
The tmux wrapper only maps the smoke socket to a unique M116 name, isolating other runs.
ADR-0068 §1–4 accepted; §5 remains proposed for M117. ADR-0002 §2 and ADR-0008 §2/§7 dated.
Real-provider esc was not run: credentials and spend require the user's own check.
