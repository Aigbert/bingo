# M202 — A call asks when it is reached

ADR-0002 (one stream), ADR-0074 (every dispatch is a chain), AGENTS.md (one esc ends the turn).
Spec: `docs/design/ember/` (`p4-model.js` `askBeat`, `pauseFor`, `resume`, `stopTurn`;
`p4b-data.js` `answeredBy`). After: M201 for bricks 1-5 and 7, and M194 part B for brick 6.
This changes when bingo asks; the person left that call to the plan (2026-10-18).

## Goal

A round runs its calls in order, the way the prototype draws them:
- a call begins when the run reaches it;
- a call that needs a person asks then, after everything before it has landed, and the turn
  waits there;
- each call lands when it finishes.

Before this milestone, every call was gated before execution and marked Running too early.
Concurrency-safe batches landed together. The kernel rounds fixed reach and landing order;
round 3 makes each call own its wait and makes the surface draw only reached work.

## Bricks

1. **The round's order** (a pure brick, tests first).
   - Input: the calls in order, and for each reached call its gate outcome: allowed and
     concurrency-safe, allowed and not, denied, or needs a person. Also what is in flight.
   - Output: the next action: start the call now, wait for the next landing, record the denial
     now, ask a person now, or the round is done.
   - A call is reached when the walk gets to it. That means the call before it has started or
     been denied, and no call that is not concurrency-safe is in flight. A reached call that
     waits holds the walk. It is decided (hooks, then the policy) when it is reached.
   - The table, by the reached call's outcome:
     - allowed, concurrency-safe: start it while everything in flight is concurrency-safe and
       fewer than `MAX_CONCURRENCY` are in flight, else wait for a landing;
     - allowed, not concurrency-safe: start it when nothing is in flight, else wait;
     - denied with nobody asked: record the denial now;
     - needs a person: ask when nothing is in flight, else wait;
     - none left: wait for each landing, then the round is done.
   - The table of cases is the test, written before the loop that uses it.
2. **The gate decides before it asks.**
   - `gate.rs` returns the decision (hooks and the policy, `check`) before anyone is asked.
     Asking is a separate step that the round calls when brick 1 says so.
   - The change-first loop (`recheck`, `Reply::Changed`) stays inside the asking step. After a
     change it checks the call again, with nothing in flight by then.
3. **One loop for the round.**
   - `turn.rs` `gate` and `execute` become one walk over the calls, driven by brick 1:
     - gate each call when it is reached;
     - before an ask, wait for the calls in flight to land;
     - record the receipt when the ask closes.
   - A rule added by "yes for this session" therefore answers a later call in the same round.
   - The before-tool hooks are still gathered once per round.
   - The `ItemUpdated` that marks a call `Running` is emitted at the instant it starts. The reason
     M201 writes there moves with it.
4. **Each call lands when it lands.**
   - `execute_chain` reports each outcome as it completes, for example from a
     `FuturesUnordered`. It returns the outcomes in input order.
   - The model sees the same results in the same order as today.
5. **One esc, as before.**
   - The calls in flight are dropped and become `Interrupted`.
   - An open ask closes.
   - Calls the run never reached become `Interrupted` with no gate and no receipt, as
     `interrupted` does today.
   - The after-tool hooks see the round once.
   - Every await in the new loop races the cancel token (`racing`).
6. **The surface draws what was reached** (prototype `stopTurn`, `deny`).
   A call is drawn once it asks or runs: from its `askedAt` or `beganAt` (brick 8), or an
   open ask on it. A `Pending` call with none is not reached and is not drawn. It reads
   `needs you` while its wait is open; its spinner and clock count from `beganAt`.
   It reads `said no` after a Deny, and `stopped` when a stop closes a call that asked or ran.
   A call stopped before it was reached is not drawn. Rows keep call order. Print, RPC,
   ACP and channels change only in frame order and the new fields.
7. **A stop says once when the person was asked.** The actor shares the earliest open ask
   before cancelling; the turn, hooks and journal read the same value without a rewrite.
   An ask a call holds (brick 8) leaves its time to the call: its receipt and the Interruption
   carry no `asked_at`. A re-asked call counts its wait from its first ask.
8. **A call says when it was asked and when it began.** Optional `askedAt` and `beganAt`,
   contract first, preserve old journals. The round stamps its own calls; bridged calls use
   `CallAsked` and `CallAllowed`; an agent call's `beganAt` is its start. One sdk `call_wait`
   reads the wait. Questions, tool-host and judge asks keep their own `asked_at`.

## Files

- `crates/bingo-core/src/{turn.rs,gate.rs,executor.rs}`, and the pure brick in a module of its
  own beside them (`round.rs`). For brick 7, `session.rs` and `session/{inputs,running}.rs`.
- SDK `event.rs`, `transcript/clock.rs`, contract/clock tests and generated schemas/types.
- Core `gate{,/asking}.rs`, `turn/{round,agent}.rs`, `session/{inputs,invoke,mailbox,interactions}.rs`.
- TUI `turns.rs`, `transcript{,/steps,/slots,/band}.rs`, `component/live.rs`, tests.
- Crew `cards.rs`; `docs/verification/m202/{drive.sh,round-3.md}`.
- Core turn and executor tests. RPC, print and ACP black-box tests where they pin frame order.

## Exit criteria

- [x] Brick 1's table tests land before the loop.
- [x] Core tests, with tool doubles released by channels, never by the wall clock:
  - `read, edit (asks), bash (asks)`:
    - the read lands before the first ask opens;
    - the bash's ask opens only after the edit lands;
  - "yes for this session" on the first of two edits in one round: the second runs without
    asking;
  - two concurrency-safe reads, the second released first: its `ItemCompleted` comes first;
  - no call is `Running` before it starts, and a call waiting behind an ask stays `Pending`;
  - esc during an ask: the ask closes, the unreached calls are `Interrupted`, and they have no
    receipt;
  - esc during a parallel batch: as today;
  - the after-tool hooks see the whole round once;
  - a stop during a question: outcome, turn-end hooks and journal Interruption hold its
    `opened_at`. During a call permission ask, the call holds `askedAt`, Interruption none.
    With no ask open, none does.
- [x] TUI: three calls where the second asks. The first row has landed, the second reads
      `needs you`, and the third is not drawn until it starts. Stop during that ask: the
      second reads `stopped`, the third is never drawn. Evidence: TUI tests and drive.sh.
- [x] ToolCall contract: round trip, old fixture, regenerated schemas and TypeScript.
- [x] Turn clock never jumps across an ask: frame-by-frame literal tests.
- [x] RPC, print and ACP black-box tests pass. Each test re-pinned for frame order is named in
      Verified.
- [x] Gates, one at a time, and the Windows check for `bingo-core`.

## Non-goals

- Which tools are concurrency-safe, and `MAX_CONCURRENCY`.
- Asking about several calls at once.
- What the policy decides; only when it is consulted changes.

## Risks

- **Hooks.** A before-tool hook now runs while earlier calls are in flight. Check ADR-0074's
  chain for anything that assumed nothing was running.
- **Teammates.** A child session runs the same loop. The parent's `◆ reviewer needs you`
  (M200) must still come from the child's open ask.
- **esc latency.** The new loop has more awaits. One that does not race the cancel token
  brings back the 2026-09-09 bug.
- **Old journals.** Recovery keeps stamps; an old interrupted call with neither is hidden.
- **ItemUpdated.** Clients must not read a stamp-only update as activity; audit each client.
- Tool-host and judge asks still have no call wait; follow-up, outside this round.

## Verified

Rounds 1–2b: [round 1](../verification/m202/round-1.md), [round 2 and 2b](../verification/m202/round-2.md).
Round 3 details, audits, re-pins and departures: [round 3](../verification/m202/round-3.md).
- [x] Contract/clock: SDK why_and_time `2 passed`, call_wait `3 passed`, turn_clock `12 passed`.
- [x] Kernel stamps, held receipts/stops and recovery: core `713 passed; 0 failed`.
- [x] Reached rows/crew: TUI `1431 passed`, crew `28 passed`; no snapshot changed.
- [x] `bash docs/verification/m202/drive.sh`: 100×32/80×24 passed; `scripts/tui-smoke.sh`: `tui-smoke ok`.
- [x] fmt/check/clippy/test/discipline/budget/deny: all passed; workspace `6567 passed`.
- [x] Windows sdk/core/crew passed; TUI and other native stops are named in round 3.
- [x] Per-commit workspace checks: 37 passed, folded helper Clippy passed, HEAD unchanged.
- Departures: box optional output for enum size; missing-tool policy/message; RPC schema generator.
- Tool-host/judge waits and mid-call recovery timing remain follow-ups; old unstamped stops hide.
