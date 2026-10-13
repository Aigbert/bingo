# ADR-0081 — The screen is the person's

Status: accepted §§1–6 · 2026-10-14 · Plan: M148; §7 proposed, M151 · Amends: ADR-0013
§2 §4, ADR-0070 §1, ADR-0079 §3 §5

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
   threshold and width, and pins go. The transcript takes the width,
   save what a pane the person opened declares (§7).
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
7. **A pane says where it wants to be; the TUI decides** (revised the
   same day at the person's word). `Slot::Pane` gains `place`, places in
   order of preference, and `size`, `{ rows, columns }` each `{ min,
   ideal, max }`. A length is cells or a percentage of the area: the
   transcript's, never the composer, the status line or the permission
   prompt. A place is a word or a rect:
   - `above`: above the composer, the full width. The default.
   - `side`: right of the transcript, `size.columns` wide.
   - `overlay`: over the whole area.
   - `{ rect: { anchor, x, y, width, height } }`: floating over the
     area; `anchor` a corner or `center` (default `top-left`), so a rect
     in percentages survives a resize. Its lengths are its ideal size.

   The TUI seats a pane at the first place that fits. `above` and `side`
   fit when the pane's minimum leaves the transcript 60 columns and 8
   rows (named constants); `overlay` and a rect fit when the area holds
   the minimum, a rect past an edge moving in before it shrinks. One
   `above`, one `side`, at most three floating, the newest on top. When
   nothing fits the pane collapses to `<title> ▸` on the status line and
   a click opens it as `overlay`. A pane not asked waits as before (`1
   pane waiting`). The person's own move — a click on the title row
   cycles the places that fit — is kept by id for the TUI's process and
   beats the declaration. The render request's `viewport` carries the
   `place`, columns and rows the pane got, and a pane is asked again when
   any of them changes (amending ADR-0079 §5 for panes), so a mod
   degrades its drawing by what it got. A wire client reads the same
   declaration under its own rules.

## Consequences

- The transcript is the screen at every width; nothing appears beside it
  but a pane the person opened.
- In-tree producers move to slots: demo-ui's board is a `Row` (its table
  the body, Tick and Reset its actions), its progress an `Activity`; the
  schedule's `wake` and rooms' `owed`/`members` are data.
- A plugin on the legacy lanes (protocol 1, external) is not drawn by the
  TUI until it places itself; its data is never lost.
- The rail's snapshots, the sheet's and the pin tests go; `tui-smoke.sh`
  reaches the board through `/board`.
- Mods parity: Claude Code's engine docks a pane by its own width rule;
  a bingo pane declares its place and size and the TUI honours what fits
  (M151). Nothing appears beside the transcript unless asked for.

## Supersedes

ADR-0013 §2's panel and live lanes as drawn surfaces and §4's rail and
pins; ADR-0070 §1's `Rail` and `Panel` as places the TUI draws; ADR-0079
§3's docking of a pane, by the engine's rule rather than the pane's.
