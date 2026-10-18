# M193 — Three bands

ADR-0097 §1, §2, §10; ADR-0095 as amended (§4, §5 stand). Spec: `docs/design/ember/`
(`p6-dock.js`: `dockLines` … `pillLine`; `p8-page.js` scene setups). After: M192. Runs beside
M194 part B.

## Goal

The terminal is three bands: the stream (bottom-anchored), the dock (where you act; its top row
is the edge line), and the footer (one row). The header row, the notice row, the `KeyBar` row,
the status line and the crew strip go, and what they said moves to the footer or the window
title.

## Bricks

1. **Regions** (`frame::regions`).
   - The footer is the last row, the dock sits above it, and the stream takes what is left.
   - The dock's rows are the edge line, its body and one pad row, all on the `you` ground across
     the whole width. The body is capped at `dock_cap(H)`, keeping its last rows.
   - **Growth** (`growDock`, `shownRows`): a growing dock rises one row per 14 ms (ease out),
     and a shrinking one lets go at once. A capture's still is the settled height, and so is
     reduced motion.
2. **The viewport** (`viewport`, `pillLine`, `scrollbar`).
   - The stream is drawn bottom-anchored, with a blank row after the last turn.
   - Scrolled up, the last stream row is the pill (` ↓ N lines below ` on `you`, centred, a
     click goes to the end).
   - The scrollbar is drawn at the right edge as the prototype draws it.
   - Today's `foot.rs` pill and `scroll.rs` keep their logic; only the drawing moves.
3. **The edge line** (`edgeLine`, `cometSegs`). Its state is derived at render time from the
   session, never stored beside it.
   - **Idle:** nothing is drawn.
   - **Live:** the comet takes 2.2 s per pass, length `min(22, round(W·0.22))`, in the view's ink
     (glow at home), over faint `▔`.
   - **An ask or a question:** amberGlow breathing over 1.4 s. A dangerous ask is bad and still
     (the danger fact is M197's).
   - **Retry:** `mix(amber, you, .45)`, still.
   - **A mode change** flashes the mode's colour for 900 ms.
   - **A turn's end** flashes good or bad for 1.1 s. It reads from `last_turn`, never a timer of
     its own.
   - **Reduced motion:** `mix(faint, glow, .4)`, still.
4. **The composer rows** (`composerLines`, `inputSegs`, `placeholder`, `inputRows`).
   - The input sits at column 3 with the ember caret `▍`, which fades (`caretInk`) and never
     blinks.
   - The placeholder depends on the state, using the seven strings in SPEC.md "Acting". The
     ones for teammates and rooms read their names from the viewed session.
   - The editing core, `on_key`, and the composer's own behaviour are unchanged: M196 changes
     what keys do.
   - The palette and mention lists keep their current place until M196 moves them into the dock.
5. **The footer** (`footerLine`, `ambientSegs`, `crewSegs`, `hintSegs`, `keyHints`, `homeHints`,
   `toastSegs`).
   - **Left, in order:** the model id (muted) and effort (dim); the mode when it is not default,
     in its colour (accept edits auto, plan plan, bypass bad; ADR-0095 §5); `context N%` from
     70% (amber from 80%); crew lamps with `N agents` while at home; `N shells`; and
     `wakes in X` when a plugin places that slot (M199 brings the slot back).
   - **Toasts** replace the left side for 2.4 s: fading in over 160 ms, out over 300 ms, on the
     wall clock. A toast whose moment has passed is dropped, never replayed (ADR-0095 §2).
   - **Right:** the keys that work right now. These are the `KeyBar` site's pairs, drawn here.
     M193 owns the home and scroll pairs:
     - `⏎ steer`, `tab queue`, `esc stop` while a turn runs with a draft. Today's
       "⏎ queue" is wrong.
     - `↑ edit queued` (when something is queued), `ctrl+b background` (when a bash step
       runs), `esc stop` while a turn runs without a draft.
     - `⏎ send`, `⇧⏎ newline` with a draft.
     - `ctrl+o details`, `/ commands`, `? keys` once the conversation has a turn, and
       `/ commands`, `@ files`, `? keys` before it.
     - `end latest`, `ctrl+f find` while scrolled.

     Each later dock kind adds its own pairs in its milestone.
   - **Narrow:** right pairs drop from the left, then left facts drop from the end. The model is
     the last to go.
6. **The window title.** OSC 2 writes `<title> — <dir>`, and away from home
   `<session> › <name>`. It is restored on exit. The header row's facts (branch, session id) go;
   the branch stays a context fact for the sheets (M198).
7. **The mode** (shift+tab, ADR-0095 §5). It flashes the edge and toasts the mode word in its
   colour, bold, then its description.
8. **What goes:** `view/header.rs`, the notice row, the `KeyBar` row (`keybar.rs` keeps the pair
   logic and `toast.rs` its clock), `status.rs`'s line (its facts feed the footer), the crew
   strip (`crew.rs` drawing; M200 brings crew back as views), and `opening.rs`'s header
   animation. Each deleted snapshot is named in Verified.

## Files

`crates/bingo-surface-tui/src/{frame.rs,view.rs,view/,keybar.rs,keybar/,toast.rs,
pill.rs,composer.rs,crew.rs,welcome.rs,opening.rs,run/look.rs,terminal.rs}`; new
`src/dock.rs` (regions, growth, edge) and `src/footer.rs`, one noun each; `bingo-mod-keybar`
(it draws pairs into the footer row). Round 2: SDK action/keybar contracts, permissions mode
meanings, and generated schemas/TypeScript.

## Exit criteria

- [x] The dock and footer bands of `live`, `done`, `mode`, `mode-later`, `reduced-mode`,
      `reduced-live`, `ascii-live`, `ascii-done`, `narrow`, `small-done` and `welcome` pass the
      harness.
- [x] Edge line states at named instants: the comet at three moments, the 1.1 s end flash at
      0/550/1100 ms, the mode flash, reduced. Each is a band test or a `moved` assertion.
- [x] Growth: a dock going from 3 to 14 rows shows the intermediate heights at 14 ms steps;
      shrinking is immediate.
- [ ] Whichever of M193 and M194 lands second also passes the full frame of `live`, `done`,
      `ascii-live`, `ascii-done`, `reduced-live`, `small-done`, `mode` and `mode-later`.
- [x] `scripts/tui-smoke.sh` waits for a footer string, not `/ commands` in the old `KeyBar`
      row, and passes.
- [x] Gates and Windows check commands as in M192 (native-toolchain blockers recorded below).
- [x] tmux drive: idle, live, mode cycle, 80×24, a toast.

## Non-goals

The stream's rows (M194); the palette, mentions and queued rows in the dock, and the key
behaviour (M196); asks (M197); sheets (M198); view segments and ctrl+g (M200); the welcome
wordmark and the exit receipt (M199).

## Risks

- **Many snapshots pin the header, `KeyBar` and status rows.** Re-pin them in bulk only after
  reading a sample, and list in Verified what changed and why.
- **The OSC 2 title can leak into a pty test's stdout.** Write it only when stdout is a
  terminal and the TUI owns it.
- **Growth animation.** Keep the deadline scheduler (`layout/deadline.rs`) as the only source of
  wakes, so an idle frame costs nothing.

## Verified

- [x] Bands — `cargo test --workspace --locked`: `m193_capture_dock_and_footer_bands ... ok` (all 11 named captures; dock and footer, exact size/moment/flags, folded session frames).
- [x] Edge — same command: `m193_edge_comet_end_mode_and_reduced_have_deadlines ... ok` (three comet samples; end 0/550/1100 ms; mode and reduced).
- [x] Growth — same command: `growth_samples_each_fourteen_ms_step_and_shrinks_at_once ... ok` (3→14, every 14 ms, immediate shrink).
- [x] Smoke/drive — `TUI_SMOKE_SOCKET_PREFIX=m193-drive-smoke scripts/tui-smoke.sh`: `tui-smoke ok`; `python3 docs/verification/m193/drive.py`: `PASS 100x32` / `PASS 80x24`, both `CLEAN` (idle/live/modes/toast; fake provider, private HOME/data/cwd; captures in `docs/verification/m193/`).
- [x] `cargo fmt --all -- --check`: exit 0; `cargo check --workspace --all-targets --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings`: `Finished dev profile` each.
- [x] `cargo test --workspace --locked` (provider credentials unset): 123 result lines, **6461 passed; 0 failed; 0 ignored**; TUI 1405, PTY 19. `cargo build -p bingo --locked`: exit 0.
- [x] `scripts/check_discipline.sh`: `discipline ok`; `scripts/budget.sh`: `dependencies (unique, normal): 423 (max 423)`, `budget ok` (disk warning); `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.
- [x] Windows — `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc`: SDK, core, mod-hello, mod-keybar and tool-bash each `Finished dev profile`, exit 0.
- [ ] Same Windows check for permissions/TUI/bingo: exit 101. Permissions' tree-sitter lacks `stdio.h`/`stdlib.h`; TUI's aws-lc-sys lacks `stdlib.h`/`windows.h`; bingo's rquickjs-sys rejects `/std:c11`. Native-toolchain checks remain owed; no bypass.
- [ ] Full frames: explicitly deferred by the task until the reviewer assigns the second-to-land criterion; M194's stream rows/fold retirement remain untouched.
- Departures: prototype mode toast holds 2800 ms plus 300 ms fade (ordinary toast 2400+300); brick 5 only names 2400. `p8-page.js::paintBar` retains ` — <dir>` in child titles, unlike brick 6/ADR's shorter spelling; prototype followed.
- Round 2 departure: the caret rests after 10 s without a key (ADR-0097 §10), then schedules nothing; no capture reaches that age. The next key wakes it. The needs-you lead moved forward from M200; view segments/ctrl+g remain there.
- Harness fix: M192 omitted snapshot/capture's `T0 = 10_000_000`; `Capture::now` now includes that origin, needed for the exact `mode-later` caret ink. Scene captures and prototype are unchanged.
- Smoke found the retired opening's invisible focus swallowing the first character after a pane closed; removed it and pinned `the_first_key_is_typed ... ok`. Two PTY expectations changed for the embedded caret and footer clipping; no unrelated test failures remained.
- Snapshots: sampled before bulk re-pin; 408 updated for removed header/notice/key/status/crew rows, bottom anchoring, embedded caret and footer ink. Twelve deleted because the old opening animation is gone; M199 owns the replacement wordmark.
- Deleted under `crates/bingo-surface-tui/src/opening/snapshots/`, prefix `bingo_surface_tui__opening__storyboard__`: `shot_0_0_80.snap`, `shot_0_5_80.snap`, `shot_1_2_80.snap`, `shot_2_0_80.snap`, `shot_2_4_80.snap`.
- Deleted under `crates/bingo-surface-tui/src/screens/snapshots/`, prefix `bingo_surface_tui__screens__opening__`: `opening_line_120x40.snap`, `opening_line_80x24.snap`, `opening_rest_120x40.snap`, `opening_rest_80x24.snap`, `opening_words_120x40.snap`, `opening_words_80x24.snap`, `opening_words_daylight.snap`.
- Shared files: `pointer.rs` removes retired Intro/crew targets; `transcript.rs` drops the opening adapter/height; `transcript/said.rs` uses `grid::fit`; `transcript/slots.rs` supplies optional meanings. `blocks.rs`/`worked.rs` only adjust tests. No M194 merge attempted.
- No dependency additions. Site/footer/activity clocks return `Drawing` deadlines; Layout collects them. Mode flash reuses the existing acknowledgement timestamp; end flash reads `last_turn`.
- Round 2 — typed mode notices, 10 s caret rest, delayed wakes, deadline fold, cleanup and four prototype fixes; contracts and ten regressions fail without fixes and pass with them. [Evidence and final gates](../verification/m193/review-round2.txt). Four needs-you snapshots updated; none deleted this round.
- `/plugins N from this repo waiting`: M199's welcome takes it.
- The 1.4 s model-change glow: the prototype uses a toast, arriving with M198's model sheet.
- `· this session` after the model and `context N%` below 70%: the prototype dropped them.
- The title's attention mark: the prototype says “waiting on you” once, with the edge (`p8-page.js`).
- `ctrl+c again to exit` as a right pair: it becomes a toast in M196.
- M198 handoff: with the stream bottom-anchored, ui.block/view-table, rewind, plugins and pager overlays cover its newest rows; M198 makes them dock sheets that push the stream up. Unchanged this round.
- Round 3 — send-scrollbar and zero-height footer/edge/ask regressions fail before and pass after; local notice lifetime/payload, shared submissions, all needs-you paths and review cleanups verified. [Evidence and gates](../verification/m193/review-round3.txt). No Ember capture or .snap changes; no wall-clock retries.
- Round 4 — welcome/inline-ask collection precedes clipping; Scroll owns flash expiry; four regressions fail without fixes and pass with them, including heights 1–60. Matching rows and all nits fixed; [evidence and gates](../verification/m193/review-round4.txt).
- Round 5 — clean rebase onto `94daa293`; four sweep mutations fail, six width regressions fail→pass; width/height wakes and all gates rechecked. [Evidence, audit and gates](../verification/m193/review-round5.txt). At merge, with the width fix squashed into one commit and the base still `94daa293`: all seven gates again, `6461 passed, 0 failed`; a hands-on tmux drive (dark and light, motion on) saw a running step's spinner advance in all 8 frames taken 250 ms apart.
