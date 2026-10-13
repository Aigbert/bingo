# M150 — The crew on screen

## Goal

[tui-identity-2026-10.md](../design/tui-identity-2026-10.md) §4, C1–C4,
the person's direction of 2026-10-13 (crew). What only bingo does —
agents, rooms, teams — gets a face on the screen: every agent a sigil
and a hue derived from its name, the status line's middle a strip of the
crew at work, a room read as a chat. The hue says who; the words and the
motion keep saying what, so `NO_COLOR` loses nothing. No new dependency,
no sdk change.

## Bricks, in build order

1. **Contract first: the face.** `crates/bingo-surface-tui/fixtures/
   faces.json` — twenty names and the face each gets — and the rule that
   produces it: FNV-1a 32-bit over the name's UTF-8; sigil `h % 6` over
   `◆ ● ▲ ■ ◗ ✦`, hue `(h >> 8) % 5` over lime, teal, violet, orchid,
   pink. The fixture is the contract a wire client (the GUI, in its own
   repository) can test against; it is written before the function.
2. **Failing tests first** (TUI):
   - `face(name)` matches the fixture; ASCII sigils `@ # % & + =` by the
     same index.
   - the hues: each ≥ 4.5:1 on both grounds and ≥ 36° of hue from
     `presence`, `good`, `bad` and `mode` (theme's own readable test,
     extended).
   - child row: `◆ reviewer(review the manifest)`, sigil and name in the
     hue, the sigil pulsing while live, `⎿  Running… 3 tools · ↓ 1.2k`
     unchanged in words (M135's vocabulary).
   - the list (`ctrl+g`): the dot column is the sigil; `~` still marks a
     live ear; a room keeps `#`.
   - crew strip: the status line's middle with two agents running reads
     `◆ reviewer  ● scout`, one that owes adds `owes #design`, one that
     needs you pulses; under 100 columns sigils only; a notice takes the
     slot for its 4 s and gives it back; with no agent alive the slot is
     what it is today; `N running` is never drawn.
   - room chat: two consecutive posts by one author draw the name once; a
     post addressed to the reader ends `→ you`; `@reviewer` in prose
     wears reviewer's hue, `#design` wears `mode`; the members row under
     the room's first message.
3. **`face.rs`** (new): the rule of brick 1, pure, and the five hues in
   `theme.rs` for dark and light (ANSI: none; the sigil is `text`).
4. **Rows** (`transcript.rs` `child_row`, `tree.rs` `bullet_style`,
   `roster.rs` `dot`, `transcript/said.rs` `post`): the face where the
   bullet or the dot was; state keeps its words and its pulse.
5. **The crew strip** (`status.rs` middle): agents under the viewed
   session that are alive by M135's vocabulary, in tree order, then
   `+N` for what the width cuts; `owes` read from the `owed` data
   M148 kept (by name, through M123's readers); a click opens `ctrl+g`.
6. **Mentions in prose** (`markdown.rs` inline pass, the room view):
   `@name` of a known agent takes its hue, `#name` of a known room takes
   `mode`; an unknown one is plain text. Known means in the tree the
   surface already folds; nothing is asked of a plugin.
7. **The room as a chat** (`transcript/said.rs`, the room view): the
   grouping, `→ you`, the members row (from the `members` data).
8. **Records.** tui.md §3 *Teams* and *Sessions*, §4 *child row*, *the
   list*, *status line*, a dated §10 entry; tui-identity §4 ticked.

## Files

- `crates/bingo-surface-tui/src/{face,theme,transcript,tree,roster,
  status,markdown}.rs`, `transcript/said.rs`, the room view, `fixtures/
  faces.json`
- `crates/bingo-surface-tui/src/**/snapshots` (teams, roster, room,
  child-row families)
- `scripts/tui-smoke.sh` (`❯ ⏺ ask me` becomes the session's sigil)
- `docs/design/tui.md`, `docs/design/tui-identity-2026-10.md`

## Exit criteria

- [ ] The fixture and each brick-2 test failed first (pasted) and pass.
- [ ] `rg 'running' crates/bingo-surface-tui/src/status.rs` finds no
      `N running` drawing.
- [ ] A tmux drive at 120×40 and 80×24 with a parent, two spawned agents
      and a room (fake provider): child rows, the strip while both run
      and after one ends, a needs-you pulse, `ctrl+g`, the room view with
      grouped posts and a mention; dark, light, `NO_COLOR`,
      `BINGO_ASCII=1`; captures pasted.
- [ ] Changed snapshots listed by family; no row moved.
- [ ] `scripts/tui-smoke.sh` passes.
- [ ] All gates of the upgrade protocol, plus the Windows cross-check
      for the TUI.

## Non-goals

- Faces chosen by a person or a persona; a face in `--print` or in a
  channel; the GUI's drawing of faces (it can read the fixture).
- Any change to what rooms record or deliver (ADR-0034).
- A per-agent colour for anything but the sigil and the name.

## Risks

- Two agents can share a face (30 faces). The name always stands beside
  it, so a collision costs a glance, never a fact.
- The strip competes with notices and placed statuses for one slot.
  Order: a notice, then a placed `Status`, then the strip; each test
  pins one handover.
- Re-verify at start: M135's lifecycle words and where the tree fold
  answers "alive"; M148's `owed` data shape; M149's caret and palette.
