# ADR-0066 — Resume starts at a checkpoint

Status: proposed · 2026-10-13 · Plan: M114 · Amends: ADR-0005 §5, §7

## Context

Resume reads the whole journal (`host/resume.rs`, `store.replay(id,
Seq::ZERO)`) and folds every frame, and the actor then keeps two copies
of the past for the rest of the process: every durable frame in
`Actor::journal`, which `subscribe(since)` filters for a client's
catch-up, and every item in `SessionState`, which `HostApi::history`
(M107) pages from. A session's memory and its resume time grow with its
age, not with what anyone is looking at. M107 bounded what a *client*
receives and named cold replay a non-goal; this is that goal.

Compaction (ADR-0006) already marks the point after which the model no
longer reads the past. ADR-0065 makes a journal's lines readable
without the live type, and ADR-0064 gives one fold whose state can be
serialised. What is missing is a durable statement of "the fold up to
seq N" that resume can trust.

`HostApi::history` keeps its shape; `SessionStore` gains two verbs the
kernel calls (§6). The ratchet question: refusing them would put the
folded state somewhere the store does not own — a journal frame that
repeats the fold, or a second file the kernel writes beside the
store's — which is a second representation of the session. The door
opens.

## Decision

1. **A checkpoint is a derived sidecar.** `<session>/checkpoint.json`
   holds `{format, version, seq, offset, line_hash, state}`: the seq of
   the last frame it covers, the byte offset just past that frame's
   line, a hash of that line's bytes, and the folded `SessionState` as of
   that seq. Like `summary.json` (§5), it is derived: deleting it loses
   nothing and the next resume rebuilds it.
2. **When it is written.** After a `Compacted` frame is durably
   appended, and after a clean close. It is written to a temporary file
   in the same directory and renamed over the old one, by the store, off
   the actor's task.
3. **Resume trusts it only when it checks.** The store reads the
   checkpoint, seeks to `offset`, confirms the line before it hashes to
   `line_hash` and is the frame with `seq`, and returns the state plus
   the frames after it. A missing, unreadable, older-version or
   mismatched checkpoint is a full replay followed by a fresh
   checkpoint — never an error, never a guess.
4. **The actor holds a window, not the past.** Working memory is the
   state's items from the latest compaction boundary onward, plus the
   durable frames since the checkpoint. The transcript records that
   older items exist (`OpenHistory`'s `before`/`has_more`, ADR-0064)
   without holding them; the model view never needed them.
   2026-10-13 clarification (ADR-0064 §1's kept items): after
   `drop_before_cut()`, (a) `model_items()` is unchanged, item for item
   and in order, including kept pre-boundary items; (b) `items()` and
   `OpenHistory` start at the compaction boundary in client order, with
   pre-boundary kept items held for the model only; (c) no held item is
   outside both model order and the client tail at/after the boundary;
   (d) storage history before the window includes kept items in full
   client order, exactly once across pages plus the window. Each item
   has one representation. The checkpoint uses its own state codec to
   preserve model order and window metadata; the client wire is unchanged.
5. **Older history is read from storage, off the actor.**
   `HostApi::history` for a page before the window, and a
   `subscribe(since)` older than the in-memory frames, are answered by
   the store on a blocking task; the actor only fixes the cut (the seq
   and generation the answer must be consistent with). A generation
   that moved meanwhile is the stale-page error M107 already defines.
6. **The `SessionStore` trait gains** `checkpoint(session, &Checkpoint)`
   and `resume(session) -> Resumed { state, tail }`, with defaults that
   fall back to `replay(Seq::ZERO)`, so a store without checkpoints is
   still correct.

## Consequences

- 2026-10-13: A checkpoint verifies only the line at its offset; a hand edit to an earlier journal line is not detected. Deleting checkpoint.json forces a full replay.

- Resume of a long session reads one sidecar and the frames since the
  last compaction; memory is bounded by the context window, not the
  session's age.
- A client that opens unbounded (no M107 budget) still receives every
  item: the kernel reads the pages it no longer holds. That client is
  slower to open; a bounded client is not.
- A crash between the `Compacted` append and the checkpoint rename
  leaves the old checkpoint, which still checks; resume reads a longer
  tail. Nothing is lost.
- The checkpoint's `state` serialises `SessionState`; ADR-0065's
  unknown-tolerance applies to it as to any record, and a version
  mismatch is a full replay.
- TUI scroll-back past the window pages through `HostApi::history`; a
  surface that assumed every item is in the snapshot must ask.
- The line hash is SHA-256 from a new dependency-free library,
  `bingo-digest`, lifted from `bingo-update`'s hand-written one (the
  tree has no other SHA-256 outside dev and C-built edges); ADR-0067's
  blobs use the same.

Refs: ADR-0005, ADR-0006, ADR-0064, ADR-0065; Plans: M107, M114;
audit-2026-10 §1
