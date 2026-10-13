# M116 — Every task has an owner

## Goal

ADR-0068 §1–4. One esc ends the turn however the turn is stuck. A
command is cancelled and joined like a turn. The actor's mailbox never
waits on a plugin. Journal I/O leaves the runtime threads. Today each of
these holds only where a `select!` was remembered.

## Bricks, in build order

1. **Failing tests first.** All on the paused clock (`#[tokio::test(
   start_paused = true)]`; bingo-core's dev tokio has `test-util`), in
   `crates/bingo-core/src/session/tests/owned.rs`, using
   `bingo-provider-fake` and fakes in `test_support/`. Each sends
   `Interrupt`, advances the clock by `INTERRUPT_GRACE` plus 1 ms, and
   asserts `TurnCompleted { Interrupted }`. Every item the turn opened
   must read `Interrupted` in `SessionState`. The fakes:
   - a `before_tool` hook that never returns;
   - a `PermissionPolicy::decide` that never returns;
   - a compactor that never returns and ignores its token (trigger
     compaction with a tiny window, as the existing compaction tests
     do);
   - a `Command` that never returns and holds the queue. Interrupt with
     no turn running: its intent is rejected `INTERRUPTED`, the queued
     prose behind it starts a turn;
   - an `on_submit` hook that never returns. While it hangs, a
     `Query`/state request is answered and a second session on the same
     runtime still streams. Interrupt rejects the gated input
     `INTERRUPTED`;
   - a store fake whose `append` blocks its thread on a
     `std::sync::Barrier`: a second session on a `current_thread`
     runtime still completes a turn (passes once brick 6's rule holds).
2. **`ErrorCode::Interrupted`.** Additive (`INTERRUPTED`), with a doc
   line ("the person stopped it; not a failure"). Pin it in `error.rs`'s
   wire test and `schema/rpc.json`; M113's fixtures are not in this tree.
3. **The supervised turn.** `session/running.rs`:
   - `INTERRUPT_GRACE: Duration = 2 s`, beside `AFTER_TURN_DEADLINE`.
   - The task races `run_turn` against
     `cancel.cancelled().then(sleep(INTERRUPT_GRACE))`. If the timer
     wins, the turn future is dropped where it stands.
   - `Msg::TurnFinished.outcome` becomes an enum `Ended { Returned(
     TurnOutcome), Panicked(String), Dropped }`. It replaces today's
     `Result<TurnOutcome, String>`; `turn_finished` matches on it.
   - On `Dropped` the actor closes the turn itself, in a function of its
     own (`close_dropped_turn`):
     - publish `ItemCompleted` with status `Interrupted` for every item
       of that turn the transcript still shows open (a
       `Transcript::open_items(turn)` brick, unit-tested in the sdk);
     - record the `Interruption` marker, as `interrupted_status` does
       in the turn;
     - publish `TurnCompleted { Interrupted { UserCancel } }` with the
       usage the state holds;
     - pass that turn's items, read from the transcript, to
       `after_turn`.
   - Every item reached the actor by mail; nothing is invented.
     `Running::abandon` (shutdown) is unchanged.
4. **Commands are owned.** `session/commands.rs`:
   - `Commands::new` takes the actor's `TaskTracker` and a root
     `CancellationToken`. Each run gets a child token and runs on the
     tracker, raced like the turn (token, then grace, then drop).
   - `CommandContext` gains `cancel: CancellationToken`, as
     `ToolContext` has, so a command that listens stops early.
   - `Commands` keeps `intent → token`. `interrupt` (`inputs.rs`) with
     no turn cancels the holding command; its `CommandFinished` carries
     `Err(INTERRUPTED)`. Close cancels the root token; the existing
     `tracker.wait()` under `AFTER_TURN_DEADLINE` joins them.
5. **The actor awaits no plugin.** A new `session/gate.rs` owns the
   input gate:
   - Inputs that need plugin work wait in a FIFO. Only the head is in
     flight, on an owned task (tracker plus child token). The task does
     `commands.find`, the submit hooks (`hooks.at(Submit)` and each
     `on_submit`), and a redirect's `host.deliver`.
   - It mails back `Msg::Gated { intent, verdict }`, where `verdict` is
     one of: prose `Input`, a found command, rejected `(code, message)`,
     or redirected `Result`. The actor acts on it as `submit_prose` /
     `submit_command` / `redirect` do today; those functions shrink to
     the acting half.
   - While the gate is non-empty, `busy()` is unchanged. A new input
     joins the gate's FIFO, so arrival order is kept without holding the
     mailbox.
   - `run_queued`'s `find` goes through the gate as a find-only verdict.
   - Interrupt with no turn and no holding command cancels the gate's
     head. Its intent is rejected `INTERRUPTED`.
   - Queries, frames from the turn, `Interrupt` and `Close` are never
     behind the gate.
6. **Journal I/O off the runtime.** `bingo-store-jsonl/src/lib.rs`:
   - `append` moves `journal::append` and the summary writes into one
     `spawn_blocking` closure. `replay`, `list` and `create` do the
     same.
   - The per-frame reopen stays (`journal.rs` says why).
   - A session's appends are already serial (the actor awaits each).
   - The `SessionStore` doc gains the rule: "an implementation does
     blocking I/O on the blocking pool".
7. **The stale doc.** `bingo-sdk/src/tool.rs` (the `ToolTraits` doc,
   ~164) names an `interrupt` property that does not exist. Restate it
   as AGENTS.md does: esc drops every call in flight.
8. ADR-0068 accepted (§1–4; §5 is M117's). Dated notes on ADR-0002 §2
   (the turn is supervised) and ADR-0008 §2 (the actor owns the
   command's task; the input gate).

## Files

- `crates/bingo-core/src/session.rs`, `session/{running,commands,inputs,
  mailbox,spawn,tests}.rs`, `session/gate.rs` (new),
  `session/tests/{owned,storage}.rs`, `test_support/**`
- `crates/bingo-sdk/src/{error.rs,command.rs,tool.rs,store.rs,
  transcript.rs}`, `transcript/tests.rs`, `schema/rpc.json`
- `crates/bingo-store-jsonl/src/lib.rs`
- every `CommandContext {` constructor outside the kernel (production and tests):
  `rg -n 'CommandContext \{' crates`
- `docs/adr/{0068,0002,0008}*.md`

## Exit criteria

- [ ] The six tests in brick 1 fail on the tree before this milestone
      and pass after (both runs pasted).
- [ ] `rg -n 'tokio::spawn' crates/bingo-core/src/session` lists only
      the actor's own (`spawn.rs`) and `Running::spawn` (pasted).
- [ ] `rg -n '\.await' crates/bingo-core/src/session/inputs.rs` shows no
      call into `commands.find`, a hook, or `host.deliver` (pasted).
- [ ] `scripts/tui-smoke.sh` passes. A tmux drive of esc during a
      permission prompt and during compaction, with a real provider,
      ends the turn within 2 s (notes pasted; skip the provider leg if
      no key is configured, and say so).
- [ ] All gates, Windows target check included.

## Non-goals

- Deadlines inside plugins (M117). A bounded mailbox (ADR-0068 §6).
  Keeping the journal file open. Batching appends.
- Changing what a turn does when it honours its token; the grace only
  catches the ones that do not.

## Risks

- Dropping may leave plugin work behind (M117); the kernel promises only that the session moves on.
- Test prose/command/prose during a slow submit hook: acknowledgements and turns keep arrival order.
- Update all `CommandContext` literals, including production `bingo-agents/src/team/command.rs`.
- Record a 1 000-frame append benchmark before and after the blocking-pool change (not a budget).
- Re-verification at `01789d66` (2026-10-13): M112's `Transcript` and the named session methods
  exist; M113's fixtures do not. The actor already has a `TaskTracker`; commands still use an
  untracked `tokio::spawn`.
- **Stopped under execution protocol step 6, before implementation.** ADR-0068 §2 requires
  `NOT_READY` with neither a running turn nor a holding command; brick 5 requires cancellation of
  the gate head then. Options: amend §2 to allow gate cancellation, or retain §2 and revise brick 5
  and its submit-hook interruption test. Neither is chosen here.
- ADR-0068 §2 prescribes command join handles on a `JoinSet`; brick 4 prescribes the actor's
  `TaskTracker` and token map. Resolve by aligning the plan to `JoinSet`, or explicitly amending the
  ADR to `TaskTracker`. No ADR Decision was edited, and no implementation gates were run.
