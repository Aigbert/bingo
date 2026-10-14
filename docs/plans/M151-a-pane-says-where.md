# M151 — A pane says where it wants to be

## Goal

Implement [ADR-0081](../adr/0081-the-screen-is-the-persons.md) §7:
a pane declares preferred places and bounded sizes; the TUI seats it at the
first fit, collapses it to status otherwise, and reports the actual viewport.
No new dependency. Hard prerequisites M140 and M148 have landed.

Re-verified at `ba70fdf6` on 2026-10-14, clean branch
`m151-a-pane-says-where`. Landed dependencies: M137 (runtime), M138 (host),
M139 (world), M140 (pulled views), M147 (rough edges), M148 (screen),
M149 (ember), M150 (crew). ADR-0078 §5 grants remain unchanged.

## Bricks, in build order

1. **Contract first.** SDK `Place = Above | Side | Overlay | Rect(Rect)`;
   rect anchors `topLeft`, `topRight`, `bottomLeft`, `bottomRight`, `center`;
   lengths are cells or percentages. `Span { min, ideal, max }` and
   `PaneSize { rows, columns }`. A pane defaults to `["above"]` and the
   legacy row size. Normalize legacy `rows` on read; keep only `size`.
   Golden tests pin ordered places, bottom-right percentages, legacy reads,
   percentages over 100 and reversed bounds. `Viewport` gains `place`.
2. **Pure seating.** `pane::seat(area, wanted, moved) -> Seating` returns
   seats, remaining transcript and collapsed ids. Reserve 60 columns and
   8 rows; one above, one side, three floating. Rects move inside before
   shrinking, and cannot shrink below minimum. Tests cover 200×50 side,
   100×30 above, 60×15 collapse, every anchor in both length units, edge
   clamping, excess docking/floating, existing pane minima and user moves.
3. **Layout and pointer.** `view.rs` composes M124's layout. Panes use
   the transcript area; floats draw newest last. Permission and toasts
   remain above them. Topmost hit testing owns clicks, wheel and selection;
   a pane gesture never starts a transcript selection.
4. **Pulled viewport.** Request `{ place, columns, rows }` uses the seated
   content dimensions. Memo keys include place and, for panes only, rows.
   Resize/move asks again; collapsed panes are never asked (ADR-0079 §5).
5. **Status.** The right edge shows `<title> ▸` for collapsed panes; clicking
   tries overlay. Unasked panes retain `1 pane waiting` and open on click.
6. **Moves.** Focused title `place ▾` and `alt+p` cycle fitting above,
   side, overlay, then declared rects; `?` lists the key.
   Moves are keyed by pane identity in the TUI process, never the session.
7. **Runtime.** `$.ui.open({ id, title, place, size })` in QuickJS/Node.
   Regenerate schema and TypeScript with `BINGO_UPDATE_SCHEMA=1`; scoped
   pre-release amendment recorded in ADR-0075. Counter prefers side/above;
   Corner declares a bottom-right 40%×30% rect and fills it with Raster.
8. **Records.** Accept ADR-0081 §7; dated ADR-0079 §3/§5 note; update
   tui.md, mods-parity §4/§8, TUI guide and mod guide with every place.

## Files

- `crates/bingo-sdk/src/{slot,render}.rs`, `slot/` helpers/tests and fixtures.
- `crates/bingo-surface-tui/src/pane/` (formerly `pane.rs`),
  `{view,layout,pointer,pulled,status,keys}.rs`, affected tests/snapshots.
- `theme.rs`: only the dim file allowlist in `every_token_is_spent_where_the_design_says` (delegate-authorized).
- `crates/bingo-mods/src/`, `sdk/typescript/src/ui.ts`.
- `crates/bingo/tests/mods_ui.rs`, `tests/fixtures/mods/{counter,corner}/`.
- Generated `schema/{rpc,plugin,plugin-2,events}.json`, TypeScript wire/events.
- ADR-0075/0079/0081; `docs/design/{tui,mods-parity-2026-10}.md`;
  `crates/bingo-mods/guide.md`, TUI guide.

## Exit criteria

- [x] SDK contract and pure seating tests failed first for expected reasons,
      then passed; `cargo test -p bingo-sdk slot::`, `cargo test -p
      bingo-sdk render::`, `cargo test -p bingo-surface-tui pane::`.
- [x] Five TestBackend snapshots: side 200×50, above 100×30, collapsed
      60×15, corner rect 120×40, permission above floating pane.
- [x] `cargo test -p bingo --test mods_ui`: seated place/dimensions cross
      RPC and a 200→100-column resize asks Counter at above/100 columns.
- [ ] Own isolated tmux drive: Counter/Corner at 200×50, 100×30, 60×15;
      collapsed click, title cycle and key cycle. Capture results recorded.
- [ ] `scripts/tui-smoke.sh`, all sequential protocol gates, Windows checks
      for each touched crate; only known third-party C-header deferrals.

## Non-goals

Dragging/resizing panes; moves persisted across processes; an unasked pane
taking the screen; covering composer/status/permission; `open.isPlaced`
(the mod reads each viewport and `open` still resolves `{}`).

## Risks

- Pointer routing competes with M92 selection; tests cover topmost wheel,
  click and drag-to-copy. Render memo and M140's in-flight cap bound redraw.
- M140 composes panes in `view.rs`; this owns integration, not `status.rs`.
  M141 shares sdk `render.rs` and TUI `status.rs`; edits stay additive.
  M142 owns the mod kit. Generated files reconcile at merge.
- Latest released tag is v0.6.5; delegate authorizes scoped protocol-2
  amendments with a dated ADR-0075 line. Kernel/sdk name no tool;
  discipline 4c is unchanged. No live or credential tests.
- Mechanical constructors outside Files: core `host/render_queue.rs`
  (`Viewport.place: None`) and TUI `toast.rs` (pane defaults).

- 2026-10-14, delegate: keep `ctrl+p` absent and M148’s test unchanged; use dim for the focused label, add only `pane/draw.rs` to the dim allowlist, keep presence/tokens fixed, and correct/review the 80×24 help snapshots.
- `alt+p` means “place”; `keys.rs` has no binding and composer `alt()` handles only b/f/Enter, so this chord is free and applies only to a focused pane.

## Verified (2026-10-14)

| Command | Result |
|---|---|
| `cargo test -p bingo-sdk pane_ --locked` | FAILED: 0 passed, 4 failed (place/size dropped, invalid bounds accepted). |
| `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk --lib --locked` | ok: 151 passed, 0 failed. |
| `cargo test -p bingo-surface-tui pane::seat --locked` (before implementation) | FAILED: 0 passed, 4 failed (no seats). |
| `cargo test -p bingo-surface-tui pane::integration_tests --locked` (before integration) | FAILED: 0 passed, 1 failed (inline viewport). |
| `cargo test -p bingo-surface-tui pane --locked` | ok: 15 passed, 0 failed; five pane snapshots. |
| `cargo test -p bingo --test mods_ui --locked` | ok: 2 passed, 0 failed. |
| `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk catalogue::tests::the_committed_catalogue_is_generated --locked` | ok: 1 passed. |
| `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-surface-rpc the_committed_schema_is_this_document --lib --locked` | ok: 1 passed. |
| `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc committed_ --lib --locked` | ok: 5 passed. |
| `cargo test -p bingo-plugin-rpc protocol_two_keeps_every_frozen_shape --lib --locked` | ok: 1 passed. |
| `git diff --name-only ba70fdf6 -- crates/bingo-core/src/host/render_queue.rs crates/bingo-surface-tui/src/toast.rs` | Mechanical constructors: `crates/bingo-core/src/host/render_queue.rs`, `crates/bingo-surface-tui/src/toast.rs`. |
| `cargo fmt --all -- --check` | exit 0. |
| `cargo check --workspace --all-targets --locked` | Finished dev profile in 26.46s; exit 0. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Finished dev profile in 26.78s; exit 0. |
| `cargo test --workspace --locked` | FAILED: TUI 1110 passed, 6 failed, 2 ignored; exit 101. |
| `scripts/check_discipline.sh` | Not run: protocol §6 stop. |
| `scripts/budget.sh` | Not run: protocol §6 stop. |
| `cargo deny check` | Not run: protocol §6 stop. |
| `cargo check -p bingo-sdk -p bingo-core -p bingo-surface-tui -p bingo-mods -p bingo --all-targets --target x86_64-pc-windows-msvc --locked` | Not run: protocol §6 stop. |
| `scripts/tui-smoke.sh` | Not run: protocol §6 stop. |
| `tmux -L <isolated-socket> capture-pane -p -t m151` | Not run: protocol §6 stop. |
