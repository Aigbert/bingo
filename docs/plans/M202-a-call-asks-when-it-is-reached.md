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

Today the kernel does otherwise:
- **Every ask comes first.** `turn.rs` `gate` (~850-907) gates every call of the round before
  `execute` runs any. A round of `read, edit, bash` asks about the edit and the bash before the
  read has run.
- **Running is claimed early.** `gate` marks each allowed call `Running` while it still waits
  behind the asks.
- **A batch lands at once.** `executor.rs` `execute_chain` (~69-118) runs consecutive
  concurrency-safe calls with `join_all`. It reports their outcomes only when the whole batch has
  landed, though its doc (~48-49) says `on_done` sees each outcome as it lands.

So a step row can spin before its call runs, a fast read lands with the slowest one, and the
person is asked about a call whose turn has not come.

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
6. **The surface draws what was reached.**
   - A call the run has not reached is `Pending` with no open ask, and the TUI does not draw it.
     It appears when its ask opens (`◆ needs you`) or when it starts (the spinner), as a
     prototype step appears when it begins.
   - `--print`, the RPC, ACP and the channels change only in frame order.
7. **A stop says once when the person was asked** (handed on by M201's review).
   - Today the turn builds its Interruption with no `asked_at`, and `emit` stamps the journal's
     copy on its way in. The turn's outcome and the turn-end hooks hold `None`; the journal holds
     the time.
   - The actor sets that time before it cancels the turn's token, as the door's `completed` is
     set. The turn reads it when it builds its Interruption, and nothing rewrites an item on its
     way to the journal. A stop during a re-asked call's second ask still counts from that ask.

## Files

- `crates/bingo-core/src/{turn.rs,gate.rs,executor.rs}`, and the pure brick in a module of its
  own beside them (`round.rs`). For brick 7, `session.rs` and `session/{inputs,running}.rs`.
- `crates/bingo-surface-tui/src/turns.rs`, for the unreached call.
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
  - a stop during a permission ask, and one during a question: the turn's outcome, the
    turn-end hooks and the journal hold the ask's `opened_at`; with no ask open, none does.
- [ ] TUI: three calls where the second asks. The first row has landed, the second reads
      `needs you`, and the third is not drawn until it starts.
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
- **Old journals.** A session killed mid-round must not leave a `Pending` call that the TUI
  hides forever. Check how resume closes it.

## Verified

Round 1: [verification](../verification/m202/round-1.md). Round 2, 2026-10-19: kernel review fixes, [verification](../verification/m202/round-2.md).
- [x] Table before loop: `cargo test -p bingo-core --lib round --locked` → `17 passed; 0 failed`;
  contract commit `e290a457` precedes the walk. The original esc bug was pinned there too.
- [x] Kernel: `cargo test --workspace --locked` → `test result: ok. 703 passed; 0 failed`.
  Channels pin reach, asks, sliding admission, landings, esc, hooks, shared ask time and recovery.
- [x] Wire, same command: CLI `309 passed; 0 failed`, RPC `104 passed; 0 failed`, wire `43 passed`.
  Final full run: 6,522 passed, zero failed. No black-box re-pins; ACP fixture was present.
- [x] `cargo fmt --all -- --check`: exit 0. Workspace check and Clippy `-D warnings`: `Finished`.
  `scripts/check_discipline.sh`: `discipline ok`; `scripts/budget.sh`: `budget ok` (423 dependencies).
  `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.
- [x] `cargo check -p bingo-core --all-targets --target x86_64-pc-windows-msvc --locked`: `Finished`.
  The same check for `bingo` stops in aws-lc-sys/rquickjs-sys native cross-toolchain setup.
- One existing CLI recovery timeout passed alone unchanged, then passed in the full rerun.
  Budget retains a 7 GB debug-directory soft warning. Detailed outputs and risk audit are linked above.
- Departures: remove unused batch entry points; round/recovery own functions to meet cohesion limits.
  Accepted cost: a stop during a second ask counts from that ask. No prototype conflict found.
- [ ] Brick 6 and its TUI criterion remain for round 3 after M194 part B; both excluded crates are untouched.
