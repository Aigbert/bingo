# M196 part 1 handoff

Bricks 1–6 and 9 are implemented. Bricks 7–8 remain part 2. Round-2 evidence
and the acting timeline are in [round-2.md](round-2.md). The sole mod-palette edit
is the explicitly authorized `tone: Neutral` constructor field.

## Shared places

- `view.rs`: `compose`, `demand`, and `render_composer` now measure and draw derived
  queue and Activity rows above the input. `Draft` and `composer_above` collect their
  deadlines before clipping; `Draft::rows()` serves both demand and drawing. Removed `strip`/`render_strip` and their geometry.
  `menu` and `layer` are unchanged; no sheet-choice branch was added.
- `ui.rs`: notices hold parts and tone, plus hold duration and `notify_for`.
  The exit window remains shared with the key policy.
  `Open` is unchanged. `ui/controls.rs` replaces `esc_armed` with `escaped_at`, adds
  queue arrival clocks and the chip-paste helper; `activity_hidden` is retained.
- `focus.rs`: unchanged, including `Focus`, `Owner`, and `owner()`.
- `keys.rs`: Escape/Tab/Interrupt policy and tests; round 2 updates only the
  authorized Escape BINDINGS description and seven palette snapshot rows.
  M198 still owns `ctrl+c` and `up/down` wording and the future keys-sheet copy.
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
- ADR-0098 keeps the background headline separate from unchanged model text.
  `transcript/notice.rs` draws its divider and 400 ms arrival fade. User items
  without a headline retain their earlier rendering; no prose is parsed for marks.
- Part 2 owns palette, palette-st, palette-ran, skills, skills-review, skills-sent,
  mention, mention-inserted, small-palette, ascii-palette, and palette ranking.
- No new dependency or package; one kernel fact, ADR-0098: a delivery may carry its headline.
- No prototype or sealed scene capture was edited.

## Reproduction

`node docs/verification/m194b/fixtures.cjs` is the sole turn exporter.
The acting test starts a real Run with clock, clipboard, host and session doubles;
keys enter through terminal_event, sends are checked, and kernel answers enter
through frame/reply. Six real-Host proofs check the answer helpers.
Only the named spinner cells in live-steered and steer-landed are exempt.

`python3 docs/verification/m196/drive.py` owns only tmux socket `m196-drive`. It refuses
an existing server, uses fake-provider sessions and temporary HOME/data/cwd, and
removes its server and temporary files on every exit. Its clipboard double is
visible only to the child process. Text and ANSI captures are beside this file.
