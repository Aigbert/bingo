# M196 part 1 handoff

Bricks 1–6 and 9 are implemented. Bricks 7–8 remain part 2; no palette,
mention, completion, list-window, or bingo-mod-palette source was changed.

## Shared places

- `view.rs`: `compose`, `demand`, and `render_composer` now measure and draw derived
  queue and Activity rows above the input. `Draft` and `composer_above` collect their
  deadlines before clipping. Removed `strip`/`render_strip` and their geometry.
  `menu` and `layer` are unchanged; no sheet-choice branch was added.
- `ui.rs`: toast hold duration, `notify_for`, and the shared 1.5-second exit window.
  `Open` is unchanged. `ui/controls.rs` replaces `esc_armed` with `escaped_at`, adds
  queue arrival clocks and the chip-paste helper; `activity_hidden` is retained.
- `focus.rs`: unchanged, including `Focus`, `Owner`, and `owner()`.
- `keys.rs`: Escape/Tab/Interrupt policy and policy tests only. `Binding` and
  `BINDINGS` are unchanged. M198 must refresh the `esc`, `ctrl+c`, and `up/down`
  descriptions. The excluded palette test that equates the old Escape description
  with every policy row currently fails; no excluded test was edited.
- `input.rs`: timed Escape/Ctrl+C chains, history/withdrawal precedence, legacy
  ESC CR handling, and idle-only `?`. `twice` still opens the existing rewind in
  one function. `?` still runs `/help`. The `ctrl+t` binding is unchanged.
- `activity.rs`: queue rendering moved to `queue_rows.rs`; Activity placements moved
  into the dock without the connector. The `activity_hidden` read remains for M198
  to remove if it deletes that flag. AbovePrompt code is unchanged.
- M202 overlap: only `transcript.rs::default_item_lines` (shell arm) and
  `transcript/band.rs::{draw,text,additions,Band::of}` changed. Module declarations
  expose shell timing and add chips/notices. `item_block`, `called`, `prompt_lines`,
  `turns::step`, `transcript/steps.rs`, `slots::{heading,step_heading}`, and
  `component/live.rs::prepare` are untouched. No merge was attempted.

## Departures and remaining work

- Brick 5 asks for a shell-specific placeholder. The prototype's `placeholder()`
  has no such case: its normal placeholder is retained.
- Shell output no longer folds. `OUTPUT_ROWS`/`EXPAND` remain for action output,
  generic tool output, and generic quiet notices; these are outside this change.
- The background completion's first line follows the prototype; existing details
  remain below it for the model. `transcript/notice.rs` draws the faint divider
  and returns its 400 ms arrival deadline through `said::lines`. Legacy persisted notices retain their generic view.
- Part 2 owns palette, palette-st, palette-ran, skills, skills-review, skills-sent,
  mention, mention-inserted, small-palette, ascii-palette, and palette ranking.
- No new dependency, package, kernel fact, prototype edit, or scene-capture edit.
- Native workspace tests retain the excluded palette-test failure. An unrelated ACP
  journaling assertion failed in the final suite and passed on its one isolated retry.
  Windows TUI/binary checks are blocked before this code by missing C headers and
  the QuickJS MSVC C toolchain. The bash crate cross-check passes.

## Reproduction

`node docs/verification/m196/fixtures.cjs` exports state from each capture's own
prototype scene, key sequence, geometry, and clock. The Rust test folds session
frames, renders a full TestBackend frame, and compares it with the sealed capture.
Only the named spinner cells in live-steered and steer-landed are tolerated.

`docs/verification/m196/drive.sh` owns only tmux socket `m196-drive`. It refuses
an existing server, uses fake-provider sessions and temporary HOME/data/cwd, and
removes its server and temporary files on every exit. Its clipboard double is
visible only to the child process. Text and ANSI captures are beside this file.
