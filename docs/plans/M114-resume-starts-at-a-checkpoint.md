# M114 — Resume starts at a checkpoint

## Goal

ADR-0066. A session resumes from a verified sidecar checkpoint plus the
journal's tail, and the actor holds a window — items from the latest
compaction boundary on, frames since the checkpoint — instead of the
whole past. Older history and old catch-up are read from storage off
the actor. M107 bounded what a client receives; this bounds what the
kernel holds.

## Bricks, in build order

1. **One SHA-256.** The tree's only dependency-free SHA-256 is
   `bingo-update/src/sha256.rs` (hand-written, tested on the standard's
   vectors; `sha2` is a dev-only edge and `aws-lc-rs` builds C). Lift it
   into a new library crate `bingo-digest` (`tier = "library"`, no
   dependencies) with its tests; `bingo-update` calls it. M115's blobs
   use the same crate.
2. **The record, with its test.** `bingo_sdk::store::Checkpoint { format,
   version, seq, offset, line_hash, state }` (`line_hash` is the SHA-256
   hex of the line's bytes). A fixture and a round-trip test.
3. **Trait verbs with defaults.** `SessionStore::checkpoint(&id,
   &Checkpoint)` (default: `Ok(())`, a store without checkpoints) and
   `SessionStore::resume(&id) -> Resumed { state: Option<SessionState>,
   tail: Vec<Frame> }` (default: `None` and `replay(Seq::ZERO)`). Tests
   that a store with only the defaults resumes exactly as today.
4. **The JSONL store writes and checks it.**
   - `checkpoint`: serialise to `checkpoint.json.tmp` in the session
     directory, rename over `checkpoint.json`, on a blocking thread.
   - `resume`: read the checkpoint; open the journal; seek to `offset`;
     confirm the bytes before it end with a line that hashes to
     `line_hash` and parses as the frame with `seq`; read the tail from
     `offset`. Any failure → full replay, `state: None`, and a
     `tracing::warn!` naming why. Tests: good checkpoint; truncated
     journal; edited line; missing file; version mismatch; checkpoint
     from another session.
5. **The kernel writes one.** After a `Compacted` frame is acknowledged
   by the store, and in `finish_close` for `Client`/`Shutdown` reasons,
   the actor sends the store a checkpoint of its state (the store does
   the I/O; the actor does not wait for it). The byte `offset` comes
   back from `append` — extend the store's internal append to return
   the offset after the line, or compute it in the store's writer.
6. **Resume uses it.** `host/resume.rs` `resume_frames` calls
   `store.resume`; with a state, the fold starts from it and applies the
   tail; without, as today. `session::replayed`, `report_lost_turns` and
   the mid-turn check work on the folded state either way. Test: a
   session with a compaction resumes reading only the tail (assert via
   a counting store wrapper), and its `SessionState` equals a full
   replay's (`PartialEq`).
7. **The window.** `Transcript` gains `drop_before_cut()`, which removes
   items before the compaction boundary and records `OpenHistory`'s
   `before`/`has_more`; the actor calls it after a checkpoint is written.
   `Actor::journal` keeps only frames after the checkpoint's seq.
8. **The past is read from storage.**
   - `subscribe(since)` with `since` older than the actor's first frame:
     the actor fixes the cut (current seq) and returns a stream whose
     first part is `store.replay(since)` filtered to `< cut`, read on a
     blocking task, then the live tail. Test: a client catching up from
     seq 0 after a checkpoint gets every frame once, in order.
   - `HostApi::history` for a page before the window reads the journal
     up to the checkpoint offset through the store, folds the items, and
     answers the page; generation checks as M107 defined. Test with
     M107's paging tests extended past the window.
9. ADR-0066 accepted; dated notes on ADR-0005 §5/§7 and on M107's
   non-goal ("cold replay: done in M114").

## Files

- `crates/bingo-digest/**` (new), `crates/bingo-update/src/{sha256.rs,lib.rs,checksums.rs}`
- `crates/bingo-sdk/src/{store.rs,transcript.rs,host.rs}`
- `crates/bingo-store-jsonl/src/{lib.rs,journal.rs,checkpoint.rs}` (new)
- `crates/bingo-core/src/{session.rs,session/*.rs,host/resume.rs}`
- tests beside each; `docs/adr/{0066,0005}*.md`, `docs/plans/M107-*.md`

## Exit criteria

- [ ] Every corrupted-checkpoint case falls back to full replay and
      rewrites a good one (tests pasted).
- [ ] Resume after compaction reads the tail only; state equals full
      replay.
- [ ] After a checkpoint the actor holds no frame and no item older than
      the window (assert lengths in a test).
- [ ] Catch-up from seq 0 and history paging past the window return
      what an un-windowed session returned (test pasted).
- [ ] All gates, `scripts/tui-smoke.sh` included.

## Non-goals

- TUI scroll-back paging UI (the TUI already pages through
  `HostApi::history`; if it assumes all items, that is a finding for
  M124, not a change here). Checkpoints between compactions.

## Risks

- The cut for catch-up must be taken in the actor's mailbox order or a
  frame is lost or doubled between the store read and the live tail.
- Folding items for a history page off the actor repeats the fold on a
  prefix; it must use `Transcript`, not a fourth fold.
- Re-verify at start: M112 and M113 have landed; `resume_frames`,
  `subscribe`, `bounded_snapshot`, `HostApi::history` by name.

- **STOP (2026-10-13, protocol steps 2/6): the literal window conflicts
  with the existing compaction contract.** At `1335830b`, ADR-0064 §1,
  `sdk/transcript.rs::splice_compaction`, `sdk/compactor.rs::Compaction`
  and `core/context.rs`'s kept-item regression preserve `kept` ids before
  `boundary`. ADR-0066 §4 and this plan's window exit criterion instead
  require no item before that boundary. Example: client items
  `[i1, i2, i3, i4]`, boundary `i3`, kept `[i1]`, summary `i4` produce
  model order `[i1, i4, i3]`; a boundary-only window loses `i1`.
  This is a Decision conflict, not mechanical fixture drift.
- **Options requiring a decision:** (A, recommended) clarify ADR-0066 §4
  and the window criterion to retain model-required pre-boundary items,
  with history paging handling the resulting gaps; (B) keep the strict
  boundary rule and change ADR-0064/the compaction contract to disallow
  pre-boundary `kept` items, which changes existing model behavior.
  Neither option is implemented; the supplied decisions remain unchanged.
- **Checkpoint codec finding:** M112's `Transcript` serializer writes
  only `historyGeneration` and client `items`; deserialization rebuilds
  model order from client order. A plain `SessionState` round trip is
  therefore not a checkpoint after compaction. Once the window decision
  is resolved, brick 2 needs a checkpoint-specific state codec preserving
  model order (and window metadata), without changing the client wire
  promised by ADR-0064. Re-verify the complete Files list before coding;
  manifests, lockfile and the dependency budget also need the new member.

## Verified (2026-10-13; stopped at re-verification)

- Branch `m114-resume-starts-at-a-checkpoint`, base `1335830b`; clean on
  entry. Read the execution protocol, audit and ADR-0066/0005, plus the
  relevant ADR-0064/0065 and M107 contracts. No implementation started.
- Temporary SDK probes constructed the four-item example above, applied
  `Compacted`, then compared model ids after a boundary window and a
  `SessionState` JSON round trip. Command:
  `cargo test -p bingo-sdk m114_reverification --locked -- --nocapture`.
  Actual output (trimmed):
  ```text
  m114_reverification_boundary_window_preserves_model ... FAILED
  left: ["i4", "i3"]
  right: ["i1", "i4", "i3"]
  m114_reverification_checkpoint_state_roundtrip ... FAILED
  left: ["i1", "i2", "i3", "i4"]
  right: ["i1", "i4", "i3"]
  test result: FAILED. 0 passed; 2 failed; 0 ignored
  ```
  Exit 101; both failures demonstrate the findings, not a build failure.
  The temporary probes were removed; only this plan is changed.
- All exit criteria remain unticked. Implementation gates, Windows checks
  and PTY smoke were not run because protocol step 6 ends this run.
  ADR-0066 remains proposed. No push, merge or PR.
