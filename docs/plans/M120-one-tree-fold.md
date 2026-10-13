# M120 — One tree fold

## Goal

ADR-0064 §5–6: replace the TUI's `Tree::apply`, channels' `state_of`,
and print's `state_of` with `bingo_sdk::tree::TreeState`. One creation
rule and one subtree removal rule; surfaces retain view selection,
connection handles and chat message maps. No kernel door or wire change.

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
   `get(&SessionId)`, `sessions()` (root first, then id order),
   `children_of(&SessionId)`, and `mark_read(&SessionId)` for the existing
   client read marker. Parent links are read from each summary. Removal
   returns every id it took, for surfaces to drop their per-session state. It holds a
   `SessionState` per session and the parent links, nothing else
   (ADR-0064 §5).
3. **A property test**: for random sequences of announces, frames,
   closes and removals, folding all at once equals folding one at a
   time, and no session is present whose ancestor was removed.
4. **The TUI moves.** `tree::Tree` wraps a `TreeState` and keeps `view`
   and the switcher rows. `apply`, `state_mut` and `close` are deleted.
   `run.rs` folds frames once and drops handles for deleted ids; gateway
   removal uses the ids returned by `removed`. Frame removal derives the
   removed ids from membership before and after applying the frame.
   `removed` keeps pruning the switcher's stored list.
   If the viewed session is among the removed, the view returns to the
   root. Existing assertions and snapshots stay unchanged; fixture construction
   calls the SDK fold directly.
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
- `crates/bingo-surface-print/src/{lib.rs,hosted.rs}` and
  `crates/bingo/tests/cli/` (print black-box)
- `docs/adr/0064-one-fold-for-the-transcript-one-for-the-tree.md`
  (accepted for §5–6, with the M112 half already accepted)
- `docs/adr/0002-event-stream.md` (dated amendment note)
- Mechanical TUI fixture updates in `view.rs`, `roster.rs`, `seats.rs`,
  `activity.rs`, `worked.rs`, and `screens.rs`; existing assertions remain unchanged.

## Exit criteria

- [x] `cargo test -p bingo-sdk tree` passes the contract and property
      tests. The failing first run is pasted.
- [x] `rg 'BTreeMap<SessionId, SessionState>' crates --glob '!**/tests/**'`
      finds only `bingo-sdk/src/tree.rs`.
- [x] `rg 'fn state_of' crates/bingo-channels crates/bingo-surface-print`
      finds nothing.
- [x] A TUI test: deleting a child whose own child is viewed returns the
      view to the root and leaves no handle for either.
- [x] Channels and print tests: a deleted child leaves the fold.
- [x] `scripts/tui-smoke.sh` passes, and the TUI's snapshots are unchanged.
- [x] Native protocol gates and the SDK Windows cross-check.
- [ ] Full Windows checks for the three surfaces and binary: `aws-lc-sys`
      cannot find SDK C headers on this Mac; left to Windows CI (step 7).

## Non-goals

M112's transcript; lifecycle words (`Status`, ADR-0077); the switcher's
stored-session listing; any change to what the gateway sends.

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
## Re-verified (2026-10-13, 052defcb)

M112's `SessionState.transcript` and `Transcript` are present. Fold sites:
TUI `tree.rs:143`, channels `runner.rs:173`, print `lib.rs:286`; print
`hosted.rs:77` also reads the map. `GatewayEvent` (`host.rs:297`) still has
`SessionRemoved`, `SessionCreated`, `CatalogChanged`. The SDK delegates the
TUI's existing read marker without mutable summary access or copied links.
No dependency change; the existing `proptest` supplies the property test.

## Verified (2026-10-13)

- Empty-fold contract, committed before implementation: `cargo test -p bingo-sdk tree --locked`:
  `left: Stale`, `right: Session`; `test result: FAILED. 0 passed; 5 failed; 0 ignored`.
- Same command after implementation: `test result: ok. 6 passed; 0 failed; 0 ignored`;
  includes 1,000 randomized sequences, prefix replay and an independent removal oracle.
- `rg 'BTreeMap<SessionId, SessionState>' crates --glob '!**/tests/**'`:
  `crates/bingo-sdk/src/tree.rs:    children: BTreeMap<SessionId, SessionState>,`
- `rg 'fn state_of' crates/bingo-channels crates/bingo-surface-print`: no matches (exit 1).
- `cargo test -p bingo-channels -p bingo-surface-print -p bingo-surface-tui --lib --locked`:
  channels `103 passed`; print `101 passed`. TUI's root-selection regression was fixed;
  `cargo test -p bingo-surface-tui --lib --locked`: `1149 passed; 0 failed; 2 ignored`.
- The workspace run below includes these exact output lines:
  `test run::tests::deleting_a_child_returns_its_viewed_descendant_and_drops_both_handles ... ok`
  `test runner::tests::a_deleted_child_leaves_the_chat_fold ... ok`
  `test tests::a_deleted_child_leaves_the_print_fold ... ok`
  `test agents::dismissing_a_child_keeps_print_stdout_valid_ndjson ... ok`
- `cargo fmt --all -- --check`: exit 0 (no output).
- `cargo check --workspace --all-targets --locked`: ``Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.36s``.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: ``Finished `dev` profile [unoptimized + debuginfo] target(s) in 14.31s``, after moving a test-only import.
- `INSTA_UPDATE=no cargo test --workspace --locked -- --test-threads=1 --skip a_token_may_be_dialled_again_once_its_stream_has_closed --skip a_proxy_with_a_token_this_run_never_minted_gets_nothing`:
  exit 0; summed output: `5028 passed; 0 failed; 3 ignored` (existing ignored tests).
- `cargo test -p bingo-provider-acp --test bridge --locked a_token_may_be_dialled_again_once_its_stream_has_closed -- --exact --test-threads=1`:
  `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out`.
- `cargo test -p bingo --test acp_bridge --locked a_proxy_with_a_token_this_run_never_minted_gets_nothing -- --exact --test-threads=1`:
  `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out`.
- `scripts/check_discipline.sh`: `discipline ok`, after trimming duplicate commentary (`run.rs`: 998 non-test lines).
- `scripts/budget.sh`: `dependencies (unique, normal): 348 (max  348)`; warm core check `0s`; relink `0 crates`; `target/debug: 5 GB`; `budget ok`.
- `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.
- Windows command, run separately for each crate: `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc`.
  `bingo-sdk`: ``Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.02s``.
  `bingo-surface-tui`, `bingo-channels`, `bingo-surface-print`, `bingo`:
  ``error: failed to run custom build command for `aws-lc-sys v0.44.0` ``;
  `fatal error: 'stdlib.h' file not found` (also `windows.h`). No Rust errors reported;
  these four checks are left to Windows CI per protocol step 7; no toolchain installed.
- `scripts/tui-smoke.sh`, with a private `TMUX_TMPDIR`: `tui-smoke ok`.
  Workspace PTY tests passed without retries. `git diff 052defcb --exit-code -- '*.snap'`: exit 0.
