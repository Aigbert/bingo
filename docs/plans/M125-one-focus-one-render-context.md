# M125 — One focus, one render context

## Goal

Audit-2026-10 §4's second half: replace independent input owners and
process-global theme/graphics detection with values. Both keys and pointer
resolve `Ui.focus` through one function. A run builds `Render` from its
probe inputs and hands it to M124's layout/paint boundary. No dependency,
sdk change or redesign of the rail/panel that M148 removes (ADR-0081).

## Bricks, in build order

1. Contract tests first: every focus variant owns all key classes and a
   left press; interactions outrank only the composer, then its menu.
   Detection covers environment, OSC 11, XTVERSION, graphics, late replies,
   and parallel draws in two looks, through production code.
2. `Focus`: `Composer`, `Intro(Playing)`, `Select(Run)`, `Search(Search)`,
   `Layer(Layer)`, `Card(CardId)`. It replaces `intro`, `select.run`,
   `search`, `layer` and the rail focus. Layer retains reveal timing;
   closing gives input back while drawing its remaining frames.
   The open interaction and menu stay derived, never stored owners.
   Escape, exit, view dismissal and dragging remain gestures.
3. Both `on_key` and `on_mouse` resolve `focus::owner(&Ui, &Tree)`.
   Existing handlers retain bindings. Mouse-order changes are tested and
   listed in commit bodies. Help's owner permits typing through it.
4. `Render { look: Theme, graphics: Graphics, motion: bool }` derives
   ASCII from its glyph table. Pure theme/graphics detection reads the
   run's environment/probe inputs. I/O stays at `terminal.rs` and its probe
   helper before taking the terminal. Late replies update run inputs.
5. `layout()` and `paint()` take `&Render`; `Render::scope` supplies
   `render::current()` to existing style helpers, restoring on unwind.
   Outside a scope, both builds use the resting look. Block/highlighter
   memo keys already read those helpers, so context changes invalidate them.
6. Delete detection's `ASK`, `GROUND`, `HEARD`, `OVERRIDE`, `saying` and
   production/test twins. Export `PROBE_THROUGH` in both builds. Fixtures
   use render scopes and production detection rather than alternate logic.
7. Update `docs/design/tui.md` §4/§7 and ADR-0002 beside M124's note.

## Files

All source paths below are under `crates/bingo-surface-tui/src/`:

- New `focus.rs`, `render.rs`, `terminal/probe.rs`, `ui/controls.rs`.
- `ui.rs`, `input.rs`, `input/switcher.rs`, `pointer.rs`, `select.rs`,
  `view.rs`, `pager.rs`, `layout.rs`, `layout/{deadline,tests}.rs`.
- `theme.rs`, `graphics/{mod,tmux}.rs`, `clock.rs`, `terminal.rs`, `lib.rs`,
  `run.rs`, `run/{look,notices,opening,showing}.rs`, `highlight.rs`.
- Fixtures: `doubles.rs`, `painted.rs`, `test_support.rs`,
  `test_support/rendered.rs`, `screens/{colours,opening}.rs`, `motion.rs`.
- `docs/design/tui.md`, `docs/adr/0002-event-stream.md`, this plan.

## Exit criteria

- [x] `cargo test -p bingo-surface-tui focus` and
      `cargo test -p bingo-surface-tui render` pass; initial failures pasted.
- [x] No theme/graphics detection twins or process globals remain.
      M124's `layout/count.rs` instrumentation and `transcript::saying`
      are unrelated and stay. Both input paths call `focus::owner`.
- [x] Production detection tests cover late graphics and background changes;
      parallel scoped draws do not leak looks.
- [x] Existing screens/motion assertions pass; fixture construction changes
      mechanically. Pointer-order changes have regression coverage.
- [x] `scripts/tui-smoke.sh` passes. An isolated tmux drive exercises layers
      and clicks with both light and dark startup.
- [x] Native protocol gates.
- [ ] Windows cross-check: blocked by SDK C headers; deferred per step 7.

## Non-goals

New bindings or configurable keys; changing the probe questions or timing;
passing `&Render` through hundreds of style helpers; redesigning the rail
or panel; live or credential tests; another milestone; push, merge or PR.

## Risks

- Removing `Layer::is` exposed `Ui` to the discipline scanner, which had
  stopped at that test-only method. Group input state in `ui/controls.rs`
  instead of hiding the 29-field violation behind another test marker.

- Old flags could coexist. One explicit focus replaces another; pager's
  internal search remains nested within the pager. Closing is visual residue.
- Pointer precedence changes must be named and tested; a stale last frame's
  targets must not activate something behind today's owner.
- Scope nesting, restoration and thread isolation need explicit coverage.
- Existing layers do not all have click-to-open/close bindings: only the
  switcher rows and interaction options act on clicks. The tmux drive checks
  those actions and that other layers consume clicks; it adds no bindings.
- Known unrelated ACP bridge refusal failure may be skipped if it occurs,
  with the exact name and output recorded. Windows SDK C-header failures
  are left to Windows CI per protocol step 7; Rust errors still stop.

## Re-verified — 2026-10-13 at f3ed55b9

- Clean `m125-one-focus`; M124 `layout(previous, tree, ui, size, now)`
  returns `Layout`; `paint(layout, frame)` copies its canvas.
- The audited keyboard/mouse orders and five independent owners remain.
  Layer also owns closing/reveal state; preserve it inside `Focus`.
- Detection twins remain theme 10, graphics 8, lib 1. M124 added three
  unrelated instrumentation twins; the original broad grep was stale.
- Added actual consumers/fixtures and the terminal probe's tmux helpers to
  Files. M134-shared `lib.rs`, `pager.rs`, `test_support.rs` edits are local;
  no edits to its `screens.rs`, transcript, fold, thoughts or ACP files.
- The original two cargo test filters were invalid; run them separately.
  ASCII is derived rather than duplicating theme glyphs. No new dependency.

## Verified — 2026-10-13

- Contract red, before implementation (`6192b538`):
  `cargo test -p bingo-surface-tui focus_has_one_owner --locked`:
  `test result: FAILED. 0 passed; 1 failed` (`Composer` instead of `Intro`).
  `cargo test -p bingo-surface-tui render::tests --locked`:
  `test result: FAILED. 0 passed; 2 failed` (detection and scoped look).
- `cargo test -p bingo-surface-tui focus --locked`:
  `test result: ok. 10 passed; 0 failed; 0 ignored`.
  `cargo test -p bingo-surface-tui render --locked`:
  `test result: ok. 5 passed; 0 failed; 0 ignored`.
- `cargo fmt --all -- --check`: exit 0, no output.
  `cargo check --workspace --all-targets --locked`:
  ``Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.52s``.
  `cargo clippy --workspace --all-targets --locked -- -D warnings`:
  ``Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.73s``.
- Let `a=a_proxy_with_a_token_this_run_never_minted_gets_nothing` and
  `b=a_token_may_be_dialled_again_once_its_stream_has_closed`.
  `INSTA_UPDATE=no cargo test --workspace --locked -- --skip $a --skip $b`:
  aggregate `5158 passed; 0 failed; 4 ignored; 2 filtered out`; TUI output:
  `test result: ok. 1133 passed; 0 failed; 2 ignored`; PTY: `16 passed`.
  `cargo test -p bingo --test acp_bridge --locked $a -- --exact --test-threads=1`:
  `test result: ok. 1 passed; 0 failed`.
  `cargo test -p bingo-provider-acp --test bridge --locked $b -- --exact --test-threads=1`:
  `test result: ok. 1 passed; 0 failed`. The known no-turn refusal test was
  `ok`, so it was not skipped. No live/credential tests or ignored tests enabled.
- `scripts/check_discipline.sh`: `discipline ok` (existing advisories).
  `scripts/budget.sh`: `dependencies (unique, normal): 353 (max 353)`;
  `relink isolation: touching the TUI recompiled 0 crates for core`; `budget ok`.
  `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.
- `cargo check -p bingo-surface-tui --all-targets --target x86_64-pc-windows-msvc`:
  `error: failed to run custom build command for aws-lc-sys v0.44.0`;
  `fatal error: 'stdlib.h' file not found`; `'windows.h' file not found`.
  TUI Windows Rust compilation remains for Windows CI; no toolchain installed.
- `PATH=/tmp/m125-tools:$PATH scripts/tui-smoke.sh`: `tui-smoke ok`.
  The wrapper mapped only its tmux socket to an isolated agent-owned server.
  `bash /tmp/m125-drive.sh`: `light: all layers and click ownership passed`;
  `dark: all layers and click ownership passed`; `m125 tmux drive ok`.
  Captured raised tints: light `48;2;238;232;221`, dark `48;2;33;29;23`.
- Static checks: theme/graphics/lib detection globals and twins `0`; input
  and pointer each have `1 owner dispatch; legacy flags: False`.
  Screens/motion snapshots unchanged. Fixed the drag fixture to start a
  selection; replacing focus now removes its deadline. Both tmux servers
  and their temporary homes were removed. No push, merge or PR.
