# M108 — The model sees what the person sees; the journal is written first

## Goal

Two kernel facts that are false today (audit-2026-10, "Bugs that do not
wait"), fixed without new architecture:

1. Text a retry withdrew comes back. The turn drops a retry's items
   (`turn/stream.rs`, `self.items.retain`) and `SessionState` drops them
   (`bingo-sdk/src/state.rs`, `turn_retrying`), but the next turn
   rebuilds its items with `ContextView::items(&run.history)`
   (`turn.rs`), which has no `TurnRetrying` arm
   (`bingo-core/src/context.rs`). The model is sent the half answer the
   person saw disappear — on the next turn and after every resume.
2. A frame the journal failed to take is published anyway.
   `Actor::publish` (`bingo-core/src/session.rs`) logs "journal append
   failed" and still pushes, applies and fans out, against ADR-0002 §2
   ("durable frames are appended to the store before being published").
   A client then holds a session that a resume will not reproduce.

ADR-0064 later removes the third fold; this milestone fixes the
behaviour and pins it with tests that will survive that refactor.

## Bricks, in build order

1. **Failing tests first.**
   - `context.rs` unit: a frame sequence with `ItemStarted`/`ItemUpdated`
     for an assistant text, then `TurnRetrying { dropped: [that id] }`,
     then a new answer: `ContextView::items` must not contain the
     dropped id. Use the existing `TurnRetrying` frame in
     `bingo-core/fixtures/frames-v1.jsonl` if it fits.
   - Session-level (`session/tests/`, with the existing `ScriptedProvider`): a
     scripted retryable failure after partial text, a successful retry,
     then a second turn. The second turn's request must not contain the
     withdrawn text. Then close, resume from the same store, submit
     again: still absent.
   - Store-failure test: a `SessionStore` fake whose `append` fails on
     the Nth durable frame. Assert the frame is not in `SessionState`,
     no subscriber receives it, a `Notice` names the error, and the
     session ends (`SessionClosed { Error }` reaches subscribers; the
     mailbox refuses new input with `SESSION_CLOSED`).
2. **`ContextView::items` honours `TurnRetrying`:** remove every id in
   `dropped`, as `SessionState::turn_retrying` does. While there, replace
   the `iter_mut().find` upsert with an id → index map local to the
   function (O(n)); keep the function's signature.
3. **Journal first.** In `publish`, a durable frame is appended before
   anything else; on `Err`:
   - do not push to `self.journal`, do not `state.apply`, do not fan
     out, do not send to observers;
   - fan out an ephemeral `Event::Notice` (never journaled) naming the
     storage error;
   - stop the session: cancel the running turn, reject queued intents
     with `STORAGE`, fan out `SessionClosed { reason: Error { message } }`
     *without* journaling it (it cannot be; the store is failing), clear
     subscribers, return `Flow::Stop` through the actor loop.
   `publish` returns a result the callers propagate; split the stop into
   its own function (`storage_failed`). Callers that publish in a loop
   stop at the first failure.
4. **The store's derived files are not the journal.** In
   `JsonlStore::append` (`bingo-store-jsonl/src/lib.rs`), once
   `journal::append` succeeded, a failure of `summary::write` or
   `summary::count_message` is logged and `Ok(())` is returned: the
   line is written, and the summary is derived (ADR-0005 §5). Test both summary paths
   with a directory at `summary.json` (replacement must fail on every OS).
5. Dated note on ADR-0002 §2: "enforced by M108: a failed append stops
   the session". No new ADR.

## Files

- `crates/bingo-core/src/context.rs` and its tests
- `crates/bingo-core/src/session.rs` (`publish`, close path), a new
  `crates/bingo-core/src/session/storage.rs` if the stop needs a module
- `crates/bingo-core/src/session/{inputs,interactions,mailbox}.rs` (propagation)
- `crates/bingo-core/src/session/tests.rs` and `tests/` (new `retry.rs`, `storage.rs`)
- `crates/bingo-core/src/test_support.rs` (a failing store fake, if none)
- `crates/bingo-store-jsonl/src/lib.rs` and its tests
- `docs/adr/0002-event-stream.md` (dated note)

## Exit criteria

- [ ] The withdrawn-text tests fail before brick 2 and pass after:
      `cargo test -p bingo-core retry` output pasted, both runs.
- [ ] Resume after a retry does not resend withdrawn text (test above).
- [ ] A failing append is never applied, broadcast or observed, and the
      session ends with `SessionClosed { Error }` seen by a subscriber.
- [ ] A failing summary write after a good journal line is `Ok` and the
      line replays.
- [ ] All gates of the execution protocol, including the Windows check
      for `bingo-core` and `bingo-store-jsonl`.

## Non-goals

- Unifying the folds (M112), the store writer task (M116), retrying a
  failed append, or any change to `SessionState`.
- Journaling `SessionClosed` after a storage failure.

## Risks

- `publish` has many callers; making it fallible may ripple. Keep the
  `Result` inside the actor and convert at the loop, not in every
  caller's signature, if the ripple is large.
- A session stopped for storage must not leave a turn task running:
  assert the turn's token is cancelled in the storage test.
- Re-verify at start: `publish`, `ContextView::items`, `turn_retrying`
  and `JsonlStore::append` by name, not by line.

## Re-verification (2026-10-13)

At `82e78d22`, all named symbols and bugs remain. Publication callers in
`inputs.rs` and `interactions.rs` need fallible propagation; mailbox replies
must not acknowledge failed writes. Existing `ScriptedProvider` records
requests; no provider dependency is needed. `summary::write` renames a
temporary file, so a read-only destination does not fail reliably on Unix.
Use a destination directory to test the same failure portably.
