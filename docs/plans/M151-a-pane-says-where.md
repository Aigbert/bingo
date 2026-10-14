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
      RPC and resize; coexistence passes with unchanged assertions.
- [x] Own isolated tmux drive: Counter/Corner at 200×50, 100×30, 60×15;
      collapsed click, title cycle, alt+p cycle and Corner raster pass
      without foreign-handler notices. Capture results recorded.
- [x] `scripts/tui-smoke.sh` and all sequential native gates pass; SDK/core
      MSVC checks pass. TUI/mods/bin defer only for known C headers to Windows CI.

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
- Windows CI owns TUI (`aws-lc-sys`) and mods/bin (`aws-lc-sys`, `rquickjs-sys`) C-header deferrals; no Rust error reported or toolchain installed.
- Mechanical constructors outside Files: core `host/render_queue.rs`
  (`Viewport.place: None`) and TUI `toast.rs` (pane defaults).

- 2026-10-14, delegate: keep `ctrl+p` absent and M148’s test unchanged; use dim for the focused label, add only `pane/draw.rs` to the dim allowlist, keep presence/tokens fixed, and correct/review the 80×24 help snapshots.
- `alt+p` means “place”; `keys.rs` has no binding and composer `alt()` handles only b/f/Enter, so this chord is free and applies only to a focused pane.

- 2026-10-14: merged dev fix `5446e403` in `7402c179`; inherited-handler coexistence regression passes with its assertions unchanged.

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
| `cargo check --workspace --all-targets --locked` | Finished dev profile in 26.23s; exit 0 (post-merge). |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Finished dev profile in 29.24s; exit 0 (post-merge). |
| `cargo test --workspace --locked` (before delegate corrections) | FAILED: TUI 1110 passed, 6 failed, 2 ignored; exit 101. |
| `scripts/check_discipline.sh` | `discipline ok`; existing advisories; rule 4c unchanged. |
| `scripts/budget.sh` | `budget ok`; 411/411 dependencies; core 0s; relink 0; target/debug 10 GB (soft max 5). |
| `cargo deny check` | `advisories ok, bans ok, licenses ok, sources ok`; exit 0. |
| `scripts/tui-smoke.sh` | `tui-smoke ok`; board/activity at 80×24, 120×40, 200×50; Counter at 80×24/200×24; ctrl+p unbound. |
| `tmux -L <isolated-socket> capture-pane -p -t m151` (Counter, post-merge) | 200×50: `Seat: side 64x43`; 100×30: `Seat: above 100x9`; 60×15: `Counter ▸`. |
| `tmux -L <isolated-socket> capture-pane -p -t m151` (click/key, post-merge) | Click collapsed: `overlay ▾`; click title: `Seat: above 200x9`; alt+p: `Seat: side 64x43`. |
| `tmux -L <isolated-socket> capture-pane -p -t m151` (both, post-merge) | 200×50: side/Corner; 100×30: above/Corner; 60×15: Counter collapsed/Corner; raster rendered at every size; no foreign-handler notices. |
| `cargo test -p bingo-surface-tui collapse_click_and_title_and_key_cycle_fit_without_changing_session_data --locked` (before key correction) | FAILED: 0 passed, 1 failed; ctrl+p moved the pane. |
| `INSTA_UPDATE=always cargo test -p bingo-surface-tui --lib --locked` | ok: 1116 passed, 0 failed, 2 ignored; unchanged M148 test and 80×24 footer test pass. |
| `git diff -- crates/bingo-surface-tui/src/snapshots/` (delegate correction) | Reviewed sheet_slide, sheet_close, help_80x30, help_100x30 and help_panel; binding rows only, footer retained. |
| `cargo build -p bingo --locked` | Finished dev profile in 25.81s; exit 0 (post-merge). |
| `cargo test -p bingo --test mods_ui --locked unrelated_pane_mods_pass_through_each_others_handlers_without_notices -- --exact` | FAILED: 0 passed, 1 failed; valid inherited handler rejected: `corner (ui.render): /children/2: handler belongs to another command`. |
| `cargo test -p bingo --test mods_ui --locked` (after merging dev) | ok: 3 passed, 0 failed; coexistence assertions unchanged. |
| `cargo test --workspace --locked -- --skip a_proxy_with_a_token_this_run_never_minted_gets_nothing --skip a_token_nobody_minted_never_gets_a_conversation --skip a_second_concurrent_stream_on_one_token_is_refused --skip a_token_may_be_dialled_again_once_its_stream_has_closed` | exit 0; summed 5311 passed, 0 failed, 5 existing manual/stress ignores; PTY 16 passed. |
| `cargo test -p bingo --test acp_bridge --locked a_proxy_with_a_token_this_run_never_minted_gets_nothing -- --exact --test-threads=1` | ok: 1 passed, 0 failed. |
| `cargo test -p bingo-provider-acp --test bridge --locked token -- --test-threads=1` | ok: 3 passed, 0 failed; the other three excluded token tests. |
| `cargo check -p bingo-sdk --all-targets --target x86_64-pc-windows-msvc --locked` | Finished dev profile in 6.55s; exit 0. |
| `cargo check -p bingo-core --all-targets --target x86_64-pc-windows-msvc --locked` | Finished dev profile in 13.69s; exit 0. |
| `cargo check -p bingo-surface-tui --all-targets --target x86_64-pc-windows-msvc --locked` | exit 101; Windows CI: aws-lc-sys lacks stdlib.h/windows.h. |
| `cargo check -p bingo-mods --all-targets --target x86_64-pc-windows-msvc --locked` | exit 101; rquickjs-sys MSVC flags need Clang cl mode; aws-lc-sys lacks Windows headers. |
| `cargo check -p bingo --all-targets --target x86_64-pc-windows-msvc --locked` | exit 101; rquickjs-sys MSVC flags need Clang cl mode; aws-lc-sys lacks Windows headers. |
| `CC_x86_64_pc_windows_msvc='clang --driver-mode=cl' cargo check -p bingo-mods --all-targets --target x86_64-pc-windows-msvc --locked` | exit 101; Windows CI: rquickjs-sys lacks stdlib.h; aws-lc-sys lacks stdlib.h/windows.h. |
| `CC_x86_64_pc_windows_msvc='clang --driver-mode=cl' cargo check -p bingo --all-targets --target x86_64-pc-windows-msvc --locked` | exit 101; Windows CI: rquickjs-sys lacks stdlib.h; aws-lc-sys lacks stdlib.h/windows.h. |
| `git diff cb2a1f35 -- crates/bingo/tests/mods_ui.rs` | Empty; regression assertions unchanged. |
| `git diff --check` | exit 0. |
