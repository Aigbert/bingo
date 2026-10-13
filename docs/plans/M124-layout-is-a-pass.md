# M124 — Layout is a pass

## Goal

Audit-2026-10 §4. The TUI lays a frame out as a side effect of drawing
it. `view::draw` fills `ui.painted: RefCell<Painted>` (`ui.rs`) from
behind a shared borrow through 10 mutable borrow sites, and keys, clicks and drags answer
against what the last draw left there (61 `painted.borrow` sites:
input 21, view 19, pointer 9, run 4, pager 2, `run/showing` 2,
`transcript/pictured` 2, ui 2). `demand()` measures the composer, strip
and activity bands by rendering them, and the draw renders them again.
`Run::animating` (`run.rs`) ORs a dozen conditions to decide whether to
draw again. Layout and row maths are the largest group of `fix(tui)`
commits.

After this milestone a frame is two steps: a pure `layout()` that cuts
the regions, renders each band once and records every target a key or
a pointer is answered against; then `paint()`, which writes that layout
to the buffer and decides nothing. The layout says when the screen next
changes, `next_frame: Option<Instant>`, and the loop sleeps until then.
No visible change, no new dependency, no sdk change.

## Bricks, in build order

1. **Contract tests first** (`layout.rs`, `#[cfg(test)]`), failing:
   - purity: the same tree, ui, size and clock give equal layouts;
   - one render per band: a frame renders each block and each band at
     most once, counted by a test-only counter;
   - hit-testing without a terminal: a rail card, a picture's cells and
     the foot are found in a layout no `TestBackend` drew;
   - `next_frame`: idle is `None`; a running turn is the next motion
     sample (33 ms breath, including each 150 ms glyph tick); a moving scroll or layer is its next sample; a notice is its
     expiry; an armed exit is its deadline; a placed `Status.until`
     (M121) is the next second its text changes.
2. **`Layout`** (`layout.rs`): the `Regions` from `frame::regions`, the
   visible lines composed into a screen-sized cell buffer, cursor, and targets `Painted` holds today:
   `blocks`, `height`, `top`, `card`, `list`, `rail`, `strip`, `cells`,
   `foot`. `cells` is still read out of the laid-out lines
   (`graphics::placed::cells`), so a picture cannot be targeted where
   the screen does not show it (M56).
3. **`layout(previous: Layout, tree, ui, size, now) -> Layout`.** The
   block memo moves from the previous layout into the next, so the
   function is pure over its arguments and a block whose revision has
   not moved is not rendered again. `demand()` takes the bands' lines
   from this pass instead of rendering them to count.
4. **`paint(&Layout, &mut Frame)`** replaces `view::draw`. It writes
   lines into rects and nothing else: every `painted.borrow_mut()` in
   `view.rs` goes, and so do `Painted::begin` and the `RefCell`.
5. **Readers move.** `Ui.shown: Layout` is the layout on the screen,
   set by the loop after each paint. Input, pointer, pager, selection
   and `run/showing` read it as a plain field. `Painted` is deleted;
   its methods (`picture_at`, `placed`, the blank rows above a short
   transcript) move to `Layout` with their tests.
6. **`next_frame`.** Each moving thing answers when it next changes:
   the activity row, scroll, layers, notices, crossfades, a send in
   flight, a block's own motion, an armed exit, a drag, placed
   countdowns. `Layout.next_frame` is the earliest. `run.rs` sleeps
   until it, the next event or the next key; an event held at the gate
   (`behind`) still asks for a frame at once. `Run::animating` goes.
7. **Records.** `docs/design/tui.md` §3 says a frame is laid out, then
   painted; §6 says a motion reports its next change. ADR-0002 gets a
   dated note that the surface's last layout is a memo of the screen,
   not state.

## Files

- `crates/bingo-surface-tui/src/layout.rs` (new), `frame.rs`
- `crates/bingo-surface-tui/src/{view,ui,run,input,pointer,pager,blocks,select}.rs`
- `crates/bingo-surface-tui/src/{run/showing,transcript/pictured,graphics/placed,graphics/decoded}.rs`
- `crates/bingo-surface-tui/src/{activity,rail,status,composer,foot,scroll}.rs`
  (each band's `next_change`)
- `crates/bingo-surface-tui/src/{test_support,screens,motion,painted,doubles,terminal,lib}.rs`
- `crates/bingo-surface-tui/src/{layers,search,clock,views}.rs`; new `layout/` helpers/tests
- `docs/design/tui.md`, `docs/adr/0002-*.md`

## Exit criteria

- [ ] `cargo test -p bingo-surface-tui layout` passes; the failing first
      run is pasted.
- [ ] `rg 'ui::Painted|RefCell<Painted>|painted\.borrow|fn animating' crates/bingo-surface-tui/src`
      finds nothing. (`painted.rs`'s test buffer is another type.)
- [ ] A pointer test answers a click on a rail card and on a picture
      from a `Layout` built without drawing.
- [ ] The counter test shows one render per band per frame, and none for
      an unchanged block.
- [ ] An idle TUI on the injected clock draws no frame over a simulated
      minute; a running turn draws on each glyph tick.
- [ ] `screens/` and `motion.rs` pass unchanged.
- [ ] `scripts/tui-smoke.sh` passes, and a tmux drive covers scroll,
      a rail card click, a picture click, a drag-select, a resize and
      an idle minute with no redraw.
- [ ] All gates of the upgrade protocol, plus the Windows cross-check
      for the TUI.

## Non-goals

- Focus and the order keys are tried in (M125); process-global theme
  and graphics detection (M125).
- Any change to what a region draws or to `frame::regions`' rules.
- The block revision keys, beyond what M121 and M123 already removed.

## Risks

- A layout holds the lines of what is visible, not the transcript. If
  it ever holds more, memory grows with the session; the counter test
  and a 10 000-item `screens` case watch that.
- `next_frame` too late leaves a cue stale; too early is today's cost.
  Each `next_change` is tested against `motion.rs`'s samples: drawn at
  any instant before it, the cue is the same; drawn at it, it differs.
- A selection drag reads the lines and cells of the frame on screen.
  `Ui.shown` is that frame, as `Painted` was; the drag tests pin it.
- Re-verified at `abe1755f` (M108–M123 landed): the `painted.borrow` count by file, `Painted`'s
  fields and `animating`'s conditions after M121, the `Revision` keys
  after M123, and each band's motion by symbol.

  `Painted` has the ten fields in brick 2 plus `regions`; `animating`
  has 13 conditions including `behind`, intro and placed countdowns.
  `Revision` has nine keys: status, size, agent, fold, linked, slots,
  run, last, look. M123 captions and `LEGACY_VOICES`/check 4f stay intact.
  `Screen::draw`, its recorder and test helpers must consume the layout;
  layer/search/selection helpers compose cells without a terminal `Frame`.
  The foot is patched into the reserved activity air after transcript
  measurement; this avoids measuring that band by rendering it twice.
  Rail geometry remains `frame::RAIL_AT` and `frame::RAIL_WIDTH` for M148.
  Motion includes breath samples as well as glyph ticks; M149 can add its
  caret and embers through the same `next_change` contract.
