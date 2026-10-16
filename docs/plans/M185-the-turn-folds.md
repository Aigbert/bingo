# M185 — The turn folds

ADR-0095 §6, §7, §8; ADR-0064. The transcript's grammar from the prototype:
live step rows, a folded line per finished turn, the stop line.

## Goal

A finished turn reads `▸ 5 steps · read · grep · edit +1 −1 · test 3 passed ·
run ok   7s`; a running turn shows `✓ ◆ ◌ ✗` step rows and ` ▍ burrowing · 1s`;
a stop reads `■ you stopped the turn · 2 of 3 steps had run; nothing after them`.

## Bricks

1. **Per-tool outcome**: each tool result carries a one-line outcome (`+1 −1`,
   `3 passed`, `ok`, `exit 1`), written by the tool, fixture-tested.
2. **Ledger** and **TurnFold** sites (sdk props, built-in owner
   `bingo-mod-ledger`), folding the turn ADR-0064 already folds.
3. Live step rows; thinking folds into steps (ADR-0095 §6); diffs show relative
   paths and hunks only, `ctrl+o` expands inline.
4. Running line in lowercase verbs; the stop line; notes `⇄ ✎ ⟲ ⇣`;
   `▸ N earlier turns` at the top of a long transcript.
5. Expand (`enter`/click on `▸`) shows the full steps; Browse unchanged.

## Files

- `crates/bingo-sdk` (outcome, props), new `crates/bingo-mod-ledger`.
- `crates/bingo-tool-{fs,bash,web}` (outcomes).
- `crates/bingo-surface-tui/src/{transcript.rs,transcript/said.rs,fold.rs,activity.rs}`.

## Exit criteria

- [ ] Snapshots: running, finished, stopped, expanded, long transcript.
- [ ] Text surface (`--print`) unchanged unless it opts into `TurnFold`.
- [ ] tmux drive with captures; gates as in M182.

## Non-goals

Changing what is persisted in the journal; Browse's layout.

## Risks

- Scrollback: folded lines are what remains in the terminal's history; an
  expanded turn must not rewrite rows above the viewport.
