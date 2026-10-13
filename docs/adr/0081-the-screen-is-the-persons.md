# ADR-0081 — The screen is the person's

Status: proposed · 2026-10-13 · Plan: M148 · Amends: ADR-0013 §2 §4,
ADR-0070 §1

## Context

ADR-0013 gave a plugin three lanes; past 120 columns the panel and live
lanes became cards down a 24-column rail, and below it rows under the
running ones. A live drive (tui-identity-2026-10 §2) found the rail
holding raw plugin views under their kinds (`owed`, `members`, `Board`),
saying again what the session list already says, while the column stood
empty for most of the session; `ctrl+p` listed the same under internal
ids, with `–` where a plugin had taken its view back. ADR-0070 let a
plugin place itself; the legacy lanes still place it without asking.
Claude Code's mods seat a pane nobody asked for only from 144 columns.
No kernel door opens here: one sdk reducer rule changes (§4).

## Decision

1. **A payload without `slots` is drawn only in the panel sheet.** It is
   never a rail card and never a row under the running ones. A person
   pins it from the sheet.
2. **An unasked card seats from 144 columns.** A `Slot::Rail` card seats
   past `RAIL_UNASKED = 144`; below it the card waits in the sheet, and
   one dim notice says `a card is waiting · ctrl+p` the first time it
   does in a session.
3. **A pinned card seats wherever the transcript keeps 80 columns** —
   `80 + RAIL_WIDTH` and up.
4. **An extension of `Null` removes its kind**, as a signal of `Null`
   already does (`SessionState::extended`, sdk `state.rs`). A plugin
   that takes its view back leaves nothing behind in the fold.
5. **The sheet speaks the plugin's words.** A row is named by its placed
   title, else by its kind with the plugin's last id segment beside it
   dim (`wake  schedule`); an empty view is not a row; the row under the
   cursor says what `⏎` does (`pin`, `unpin`).
6. **Data for readers carries `slots: []`**: placed nowhere, drawn
   nowhere, read by name by whoever reads it. The rooms plugin's `owed`
   and `members` are data: the session list and the crew strip
   (tui-identity C3) say who owes and who sits where.

## Consequences

- Most of the screen is the transcript at every width under 144 unless
  the person pinned something; the rail exists for what they asked for.
- A plugin on the legacy lanes (protocol 1, external) is still reachable
  in one key and never loses data; to be seen unasked it places itself.
- The `owed` and `members` cards and their snapshots go; `seats.rs`
  readers are unchanged, since a reader reads by name.
- Pins stay the surface's and per session (ADR-0013 §4), not journaled.

## Supersedes

ADR-0013 §2's drawing of the panel and live lanes as rail cards, and §4's
120-column rail for them. ADR-0070's `Rail` keeps its meaning, gated by
§2 here.
