# M112 — One transcript fold

## Goal

ADR-0064 §1–4. One `Transcript` type in `bingo-sdk` owns the item list
and its rules; `SessionState` embeds it, `ContextView` becomes its model
view, and the turn applies its own events through it. A turn no longer
starts by cloning the journal and re-folding it from the first frame.
After M108 the three folds agree on `TurnRetrying`; after this one
there is only one fold to agree.

## Bricks, in build order

1. **The contract, failing.** In `bingo-sdk`, a `transcript` module with
   the type's public surface stubbed (`Transcript::new`, `apply(&Event)
   -> Applied`, `items() -> &[Arc<Item>]`, `get(&ItemId)`, `generation()`,
   `cut() -> Option<&Cut>`) and the property test:
   - generate frame sequences over item start/update/complete,
     `TurnRetrying`, `Rewound`, `Compacted` (with known and unknown
     boundaries), using ids from a small pool so updates hit;
   - assert: incremental fold == fold from scratch; the item order and
     content equal what today's `SessionState::apply` produces (a
     reference implementation kept in the test, copied from today's
     code before it is removed); no dropped id survives.
   Add `proptest` to `bingo-sdk`'s dev-dependencies (already a
   workspace dependency; no new crate in the tree).
2. **`Transcript`.** `Vec<Arc<Item>>` plus `HashMap<ItemId, usize>`;
   removal rebuilds the index (removal is rare: retry, rewind). The
   compaction cut is recorded, not applied. `Serialize` writes the item
   list as today's `items` field; `Deserialize` rebuilds the index.
3. **`SessionState` embeds it.** Replace `items: Vec<Item>` and
   `history_generation` with `transcript: Transcript`, flattened so the
   serialised form is unchanged (fixture: today's `SessionState` JSON
   from an RPC snapshot test must round-trip byte-equal). Every reader of
   `state.items` moves to `state.transcript.items()`; rewind's
   `dropped(&self.state.items, ..)` takes the transcript.
4. **The model view.** `ContextView::model_view(&Transcript) ->
   Vec<Message>` applies the cut via today's `splice_compaction` logic
   (moved, not copied), skips `Item::external()` tool calls, and folds.
   `ContextView::items(frames)` and `fold(frames)` become test helpers
   (`#[cfg(test)]`) that build a `Transcript` first.
5. **The turn folds through it.** `TurnRun.history: Vec<Frame>` becomes
   `TurnRun.transcript: Transcript` (a clone is a vector of `Arc`s).
   The turn's `self.items`, `upsert`, its compaction splice and the
   retry `retain` are replaced by applying the event it emits to its own
   `Transcript` before `host.emit` — one function, `emit_and_fold`, so
   the two cannot be called apart. `Running::spawn` passes
   `actor.state.transcript.clone()`; `actor.journal` is no longer read
   by the turn.
6. **Measure.** A bench-style test (not a timing assertion): build a
   10 000-item session, assert that starting a turn clones no `Frame`
   (count via a test hook) and touches no item body (`Arc::strong_count`
   rises, contents unchanged).
7. ADR-0064 §1–4 accepted (the tree sections stay proposed until M120);
   dated note on ADR-0002 §3.

## Files

- `crates/bingo-sdk/src/{transcript.rs,state.rs,lib.rs}`, `Cargo.toml`
- `crates/bingo-core/src/{context.rs,turn.rs,turn/*.rs,rewind.rs}`
- `crates/bingo-core/src/session.rs`, `session/{running.rs,spawn.rs}`
- every reader of `SessionState::items` (`rg -n '\.items\b' crates` —
  surfaces read it: TUI, print, channels, rpc, demo-ui)
- `docs/adr/0064-*.md`, `docs/adr/0002-event-stream.md`

## Exit criteria

- [ ] The property test passes with ≥ 1 000 cases (output pasted).
- [ ] `rg -n 'ContextView::items|history: actor.journal' crates` finds
      only test code.
- [ ] `rg -n 'fn upsert|fn splice_compaction' crates` finds one
      definition of each rule, in `bingo-sdk`.
- [ ] `SessionState` JSON is byte-equal to before for the RPC fixtures.
- [ ] The M108 retry and resume tests pass unchanged.
- [ ] All gates, `scripts/tui-smoke.sh` included (surfaces changed).

## Non-goals

- The tree fold (M120); the checkpoint and windowed memory (M114);
  removing `Actor::journal` (M114 does, with catch-up from the store).
- Any change to what the model is sent.

## Risks

- `SessionState` is on the wire (ADR-0007); `#[serde(flatten)]` with a
  custom `Transcript` serialiser must not change field order or
  `skip_serializing_if` behaviour — the byte-equal fixture is the guard.
- `Arc<Item>` changes `&mut Item` access in the turn (status updates in
  `gate`); use `Arc::make_mut` on the turn's own copy.
- Re-verify at start: M108 has landed; `ContextView`, `TurnRun`,
  `Running::spawn` by name.

- **STOP (2026-10-13, protocol step 6):** re-verification at `72036be6`
  found that ADR-0064 §1–2's full client item list plus only the latest
  `(boundary, kept, summary)` cut cannot preserve the existing model view.
  `ContextView::items` applies cuts sequentially; `splice_compaction`
  moves each summary before the boundary, while `SessionState` keeps
  summaries in insertion order. Applying just the latest cut resurrects
  an earlier summary. This contradicts the Non-goal of changing model input.
- Reproduction using the current fold and splice in `context.rs`:
  complete `a`, `b`, `s1`; compact at `b`, keep nothing, summary `s1`;
  complete `s2`; compact at `b`, keep nothing, summary `s2`.
  The client list is `[a, b, s1, s2]`; sequential model replay is
  `[s2, b]`; applying only the latest cut to the client list is
  `[s2, b, s1]`. Omitting the first cut leaves the same client items
  and latest cut but changes the required model result to `[s2, b, s1]`:
  the proposed stored facts do not distinguish these histories.
- Options requiring a decision: (A) let `Transcript` retain cumulative
  model visibility and ordering metadata, folded alongside client items,
  instead of only the latest cut (recommended; item bodies can stay shared);
  (B) retain ordered cuts and enough event ordering to replay their effects
  (must address growth and request cost); (C) explicitly change model
  semantics to latest-cut-only (would allow the demonstrated resurrection).
  Do not implement any option or accept ADR-0064 until this is resolved.

## Verified (2026-10-13 — blocked, not complete)

- Worktree clean at entry; branch `m112-one-transcript-fold`, HEAD
  `72036be6`. M108's indexed retry removal is present. No branch switch.
- A temporary diagnostic test was added inside `context.rs` to replay
  the sequence above and compare it with the proposed latest-only splice.
  Command (exit 101; expected assertion failure, not a timing failure):
  `cargo test -p bingo-core --locked m112_latest_cut_preserves_repeated_compaction -- --exact context::tests::m112_latest_cut_preserves_repeated_compaction --nocapture`
  ```text
  running 1 test
  assertion `left == right` failed
    left: ["s2", "b", "s1"]
   right: ["s2", "b"]
  test context::tests::m112_latest_cut_preserves_repeated_compaction ... FAILED
  test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 410 filtered out; finished in 0.02s
  ```
- Removed the temporary diagnostic after recording its result; production
  code, dependencies, and ADR decisions are unchanged. Exit criteria remain
  unticked. Implementation and all gates are deferred because step 6
  requires stopping on a Decision conflict. No push, merge, or PR.
