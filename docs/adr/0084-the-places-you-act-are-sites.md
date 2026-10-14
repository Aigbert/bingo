# ADR-0084 — The places you act are sites

Status: proposed · 2026-10-14 · each site accepted with its screen's prototype; first: `ModelPicker` (M166) · Amends: ADR-0079 §4, ADR-0082 Consequences

## Context

ADR-0082 made every place that shows something a site and deferred the
places where the person acts: the composer, completion, pickers, the
key hints, browsing the transcript. They are still drawn by the
terminal alone, three of them as separate UIs for one job (`/`
completion, `?` help, `ctrl+r` sessions). The redesign (canvas and
interactive prototype, 2026-10-14) draws them anew, and the desktop
client and the chat need the same places.

The ratchet: no kernel door opens. `HostApi::render` exists; this
record adds component names and props to the sdk.

## Decision

1. **Seven interaction sites**, each a `Component` with sdk props:
   - `Composer { text, cursor, mode, queued, attachments }`;
   - `KeyBar { actions: [{ action, keys }] }`: what the keys do now;
   - `Palette { query, open: commands | keys | sessions, groups }`;
   - `ModelPicker { filter, selected, scope, effort, sheet }`;
   - `Browse { selected, searching, matches }`;
   - `Ledger { steps }` and `TurnFold { turn, steps, outcome }`: the
     turn's steps and its folded line (ADR-0064's fold, not a copy).
2. **Props name things; the catalogue holds them.** A picker's entries
   are catalogue ids with the catalogue's meta as read at ask time; an
   action is its name and its bound keys from the keymap, never a
   label laid out for a width.
3. **Input stays with the surface.** A mod draws a site; keys, the
   cursor, filtering and selection are the surface's. A mod acts only
   through minted handlers (ADR-0079 §6). Entries come from catalogues
   (a mod's commands appear because it registered them), not from a
   drawing.
4. **`KeyBar` is sealed while an interaction is open** (ADR-0082 §6):
   the keys that answer a permission ask, a question or a sign-in are
   the host's drawing.
5. **`surfaces()`**: `Composer`, `KeyBar`, `Palette`, `ModelPicker`,
   `Browse`: terminal and desktop. `Ledger`, `TurnFold`: terminal,
   desktop, text. The chat answers none of them.

## Consequences

- Each site is added in the milestone that redraws its screen, after
  the person approves that screen's prototype; until then it stays the
  surface's private drawing.
- A mod can restyle the picker or the ledger without forking the
  terminal surface; it cannot change what a key does.
- Snapshot tests per site, as ADR-0082 had.

Refs: ADR-0064, ADR-0079, ADR-0082, ADR-0085.
