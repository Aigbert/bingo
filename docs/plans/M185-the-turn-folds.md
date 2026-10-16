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

- [x] Snapshots: running, finished, stopped, expanded, long transcript.
- [x] Text surface (`--print`) unchanged unless it opts into `TurnFold`.
- [x] tmux drive with captures; gates as in M182.

## Non-goals

Any journal change but an additive one (the optional `upshot` on a tool
output); Browse's layout.

## Risks

- Scrollback: folded lines are what remains in the terminal's history; an
  expanded turn must not rewrite rows above the viewport.

## Verified — 2026-10-17

Branch `m185-turn-folds`, rebased onto dev 3c37d20e (M191). Fake provider,
temp HOME/data/cwd and own `tmux -L` sockets only; no credentials.

**Bricks as built.** 1: `ToolOutput.upshot` (optional, `skip_serializing_if`,
journal/wire additive; schemas and `wire.ts` regenerated), written by
`tool-fs` (`+1 −1`, `1 file`), `tool-bash` (`ok`, `exit 1`), `tool-web`.
2: `Ledger` (per step) and `TurnFold` sites in `bingo-sdk`, owned by the new
built-in `bingo-mod-ledger`. 3/5: step rows at col 3 with the outcome two cells
from the edge; a finished turn folds onto its first step; `ctrl+o`, `⏎` on the
held line and a click open it in place (a click folds it again, a second
`ctrl+o` goes on to the pager); paths relative. 4: verbs are the lowercase tool
names (an ACP call reads `read Read …`); the stop line is the surface's,
asked of the `Interruption` site.

**Not done.** The notes `⇄ ✎ ⟲ ⇣`. `▸ N earlier turns`: the TUI owns an
alternate screen and its own scroll, so nothing reaches the terminal's
scrollback to fold; the Risk above does not arise and the line was left out.
Browse, `ui.block` answered views, slot-drawn rows (shell `$`, Skill `❖`),
agent-spawning `◗` rows and quiet calls keep their grammar.

**Snapshots** (`crates/bingo-surface-tui/src/snapshots/`, tests in
`src/screens/folds.rs`): `turn_running`, `turn_finished`, `turn_stopped`,
`turn_expanded`, `turns_many` at 80×24 and 120×40, `ascii_turn_expanded`,
`ascii_turn_stopped`; `every_row_of_a_turn_keeps_the_same_left_edge` asserts
col 1 / col 3 / two-from-the-edge on all of them;
`ctrl_o_opens_the_latest_folded_turn`, `a_click_or_enter_on_the_folded_line_opens_it`.

**`--print`.** It does not opt into `TurnFold`; `crates/bingo-surface-print`
changed only in a test fixture (`upshot: None`); no CLI black-box test was
edited. Fake-provider run of Read+Grep+answer: `text` exit 0, stdout `Done.`;
`stream-json` exit 0, tool results carry `content` only (no `▸`, no `upshot`).

**Drive** (`docs/verification/m185/drive.sh`, `tmux -L m185-drive-<cols>`):
`{asked,running,finished,expanded,pager,stopped,stopped_expanded}_{120x40,80x24}.txt`.
Finished 80×24:

```text
 › label the total in main
   ▸ 4 steps · read · grep · edit +1 −1 · bash ok                           5s
 ▍ Done. main now prints a labelled total.
```

Stopped: `▸ 3 steps · read · grep · bash · stopped   1s` over
`■ you stopped the turn · 2 of 3 steps had run; nothing after them`.

**Gates** (env without provider keys/URLs; final tree):

```text
cargo fmt --all -- --check                                  exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings   exit 0
RUST_TEST_THREADS=4 cargo test --workspace --locked --no-fail-fast
  exit 0 — 118 suites, 6344 passed, 0 failed (bingo-surface-tui 1369)
scripts/check_discipline.sh   exit 0 "discipline ok"; the same 239 warnings as
  dev 3c37d20e (sizes moved; sites_tests `fn site` 71 → 69 lines)
scripts/budget.sh             exit 0
  dependencies (unique, normal): 423 (max  423)
  warm cargo check -p bingo-core: 0s (max  20s)
  relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
  budget ok
cargo deny check              exit 0 "advisories ok, bans ok, licenses ok, sources ok"
TUI_SMOKE_SOCKET_PREFIX=m185-smoke scripts/tui-smoke.sh   exit 0 "tui-smoke ok"
cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc
  ok: sdk core hooks-shell mod-hello mod-ledger mod-models plugin-host
      provider-acp tool-bash
  blocked by aws-lc-sys (C headers, also lib-only): mcp plugin-rpc
      surface-print tool-fs tool-web surface-tui bingo; their changes here
      are string building and one `#[cfg(test)]`
```

Budget: `bingo-mod-ledger` is one new workspace member and no external crate;
`max_dependencies` 422 → 423 in `scripts/budget.toml` with its reason.
The smoke now reads `you stopped the turn` and presses `ctrl+o` before
looking for a step's picture, as the pty picture tests do.

**Rebase onto M191.** 1e4b49c1 M191's room rows off col 0; 9bd3a407 the hour
in UTC under test; ad9d2919 `input/turn.rs` (input.rs passed 1000 lines).

**Review fixes.** 53df0abd bash counts tests from whole output only;
ec89bc3a a diff's `+ −` count only inside hunks; f7211b69 one answers map per
frame (`Blocks::sync`, `long_session(500)`, debug: first 529 → 33 ms, warm
450 → 12 ms, `Roles::of` 226 → 7 ms); a2965556 a refusal read from its
decision; c55f2847 drops `TurnEnd::Failed`; 85580bed stop line drawn by the
surface; 2f696486 a turn folds only from its folded line; e922ad04 the
ledger's marks travel in `LedgerProps`/`TurnFoldProps.marks` (ASCII reaches
the built-in mod); 60512ae9 an opened turn asks `ToolResult`, `Reasoning`
and `Interruption`, site tests sit inside a turn, ADR-0095 §8 says which
sites a turn asks (none retires).

**Follow-ups.** `turns.rs` copies `ledger.rs` layout across sdk/TUI;
`row::Ink` is a second styled-text model naming colours as strings; the
spinner's tmux-only detection and copied `clock::alternating` maths; retire
the turnless `▏ Thought`/`▍ Name(args)` grammar (`thinking::lines`,
`Place.run`, `THOUGHT_ROWS`), reached now only outside every turn.
Residuals: a `cat` of an old log still counts tests; fold-word separators
(`·` `×` `−`) are not ASCII-switched; an unlinked receipt under an opened
step keeps its `⎿` row; a running step in a finished turn still spins.

**Found, not fixed.** On interrupt `wait_for` in
`crates/bingo-tool-bash/src/lib.rs` drops `listening` without republishing the
job list, so a stale `background` action stays offered; the TUI now hangs no
action under an ended step. The permission card's diff preview and the activity
row still show absolute paths.
