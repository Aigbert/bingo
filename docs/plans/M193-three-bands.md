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

`crates/bingo-surface-tui/src/{frame.rs,view.rs,view/,status.rs,keybar.rs,keybar/,toast.rs,
foot.rs,composer.rs,crew.rs,welcome.rs,opening.rs,run/look.rs,terminal.rs}`; new
`src/dock.rs` (regions, growth, edge) and `src/footer.rs`, one noun each; `bingo-mod-keybar`
(it draws pairs into the footer row).

## Exit criteria

- [ ] The dock and footer bands of `live`, `done`, `mode`, `mode-later`, `reduced-mode`,
      `reduced-live`, `ascii-live`, `ascii-done`, `narrow`, `small-done` and `welcome` pass the
      harness.
- [ ] Edge line states at named instants: the comet at three moments, the 1.1 s end flash at
      0/550/1100 ms, the mode flash, reduced. Each is a band test or a `moved` assertion.
- [ ] Growth: a dock going from 3 to 14 rows shows the intermediate heights at 14 ms steps;
      shrinking is immediate.
- [ ] Whichever of M193 and M194 lands second also passes the full frame of `live`, `done`,
      `ascii-live`, `ascii-done`, `reduced-live`, `small-done`, `mode` and `mode-later`.
- [ ] `scripts/tui-smoke.sh` waits for a footer string, not `/ commands` in the old `KeyBar`
      row, and passes.
- [ ] Gates and checks as in M192. tmux drive: idle, live, mode cycle, 80×24, a toast.

## Non-goals

The stream's rows (M194); the palette, mentions and queued rows in the dock, and the key
behaviour (M196); asks (M197); sheets (M198); the needs-you lead, view segments and ctrl+g
(M200); the welcome wordmark and the exit receipt (M199).

## Risks

- **Many snapshots pin the header, `KeyBar` and status rows.** Re-pin them in bulk only after
  reading a sample, and list in Verified what changed and why.
- **The OSC 2 title can leak into a pty test's stdout.** Write it only when stdout is a
  terminal and the TUI owns it.
- **Growth animation.** Keep the deadline scheduler (`layout/deadline.rs`) as the only source of
  wakes, so an idle frame costs nothing.
