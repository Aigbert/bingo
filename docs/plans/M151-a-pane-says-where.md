# M151 — A pane says where it wants to be

## Goal

[ADR-0081](../adr/0081-the-screen-is-the-persons.md) §7, the person's
word of 2026-10-13: a pane declares its place — a word (`above`, `side`,
`overlay`) or a rect — in order of preference, and its size as `{ min,
ideal, max }`; the TUI seats it at the first place that fits, collapses
it to the status line when none does, and tells the mod what it got
through the render `viewport`. M140 seats every asked pane `above`; this
milestone adds the rest. No new dependency. After M140 and M148.

## Bricks, in build order

1. **Contract first** (sdk `slot.rs`, `render.rs`). `Place = Above |
   Side | Overlay | Rect(Rect)`; `Rect { anchor, x, y, width, height }`;
   `Anchor = TopLeft | TopRight | BottomLeft | BottomRight | Center`
   (default `TopLeft`); `Length = Cells(u16) | Percent(u8)`, JSON `12`
   or `"40%"`; `Span { min, ideal, max }`; `PaneSize { rows, columns }`.
   `Slot::Pane` gains `place: Vec<Place>` and `size`. Golden fixtures:
   `["side", "above"]`, `[{ "rect": { "anchor": "bottomRight", "width":
   "40%", "height": "30%" } }]`, and a pane without either, which reads
   as `["above"]` at M140's size. `Viewport` gains `place`.
   `published_ok` refuses a percentage above 100 and a `min` above a
   `max`. Every test fails first.
2. **The pure brick** (TUI `pane/seat.rs`). `seat(area, wanted, moved)
   -> Seating`, pure in sizes: for each asked pane, the first place that
   fits, with its rect; the transcript's remaining rect; the collapsed
   list. Fit is ADR-0081 §7's: `TRANSCRIPT_MIN_COLUMNS = 60`,
   `TRANSCRIPT_MIN_ROWS = 8`; one `above`, one `side`, `FLOATING_MAX =
   3`; a rect past an edge moves in, then shrinks to the area, and does
   not fit below its minimum. Table tests, each a named case:
   - 200×50: `side` fits at its ideal; the transcript keeps the rest.
   - 100×30: `side` leaves under 60 columns, so the pane is `above`.
   - 60×15: nothing fits; collapsed.
   - a rect at each anchor, in cells and percentages; one past the right
     edge moves in; one larger than the area shrinks; one under its min
     after shrinking does not fit.
   - two panes asking `side`: the second takes its next place.
   - a fourth floating pane takes its next place or collapses.
   - a move the person made wins over the declaration while it fits.
3. **Layout** (M124's pass). The transcript's region is `Seating`'s;
   `above` sits between it and the composer; `side` to its right;
   floating panes draw after the transcript, newest last; the permission
   prompt and toasts draw over them all. Pointer routing (`pointer.rs`):
   a click, a wheel and a drag-to-select (M92) go to the topmost pane
   under the pointer; the transcript's selection never starts inside a
   pane.
4. **What the mod is told** (`pulled.rs`). The render request's
   `viewport` is `{ place, columns, rows }` as seated; the memo key gains
   `place` and, for a pane only, `rows` (ADR-0079 §5, dated note). A
   resize that moves a pane from `side` to `above` asks it again at its
   new width. A collapsed pane is not asked.
5. **Collapsed and waiting** (`status.rs`). A collapsed pane is `<title>
   ▸` in the status line's right part; a click seats it as `overlay`. A
   pane not asked keeps M140's `1 pane waiting`.
6. **The person's move.** A focused pane's title row ends in `place ▾`;
   a click cycles the places that fit, in `above`, `side`, `overlay`
   order, then the declared rect. The same on a key, chosen at the start
   against `keys.rs` and listed in the `?` sheet. Moves are kept by pane
   id in the TUI's process (`pane/moved.rs`), never in the session.
7. **The mod's side.** `$.ui.open({ id, title, place, size })` in the
   mod runtime and `sdk/typescript/src/ui.ts`; plugin-rpc's
   `Slot::Pane` through the schema, regenerated with
   `BINGO_UPDATE_SCHEMA=1`. M140's fixture mod `counter` declares
   `place: ["side", "above"]`; a second fixture, `corner`, declares a
   bottom-right rect of `"40%"` by `"30%"` with a `Raster` that fills it.
8. **Records.** ADR-0081 accepted for §7; dated notes on ADR-0079 §3 §5;
   tui.md (the pane's places), mods-parity §4 and §8, the guide page on
   panes with one example per place.

## Files

- `crates/bingo-sdk/src/{slot,render}.rs` and their fixtures;
  `schema/{rpc,plugin-2}.json` (regenerated, additive)
- `crates/bingo-surface-tui/src/pane/{mod,seat,moved}.rs` (`pane.rs`
  becomes the module), `{layout,pointer,pulled,status,keys}.rs`
- `crates/bingo-mods/src/` (`$.ui.open`), `sdk/typescript/src/ui.ts`
- `crates/bingo/tests/{mods_ui.rs,fixtures/mods/{counter,corner}/}`
- `docs/adr/{0079,0081}-*.md`, `docs/design/{tui,mods-parity-2026-10}.md`,
  the panes guide page

## Exit criteria

- [ ] Brick 1's and brick 2's tests failed first (output pasted) and pass:
      `cargo test -p bingo-sdk slot:: render::`, `cargo test -p
      bingo-surface-tui pane::`.
- [ ] `TestBackend` snapshots: `side` at 200×50, `above` at 100×30,
      collapsed at 60×15, the `corner` rect at 120×40 over the
      transcript, the permission prompt drawn over a floating pane.
- [ ] `cargo test -p bingo --test mods_ui`: the render request carries
      the seated `place`, columns and rows; a resize from 200 to 100
      columns asks `counter` again with `place: "above"`.
- [ ] A tmux drive (own session only) with `counter` and `corner`: open
      at 200×50, resize to 100×30 and 60×15, click `<title> ▸`, click
      `place ▾`; each step's `capture-pane` pasted.
- [ ] `scripts/tui-smoke.sh` passes; all gates of the upgrade protocol;
      the Windows check of `bingo-surface-tui`, `bingo-sdk`, `bingo-mods`.

## Non-goals

- Dragging a pane's edge or body with the mouse; the move is a cycle.
- Keeping the person's moves across processes or in the session.
- A pane over the composer, the status line or the permission prompt.
- `open`'s `isPlaced`: the mod learns its seat from each render's
  `viewport`, and `open` still resolves `{}` (design §8).
- An unasked pane taking the screen at any width.

## Risks

- **Pointer routing.** Floating panes put a second hit-test over the
  transcript; select-to-copy (M92) and the wheel are the likely breaks.
  Brick 3's tests drive both before the drawing lands.
- **Redraw cost.** A pane asked again on every resize step; the memo and
  M140's four-in-flight cap hold it, and the drive says if it lags.
- Re-verify at start: M140's `Slot::Pane`, `Viewport` and `pane.rs`;
  M124's layout targets after M148; `pointer.rs` after M92 and M148;
  which key is free.
