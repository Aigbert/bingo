# ADR-0081 — The screen is the person's

Status: proposed · 2026-10-13 · Plan: M148 · Amends: ADR-0013 §2 §4,
ADR-0070 §1, ADR-0079 §3

## Context

ADR-0013 gave a plugin three lanes; past 120 columns the panel and live
lanes became cards down a 24-column rail, and below it rows under the
running ones. A live drive (tui-identity-2026-10 §2) found the rail
holding raw plugin views under their kinds (`owed`, `members`, `Board`),
saying again what the session list already says, while the column stood
empty for most of the session; `ctrl+p` listed the same under internal
ids, with `–` where a plugin had taken its view back. No plugin in the
tree places itself on the rail: every card there is a legacy payload.
The person's word (2026-10-13): the rail goes, and `ctrl+p` with it.
No kernel door opens here: one sdk reducer rule changes (§5).

## Decision

1. **The TUI has no side column.** No rail at any width: `rail.rs`, its
   threshold and width, and pins go. The transcript takes the width.
2. **The TUI has no panel sheet.** `ctrl+p` and `panel.rs` go; the key is
   free.
3. **A payload without `slots` is not drawn by the TUI.** It stays in the
   fold for any wire client that reads it. To be seen in the TUI, a
   plugin places itself (ADR-0070) in a slot the TUI draws: `Status`,
   `Activity`, `Mode`, `Row`, and from M140 an asked `Pane`.
4. **`Slot::Rail` and `Slot::Panel` stay on the wire, unplaced in the
   TUI.** A wire client may draw them; the TUI draws neither, as a
   surface draws the fold of what is outside its table (ADR-0079 §2).
5. **An extension of `Null` removes its kind**, as a signal of `Null`
   already does (`SessionState::extended`, sdk `state.rs`). A plugin
   that takes its view back leaves nothing behind in the fold.
6. **Data for readers carries `slots: []`**: placed nowhere, read by
   name. The rooms plugin's `owed` and `members` are data: the session
   list and the crew strip (tui-identity C3) say who owes and who sits
   where.
7. **A pane sits inline, and only when asked** (ADR-0079 §3, M140): above
   the composer at a third of the height, at every width; it never
   docks. An unasked pane waits; the status line says `1 pane waiting`,
   and a click on it, or the mod's own command, opens it.

## Consequences

- The transcript is the screen at every width; nothing appears beside it.
- In-tree producers move to slots: demo-ui's board is a `Row` (its table
  the body, Tick and Reset its actions), its progress an `Activity`; the
  schedule's `wake` and rooms' `owed`/`members` are data.
- A plugin on the legacy lanes (protocol 1, external) is not drawn by the
  TUI until it places itself; its data is never lost.
- The rail's snapshots, the sheet's and the pin tests go; `tui-smoke.sh`
  reaches the board through `/board`.
- Mods parity: Claude Code docks a pane at 144 columns; bingo does not,
  by the person's choice. The parity matrix records it as a gap.

## Supersedes

ADR-0013 §2's panel and live lanes as drawn surfaces and §4's rail and
pins; ADR-0070 §1's `Rail` and `Panel` as places the TUI draws; ADR-0079
§3's docking of a pane.
