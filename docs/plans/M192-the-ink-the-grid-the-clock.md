# M192 — The ink, the grid, the clock

ADR-0097 §3, §10, §11; ADR-0095 as amended. Spec: `docs/design/ember/` (the prototype's
`p3-core.js`; the captures in `scenes/`). After: nothing. Runs beside M194 part A.

## Goal

The bricks every Ember screen is drawn with, and the harness that holds a screen to its capture.
No layout changes: today's screens only take Ember's inks until their milestone redraws them.

## Bricks

1. **Inks** (`theme/ink.rs`). The base tokens are a light and a dark pair each, exactly `BASE`
   in `p3-core.js`: bg, fg, ember, glow, coal, amber, amberGlow, good, bad, plan, auto, code,
   kw, str, num, fn, ty, and crew0–crew5.
   - The derived tokens are blends computed as `palette()` computes them: sRGB per channel,
     JavaScript's `Math.round`, with `pick` and `amberTint` mixed from the already rounded
     `you`. They are muted, dim, faint, you, well, wellDeep, pick, pickBg, addTint, delTint,
     addWord, delWord, badTint, amberTint and glowTint.
   - When the terminal answers, they blend its own fg and bg. When it does not, they use the
     Ember pair of the side it is guessed to be (dark unless something says light).
   - Body text stays `Color::Reset`; the default ground stays the terminal's own (M73).
   - Today's accessors (`theme::glow()`, `raised()`, `ash()` …) keep their names and map onto
     Ember tokens (raised→you, panel→well, rule→faint, ash→muted, presence→ember, mode→plan,
     good_tint→addTint, bad_tint→delTint). M199 deletes the ones nothing calls any more.
2. **Depth** (`theme/depth.rs`). Truecolor takes the hex, 256-colour the nearest index (today's
   `nearest`), 16-colour today's ANSI mapping. `NO_COLOR` has no colour at all: muted, dim and
   faint become DIM, a picked row REVERSED; bold, italic and underline stay.
3. **Detection.** Ask OSC 10 beside OSC 11 (terminal-colorsaurus `color_palette`, already a
   dependency) at start and wherever M71 asks again (`run/look.rs`); a new answer recomputes the
   derived inks. `BINGO_THEME=light|dark` picks a pair outright, with no blend.
4. **Glyphs** (`theme/glyphs.rs`): the `GU`/`GA` tables (lamp, bullet, sep, ok, still, edge,
   base, thin, vbar, quote, caret, ell, arrow, up, down, minus, times, dot, ret, pin, side,
   think, spin) and key names with their ASCII twins (`KEYA`, `keyText`). Today's glyph
   accessors stay until the screens that use them are redrawn.
5. **Grid** (`layout/grid.rs`): SIDE 0, TX 3, VX 5, AX 13, RM 2, MEASURE 100;
   `dock_cap(h) = max(6, ⌊0.66·h⌋ − 2)`; `fit` (cuts with a dim `…`), `pad_to`; `short_path`
   (`pathSegs`: leading folders give way first behind `…/`, the file name last); `fmt_dur`,
   `fmt_k`, `plural`.
6. **Motion** (`clock.rs`): `breathe(now, period)`, `prog(now, at, ms)`, `ease_out`,
   `ease_in_out`, the per-column `shimmer` weight (2.4 s), the caret fade (1.2 s, `caretInk`),
   and the spinner frame (braille 80 ms, ASCII 120 ms, never under 120 ms in tmux,
   `spinner::period`). Each is a pure function of `Now` with a still form under
   `BINGO_MOTION=off` (the prototype's `S.reduced`): `prog` 1, `breathe` 0, the spinner `still`.
7. **Line breaking** (`wrap.rs`). Port `units`/`wrapSegs`:
   - No line starts with a `NO_START` character or ends with a `NO_END` one.
   - A Latin run split only by styling travels as one unit, and only a unit wider than the
     whole line is cut.
   - Spaces after a soft break are dropped.
   - The callers of `wrap::wrap` keep their signatures.
8. **Syntax** (`highlight.rs`). The syntect theme is built from kw, str, num, fn and ty, with
   comments dim italic and everything else fg, as the captures read.
9. **Harness** (`test_support/ember.rs`).
   - `Capture::load(name)` reads `<name>.txt`, `<name>.ink` and the capture's `index.json`
     entry (size, moment, flags, keys) from `docs/design/ember/scenes/` at test time.
   - `Band::{Stream, Dock, Footer, Frame}` splits a capture into bands:
     - the footer is the last row;
     - the dock is the run of rows above the footer whose ground covers the whole width;
     - the stream is everything above the dock.
   - A rendered buffer is split by the frame's own regions. A stream band can be rendered alone
     at the capture's stream size, so stream tests need not wait for M193, and dock tests need
     not wait for M194.
   - `assert_band(capture, band, buffer)` compares as follows:
     - Characters match exactly. A wide character is written once and fills two cells.
     - A spinner frame matches any frame of the same set.
     - fg and bg are compared wherever the capture names a token. `Color::Reset` reads as `fg`
       for a foreground and as `bg` for a ground. kw/str/num/fn/ty and hex blends are compared
       by character only.
     - On failure it prints both bands' rows side by side, a caret under the first differing
       cell, and that row's ink runs.
   - It renders with one look: truecolor, light, detected fg `#3a342e` and bg `#f9f4f2`, so
     every derived ink equals the prototype's.
   - `moved(a, b)` lists the cells whose character or ink differ between two frames. The
     milestones that own a moving scene use it to hold the motion budget (only the side light,
     the comet and the spinner move while a turn works).
10. **Views speak Ember** (`bingo-sdk` `view/style.rs`; `views/text.rs`, `views/flex.rs`). Mods
    draw `View` trees, so the vocabulary names Ember's tokens. Every later plan's mod drawing
    uses it.
    - `TextStyle.color` takes any ink name (fg, muted, dim, faint, ember, glow, coal, amber,
      amberGlow, good, bad, plan, auto, code, crew0–crew5, kw, str, num, fn, ty) or `#rrggbb`.
      The old `ash`, `presence` and `attention` keep their meaning.
    - `TextStyle.ground` and `BoxStyle.ground` name a ground (you, well, wellDeep, pick, pickBg
      and the tints), filled across the box's width.
    - `TextStyle.motion` declares what moves: `breathe { with, periodMs, depth }`, `spin`, or
      `fade { from, atMs, ms }` (from an ink to its own, starting at a wall-clock instant). The
      surface runs it on its clock, wakes through the deadline scheduler, and draws the still
      form under `BINGO_MOTION=off`. A mod never animates by redrawing.
    - All of it is additive. The schemas, `wire.ts` and the TS SDK types regenerate, and an
      unknown name draws plain, never an error.

## Files

- `crates/bingo-surface-tui/src/theme.rs` → `theme/{mod,ink,depth,glyphs,ground}.rs` (accessors
  unchanged; `ground` is M71's blend).
- `src/{clock.rs,spinner.rs,wrap.rs,highlight.rs,paths.rs,terminal.rs,run/look.rs,render.rs}`.
- New `src/layout/grid.rs`, `src/test_support/ember.rs`; `src/painted.rs` (Ember inks).
- `src/views/{text.rs,flex.rs}`; `crates/bingo-sdk/src/view/style.rs`, the regenerated schemas,
  `wire.ts` and `sdk/typescript`.

## Exit criteria

- [x] Ink test: every base and derived token, light and dark, equals the value `p3-core.js`
      computes. Compute the expected hex once with node from the prototype and write it into
      the test.
- [x] Every one-cell glyph in both tables (all but `sep`, `ell`, `ret`) and every spinner frame
      is one cell (unicode-width), and every ASCII twin is ASCII.
- [x] A view naming each ink and ground draws it; a breathing text differs between two frames
      and is still under `BINGO_MOTION=off`; an unknown name draws plain.
- [x] Each case `docs/design/ember/prototype/wraptest.js` prints passes in Rust, word for word.
- [x] Harness self-test: a buffer hand-built from three rows of `done` passes. Changing one
      character fails with the side-by-side report, and so does changing one ink.
- [x] A `NO_COLOR` render of an existing screen carries no colour; a 256-colour render uses
      indices only.
- [ ] Every existing snapshot keeps its characters, and painted.rs asserts Ember's inks. Gates
      and checks as below.

## How every Ember plan is verified (M192–M200)

- **Gates:** `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`,
  `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo test --workspace --locked`, `scripts/check_discipline.sh`, `scripts/budget.sh`,
  `cargo deny check`.
- **Windows:** `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc` for every
  crate the milestone touches.
- **Captures:** every capture a plan owns gets a `TestBackend` test through the harness, at the
  capture's size, moment and flags. Fixtures are ported from `prototype/p4b-data.js`, and the
  state is built the way a session builds it (frames folded with `SessionState::apply`), never
  by poking a view. A band test is used where the plan says so.
- **When a capture is wrong:** change the prototype first, rebuild, re-export, and say so in
  Verified (`docs/design/ember/README.md`). A capture is never edited by hand. A change to what
  a scene shows waits for the person.
- **Drive:** a tmux drive in a harness-owned socket (`tmux -L m<n>-…`) at 100×32 and 80×24,
  with the fake provider and a temporary HOME, data and cwd. Captures go under
  `docs/verification/m<n>/`. Never launch a terminal application or signal a process the run
  did not start.
- **Dependencies:** none is expected. One that is needed takes a line in its plan and a
  `scripts/budget.sh` run.
- **Record:** a Verified section with command output is appended to the plan.

## Non-goals

Any screen's layout or keys; the characters of today's snapshots; node or playwright in CI.

## Risks

- The theme split is a large move. Keep every accessor, so no caller changes here.
- OSC 10 can answer after the first frame. That frame uses the guess, and the answer re-blends.
- A rounding drift shows as an ink mismatch on every scene, so the ink test pins every token.
- M194 part A regenerates the same schema files. Whichever lands second regenerates them after
  its rebase, never merges them by hand.

## Verified

- [x] Inks — `cargo test --workspace --locked`: `every_light_prototype_ink_matches ... ok`; `every_dark_prototype_ink_matches ... ok` (76 values computed once with node, embedded in Rust).
- [x] Glyphs — same command: `prototype_glyphs_have_their_documented_cell_widths ... ok`; only ASCII `->` has the additional two-cell exemption.
- [x] View vocabulary/motion — same command: `every_ember_ink_is_available_to_views ... ok`; `every_ember_ground_fills_text_and_boxes ... ok`; `text_motion_uses_the_frame_clock_and_has_a_still_form ... ok`; unknown names/kinds pass.
- [x] Wrapping — same command: `every_wraptest_case_matches_word_for_word ... ok`; `brackets_styled_words_long_units_and_newlines_match_the_prototype ... ok`.
- [x] Harness — same command: `three_hand_built_done_rows_pass_and_each_failure_explains_its_cell ... ok`; `every_capture_loads_its_geometry_metadata_and_ink_runs ... ok` (105 captures).
- [x] Depth — same command: `existing_screen_uses_ember_inks_and_respects_terminal_depth ... ok` (NO_COLOR, indexed and truecolor).
- [ ] Snapshot/gates/platform criterion — `python3 docs/verification/m192/check-snapshots.py`: `PASS: four syntax snapshots retain every character after removing ink annotations`; `PASS: the only other snapshot change is palette values`; `PASS: Cargo.lock, prototype and all scene captures are unchanged`. Native gates pass; Windows limitations below remain.

### Review fixes

- Probe/rounding/alignment — workspace tests: `palette_probe_keeps_a_background_only_answer ... ok`; `grid_and_formatters_match_the_prototype ... ok` (all six rounding cases).
- Motion/depth/inheritance — workspace tests: `breathe_and_fade_resolve_inks_before_converting_depth ... ok`; `nested_motion_uses_its_inherited_ink_and_unknown_motion_is_still ... ok`; `nested_text_motion_keeps_its_clock_and_deadline ... ok`.
- Explicit clocks/visible memo — workspace tests: `a_cached_mod_row_animates_without_a_new_view_or_event ... ok`; `a_breathing_row_off_screen_neither_redraws_nor_schedules ... ok`; `visible_cache_ranges_skip_inline_replacements ... ok`. `PASS`, `track`, `observe`, `Restore`, `motion::now` and `views::next_change` are deleted; no second view deadline walk remains.
- Capture rules — workspace tests: `ordinary_punctuation_is_not_a_spinner ... ok`; `wide_cells_reset_inks_and_spinner_sets_compare_as_terminal_cells ... ok`; `stream_regions_include_activity_and_strip ... ok`.
- Palette/shared mechanisms — workspace tests: `terminal_foreground_and_background_reblend_rounded_grounds ... ok`; glyph and ramp tests pass. Private palette construction, shared blends/ANSI/spinner tables, wrap docs/saturation, highlight copy and item-level grid allowances pass Clippy/discipline.
- Wire — `BINGO_UPDATE_SCHEMA=1 cargo test -p <crate> --locked` for `bingo-sdk`, `bingo-plugin-rpc`, `bingo-surface-rpc`: exit 0; the same environment with `cargo test -p bingo-mods --locked dollar::tests::schema_and_types_are_generated_from_the_same_rows -- --exact`: `1 passed; 0 failed`. No generated files edited by hand.
- Unknown motion — workspace tests: `unknown_motion_keeps_the_view_readable ... ok`; `unknown_motion_is_not_an_authoring_variant ... ok`. `#[serde(other)] Unknown` draws still; `#[schemars(skip)]` excludes it from schemas and generated TypeScript.

### Gate evidence

- `cargo fmt --all -- --check`: exit 0, no output.
- `cargo check --workspace --all-targets --locked`: `Finished dev profile ... in 2.29s`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: `Finished dev profile ... in 4.25s`.
- `env -u OPENAI_API_KEY -u OPENAI_BASE_URL -u ANTHROPIC_API_KEY -u ANTHROPIC_BASE_URL cargo test --workspace --locked`: 120 result lines, **6387 passed; 0 failed; 0 ignored**; TUI: `1397 passed; 0 failed`.
- `scripts/check_discipline.sh`: `discipline ok` (advisory warnings only).
- `scripts/budget.sh`: `dependencies (unique, normal): 423 (max 423)`; `warm cargo check -p bingo-core: 0s`; `relink isolation: touching the TUI recompiled 0 crates for core`; `target/debug: 13 GB (soft max 5)`; `budget ok`.
- `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.
- `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc`: `bingo-sdk` / `bingo-surface-rpc`: `Finished dev profile` (7.09s / 9.01s), exit 0.
- Same Windows command: `bingo-surface-tui` / `bingo-plugin-rpc`: exit 101, `aws-lc-sys`: `fatal error: 'stdlib.h' file not found`, `'windows.h' file not found`; `bingo-mods` additionally fails in `rquickjs-sys`: `clang: error: no such file or directory: '/std:c11'`. Known host C toolchain limitation; CI's Windows job is the backstop. No bypasses.
- `cargo build -p bingo --locked`: `Finished dev profile ... in 1m 14s`; `TUI_SMOKE_SOCKET_PREFIX=m192-drive-smoke scripts/tui-smoke.sh`: `tui-smoke ok`.
- `python3 docs/verification/m192/drive.py`: `PASS 100x32: fake-provider reply captured; private HOME/data/cwd`; `PASS 80x24: fake-provider reply captured; private HOME/data/cwd`; both `CLEAN` lines confirm owned servers stopped and temporary files removed. Updated `.txt`/`.ansi` captures in `docs/verification/m192/`; both contain Ember glow `38;2;239;122;44`.

### Departures and retained M199 work

- Prototype wins: only `GA.arrow` (`->`) is two cells; `GU.arrow` (`→`) is checked. Old crew hue/contrast assertions now check membership in the exact Ember palette.
- Review narrows spinner tolerance to named capture cells and makes 16-colour/NO_COLOR motion still. Supporting drawing/cache files extend the abbreviated plan file list; screens keep their layout.
- Motion remains boxed to avoid enlarging SDK enums. `Drawing` carries lines/deadlines explicitly; geometry-only passes and `render` stay still. No retained `views::next_change` caller.
- Retained for M199: legacy `breath` beside `breathe`, `paths::elide` beside `short_path`, and the old accessor names until their screens are redrawn.
- Initial review tests `the_item_that_is_still_arriving_is_the_only_one_drawn_again` and `a_failure_flares_and_cools_into_the_words_behind_it` failed identically alone; fixtures now advance to the next deadline and sample the memo chronologically. Both full workspace runs pass.
- Earlier provider-key test failure reproduced alone with ambient credentials; subsequent test subprocesses unset provider variables. No provider code changed.
- Omitted: only host-blocked Windows verification. No new dependency, later milestone layout work, prototype edit or scene capture edit.
