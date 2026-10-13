# M114 — Resume starts at a checkpoint

## Goal

ADR-0066. A session resumes from a verified sidecar checkpoint plus the
journal's tail, and the actor holds the client tail from the latest
compaction boundary, model-only kept items, and frames since the checkpoint
instead of the whole past. Older history and old catch-up are read from storage off
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
   hex of the line's bytes, newline included). Its own state codec preserves
   model order and window metadata; fixture and round-trip tests first.
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
   from the store writer, which seeks backward to the requested durable
   seq and hashes that complete line; the public append verb is unchanged.
6. **Resume uses it.** `host/resume.rs` `resume_frames` calls
   `store.resume`; with a state, the fold starts from it and applies the
   tail; without, as today. `session::replayed`, `report_lost_turns` and
   the mid-turn check work on the folded state either way. Test: a
   session with a compaction resumes reading only the tail (assert via
   a counting store wrapper), and its `SessionState` equals a full
   replay's after the same window projection (`PartialEq`).
7. **The window.** `Transcript` gains `drop_before_cut()`, which removes
   unneeded pre-boundary items, hides model-only items, and records `OpenHistory`'s
   `before`/`has_more`; the actor calls it after a checkpoint is written.
   `Actor::journal` keeps only frames after the checkpoint's seq.
8. **The past is read from storage.**
   - `subscribe(since)` with `since` older than the actor's first frame:
     the actor fixes the cut (current seq) and returns a stream whose
     first part is `store.replay(since)` filtered to `< cut` (current seq + 1), on a
     blocking task, then the live tail. Test: a client catching up from
     seq 0 after a checkpoint gets every frame once, in order.
   - `HostApi::history` for a page before the window reads the journal
     through the store, filters to the fixed seq cut, folds the items, and
     answers the page; generation checks as M107 defined. Test with
     M107's paging tests extended past the window.
9. ADR-0066 accepted; dated notes on ADR-0005 §5/§7 and on M107's
   non-goal ("cold replay: done in M114").

## Files

- `crates/bingo-digest/**` (new), `crates/bingo-update/src/{sha256.rs,lib.rs,checksums.rs}`
- `crates/bingo-sdk/src/{store,transcript}/**`, `{store.rs,transcript.rs,host.rs,lib.rs}`
- `crates/bingo-store-jsonl/src/{lib.rs,journal.rs,checkpoint.rs,checkpoint/**}`
- `crates/bingo-core/src/{session.rs,session/*.rs,host/resume.rs}`
- `crates/bingo/tests/cli/{jobs,agents}.rs::journal_text` (journal files only)
- Workspace/crate `Cargo.toml`, `Cargo.lock`, `scripts/budget.toml` (+1 member)
- tests/fixtures beside each, core host tests; `docs/adr/{0066,0005}*.md`, `docs/plans/M107-*.md`

## Exit criteria

- [ ] Every corrupted-checkpoint case falls back to full replay and
      rewrites a good one (tests pasted).
- [ ] Resume after compaction reads the tail only; state equals full
      replay.
- [ ] After a checkpoint the actor holds no covered frame and no item
      outside both model order and the client tail at/after the boundary.
- [ ] Four-item cut: model `[i1,i4,i3]`, client `[i3,i4]`, storage page
      `[i1,i2]`; no gaps/duplicates, and model order survives the cut.
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

- 2026-10-13 decision: use ADR-0066 §4's clarified model/client window
  invariant and a checkpoint-specific state codec; client wire unchanged.

- 2026-10-13 decision: restrict jobs/agents test journal helpers to `journal.jsonl`; assertions unchanged.

## Verified (2026-10-13; remaining gates pending)

- Branch `m114-resume-starts-at-a-checkpoint`, base `1335830b`; clean on entry.
  Temporary probes (`cargo test -p bingo-sdk m114_reverification --locked
  -- --nocapture`): `0 passed; 2 failed`. Window model `[i4,i3]` and
  serialized model `[i1,i2,i3,i4]` differed from `[i1,i4,i3]`. Probes removed;
  the user decision above resolved that initial stop.
- RED contracts: `cargo test -p bingo-sdk checkpoint --locked`: `2 failed`
  (missing codec, client still i1–i4); `cargo test -p bingo-store-jsonl
  checkpoint --locked`: `2 failed` (state None, sidecar absent);
  `cargo test -p bingo-core checkpoint_ack --locked`: `1 failed`
  (`actor.journal.is_empty()`). Contract commits precede implementations.
- GREEN: `cargo test -p bingo-digest -p bingo-update`: `3 passed` / `47 passed`;
  `scripts/budget.sh`: `349 (max 349)`, `budget ok`.
  `cargo test -p bingo-sdk -p bingo-store-jsonl --locked`: SDK `111 passed`
  plus record/opaque tests/docs; store `54 passed; 1 ignored` (measurement).
  `cargo test -p bingo-core --locked`: `432 passed`; then the additional
  held-write/oversized-page cases with `cargo test -p bingo-core checkpoint
  --locked`: `9 passed; 0 failed`. Logs: `target/m114-*-tests.log` and
  `target/m114-core-checkpoints.log`.
- Gates: fmt exit 0; workspace all-targets locked check exit 0 (54.56s);
  workspace all-targets locked clippy with `-D warnings` exit 0 (26.75s).
- `cargo test --workspace --locked -- --test-threads=1 --skip
  a_token_may_be_dialled_again_once_its_stream_has_closed --skip
  a_proxy_with_a_token_this_run_never_minted_gets_nothing`: exit 101.
  `target/m114-workspace-tests.log`, CLI: `248 passed; 3 failed`:
  ```text
  jobs::a_default_watch_still_says_it_once_and_counts_nothing: 2 != 1
  jobs::a_pinned_watch_wakes_once_and_counts_the_rest_onto_the_end: 2 != 1
  jobs::an_ongoing_watch_on_a_slow_stream_wakes_on_every_line: 4 != 2
  ```
- Stopped before remaining tests (including isolated ACP), discipline, final
  budget, deny, Windows and TUI smoke. Exit criteria await final verification;
  ADR-0066 remains proposed. No push, merge, PR or parallel-owner file edits.
