# M120 — One tree fold

## Goal

ADR-0064 §5–6. Three surfaces fold the session tree from the gateway's
`(SessionId, Frame)` stream, each in its own words:

- the TUI's `Tree::apply` (`crates/bingo-surface-tui/src/tree.rs`);
- the channels runner's `state_of` and `states`
  (`crates/bingo-channels/src/runner.rs`);
- `--print`'s `state_of` and `states`
  (`crates/bingo-surface-print/src/lib.rs`).

Only the TUI removes a child, and only the child, not its descendants.
One `bingo_sdk::tree::TreeState` replaces the three, with one creation
rule and one removal rule. Each surface keeps the state that is its own:
which child the TUI shows, its connection handles, a chat's message map.
No kernel door, no wire change.

## Bricks, in build order

1. **Contract tests first** (`bingo-sdk/src/tree.rs`, `#[cfg(test)]`),
   written against an empty `TreeState` so they fail:
   - a child appears on its `SessionUpdated` and folds what follows;
   - any other frame for an unknown session is `Applied::Stale` and
     changes nothing;
   - `SessionClosed { Deleted }` on a child removes it and every
     descendant reached through `SessionSummary.parent`;
   - `SessionClosed` for any other reason keeps the child, so the
     summary that reopens it folds into the same state;
   - `GatewayEvent::SessionRemoved` removes a child and its subtree, and
     is ignored for the root;
   - a frame for a removed session is stale until a new
     `SessionUpdated` announces it.
2. **`TreeState`**: `new(root: SessionState)`,
   `apply(&mut self, &Frame) -> Applied`,
   `removed(&mut self, &GatewayEvent) -> Vec<SessionId>`, `root()`,
   `get(&SessionId)`, `sessions()` (root first, then id order), and
   `children_of(&SessionId)`. Removal returns every id it took, so a
   surface drops what it keeps per session in the same step. It holds a
   `SessionState` per session and the parent links, nothing else
   (ADR-0064 §5).
3. **A property test**: for random sequences of announces, frames,
   closes and removals, folding all at once equals folding one at a
   time, and no session is present whose ancestor was removed.
4. **The TUI moves.** `tree::Tree` wraps a `TreeState` and keeps `view`
   and the switcher rows. `apply`, `state_mut` and `close` are deleted.
   `run.rs` `closed` and `removed` call the fold and drop `handles` for
   each id it returns. `removed` keeps pruning the switcher's stored list.
   If the viewed session is among the removed, the view returns to the
   root. The existing `tree.rs` tests and `screens/` snapshots pass
   unchanged.
5. **Channels moves.** The runner holds a `TreeState` in place of
   `states`. `state_of` goes, and `concerns_this_chat` reads the fold. A
   deleted child is dropped, which it never was. A new runner test
   deletes a child and checks the fold no longer holds it.
6. **Print moves.** The same in `crates/bingo-surface-print/src/lib.rs`,
   with `show` and `concerns_the_run` reading the fold, and a black-box
   test in which a sub-agent is deleted during a `--print` run and stdout
   stays valid NDJSON.

## Files

- `crates/bingo-sdk/src/{tree.rs,lib.rs}` (new module)
- `crates/bingo-surface-tui/src/{tree.rs,run.rs}` and the tests that
  build a `Tree` (`test_support.rs`, `screens/`)
- `crates/bingo-channels/src/runner.rs` and its tests
- `crates/bingo-surface-print/src/lib.rs` and
  `crates/bingo/tests/cli/` (print black-box)
- `docs/adr/0064-one-fold-for-the-transcript-one-for-the-tree.md`
  (accepted for §5–6, with the M112 half already accepted)

## Exit criteria

- [ ] `cargo test -p bingo-sdk tree` passes the contract and property
      tests. The failing first run is pasted.
- [ ] `rg 'BTreeMap<SessionId, SessionState>' crates --glob '!**/tests/**'`
      finds only `bingo-sdk/src/tree.rs`.
- [ ] `rg 'fn state_of' crates/bingo-channels crates/bingo-surface-print`
      finds nothing.
- [ ] A TUI test: deleting a child whose own child is viewed returns the
      view to the root and leaves no handle for either.
- [ ] Channels and print tests: a deleted child leaves the fold.
- [ ] `scripts/tui-smoke.sh` passes, and the TUI's snapshots are unchanged.
- [ ] All gates of the upgrade protocol, plus the Windows cross-check
      for the three surface crates.

## Non-goals

- The transcript fold (M112), which this builds on, unchanged.
- The words a surface uses for a child's lifecycle (`Status`: running,
  done, idle, stored) stay where they are. Moving them is ADR-0077's.
- The TUI's switcher listing of stored sessions the store answers with.
  That is not a tree fact.
- Any change to what the gateway sends.

## Risks

- Subtree removal is new behaviour in the TUI: today a grandchild of a
  deleted child stays listed. That listing was a bug, not a feature, but
  a person may notice a row disappearing. Say it in the commit body.
- `SessionSummary.parent` must be present on every announced child. If
  a child of a child arrives before its parent's summary, the fold keeps
  it as a root-level orphan until the parent arrives. The property test
  covers that order.
- Channels and print read the fold from a hot path. `TreeState` must not
  clone a `SessionState` per frame.
- Re-verify at start: M112's `Transcript` inside `SessionState`, the
  three fold sites' line numbers, and the `GatewayEvent` variants
  (`bingo-sdk/src/host.rs`).
