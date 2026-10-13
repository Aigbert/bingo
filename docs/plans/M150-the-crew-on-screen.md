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
     hue, the sigil pulsing while live, `⎿  working · 3 tools · ↓ 1.2k`
     in M135's vocabulary; `asking` and `idle · completed` keep their words.
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
   session whose lifecycle is `Working` or `Asking`, in tree order, then
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
- TUI wiring: `src/{lib,blocks,seats,pointer,layout,view}.rs`,
  `src/layout/deadline.rs`, new `src/crew.rs`,
  `src/markdown/mentions.rs`, and TUI test modules/fixtures as needed.
  The room view is `transcript/said.rs`; cache revisions must include its
  render inputs and the known names used by prose.
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
      `BINGO_ASCII=1`; concise capture assertions recorded, no capture files.
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
- Re-verified at `9150697a` (2026-10-14), after M108–M136 and M147–M149.
  ADR-0034, ADR-0070/0071, ADR-0077 and ADR-0081 §§1–6 still hold.
  `Lifecycle::of_state` owns the words; `Tree::sessions` is root then id
  order, and `Scope::Under` walks descendants. `seats::seat` reads debts
  through `bingo_collab::room::debts` on the room's parent; `members` reads
  the room itself. Both payloads retain data with empty slots.
- There is no separate room-view module. `Place::previous` supplies chat
  grouping; `Rows` supplies folded context, and `blocks::Revision` must
  invalidate when that context changes. The status click belongs in pointer
  handling against the layout's status region; no mirror session state.
- C1's proposed dark pink is only 35.53° from `bad` in RGB hue; adjust
  crew colours minimally to satisfy 36° and 4.5:1 on their own palette's
  ground and raised tint. Existing state accents stay unchanged.
- No new dependency or wire/schema change is planned. M137's plugin-host,
  plugin-rpc, plugin manifests and schemas are outside this work.
