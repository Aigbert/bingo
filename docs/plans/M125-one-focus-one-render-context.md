# M125 — One focus, one render context

## Goal

Audit-2026-10 §4, its second half. Two things in the TUI are answered
by order and by process globals instead of by a value:

- **Who owns a key.** `input::on_key` tries about ten pieces of state in
  a fixed order: `intro`, `block`, `esc_armed`, the leaving gesture,
  `select.run`, `search`, the captured layer (`Open::Picker`, `Pager`,
  `Rewind`), the chords, the panel and switcher, `esc`, the session's
  open interaction, `Tab`, the menu, then editing. `pointer::on_mouse`
  tries a different order (card option, listed row, rail card, foot,
  picture, block). Nothing stops two of them being set at once, and a
  click and a key can disagree about who is in front.
- **What a frame is drawn in.** The palette (`theme.rs`: `ASK`
  `OnceLock`, `GROUND` `AtomicU8`) and the graphics answer
  (`graphics/mod.rs`: `HEARD` `RwLock`, `notice`, `late`) are process
  globals with 19 `cfg(not(test))` twins (theme 10, graphics 8, lib 1).
  The test build swaps in a `thread_local` `OVERRIDE` and `saying`, so
  production detection never runs under test.

After this milestone `Ui.focus: Focus` is one enum, and both `on_key`
and `on_mouse` ask one function who owns the event. A `Render` value
holds the look, the graphics answer and the motion switch; `run.rs`
builds it from pure detection and hands it to M124's `layout()`. No
visible change, no new dependency, no sdk change.

## Bricks, in build order

1. **Contract tests first**, failing:
   - `focus.rs`: a table of every `Focus` variant against each key class
     (printable, `esc`, `Tab`, chord, arrow, `Enter`) and a left press,
     naming the owner; the open interaction outranks the composer and
     nothing else;
   - `render.rs`: detection as a table, environment plus probe answers
     (`OSC 11`, `XTVERSION`, the graphics query, a late reply) to a
     `Render`, through the code production runs;
   - two tests drawing in two looks on parallel threads do not see each
     other's.
2. **`Focus`**: `Composer`, `Intro(Playing)`, `Select(Run)`,
   `Search(Search)`, `Layer(Layer)`, `Card(CardId)`. It replaces
   `intro`, `select.run`, `search`, `layer` and the card `focus`,
   so two owners cannot be set at once. `Layer` retains its reveal timing;
   a closing layer is visual residue and derives the composer owner. Two owners stay derived, never
   stored: the session's open interaction (read from the tree) and the
   menu (read from the composer's line and `Menu.dismissed`), each the
   owner only while `Focus` is `Composer`. `esc_armed`, `block` and the
   armed exit are gestures, not owners, and stay where they are.
3. **`focus::owner(&Ui, &Tree) -> Owner`** and one dispatch. `on_key`
   and `on_mouse` each match on the owner, then on the event. Each
   owner's handler is the function it is today. Where the mouse order
   differed from the key order, brick 1's table chooses one, and the
   commit body lists each case that changed.
4. **`Render { look: Theme, graphics: Graphics, motion: bool, ascii: bool }`.**
   `theme::detect` and `graphics::detect` become pure functions of what
   the environment and the probe said. The probe's I/O stays in
   `terminal.rs`, before the terminal is taken. `run/look.rs` (M71)
   and a late graphics reply produce a new `Render` instead of writing
   a slot.
5. **One door into the pass.** `layout()` and `paint()` take `&Render`.
   Inside them, style helpers read it through `render::current()`, a
   scoped thread-local that `Render::scope` sets for the pass and that
   falls back to the resting look outside one. That keeps the ~490
   `theme::` call sites as they are. The block memo's look key and the
   highlighter's memo read the `Render` they were drawn in.
6. **Twins go.** `ASK`, `GROUND`, `HEARD`, `OVERRIDE`, `saying` and the
   19 `cfg(not(test))` items are deleted. Tests build a `Render` as
   production does. `PROBE_THROUGH` in `lib.rs` is exported in both
   builds.
7. **Records.** `docs/design/tui.md` §7 says one owner at a time and
   how it is chosen; §4 says the look is a value of the run.
   ADR-0002 gets a dated note beside M124's.

## Files

- `crates/bingo-surface-tui/src/{focus.rs,render.rs}` (new)
- `crates/bingo-surface-tui/src/{ui,input,pointer,layers,composer,select,view,pager,clock}.rs`, `input/switcher.rs`
- `crates/bingo-surface-tui/src/{theme,lib,terminal,run,blocks,highlight,layout}.rs`
- `crates/bingo-surface-tui/src/graphics/mod.rs`, `run/{look,notices,opening}.rs`, `layout/{deadline,tests}.rs`
- `crates/bingo-surface-tui/src/{test_support,screens,motion}.rs`, `screens/{colours,opening,windows}.rs`; mechanical fixture callers of changed types
- `docs/design/tui.md`, `docs/adr/0002-*.md`

## Exit criteria

- [ ] `cargo test -p bingo-surface-tui focus` and `cargo test -p bingo-surface-tui render` passes; the failing
      first run is pasted.
- [ ] Theme/graphics detection has no `cfg(not(test))` twins or
      `ASK`, `GROUND`, `HEARD`, `OVERRIDE`, `graphics::saying` globals.
      M124's `layout/count.rs` test instrumentation is unrelated and stays.
- [ ] `rg 'search\.is_some\(\)|select\.run\.is_some\(\)|layer\.captures\(\)' crates/bingo-surface-tui/src/{input,pointer}.rs`
      finds nothing; both dispatch on `focus::owner`.
- [ ] The detection table runs the production path under `cargo test`,
      including a late graphics reply and a background change.
- [ ] `screens/` and `motion.rs` pass unchanged, apart from the
      mouse-order cases the commit body names.
- [ ] `scripts/tui-smoke.sh` passes, and a tmux drive opens and closes
      each layer by key and by click, and starts with `BINGO_THEME`
      light and dark.
- [ ] All gates of the upgrade protocol, plus the Windows cross-check
      for the TUI.

## Non-goals

- Changing a binding, a chord or a layer; configurable keys.
- Passing `&Render` to every style helper by argument. The scope is
  the one door; threading it through ~490 sites is not worth the diff.
- Any change to what the probe asks or when (`run/look.rs`'s clock).

## Risks

- Some flags may be set together today on purpose (a search while the
  pager is open). Brick 1's table finds them. If two must coexist, the
  inner one nests in its owner's variant rather than becoming a flag.
- A click that went to the transcript while a card was open may now go
  to the card, or the reverse. Each change is named and tested.
- A helper outside a scope uses the resting look in both builds; scope
  restoration, nesting and parallel isolation must be tested.
- Re-verify at start: `on_key`'s and `on_mouse`'s orders, `Ui`'s
  fields, the twin count by file, and M124's `layout()` signature.

## Re-verified — 2026-10-13 at f3ed55b9

- Clean `m125-one-focus`; M124 `layout(previous, tree, ui, size, now)`
  returns `Layout`; `paint(layout, frame)` copies its canvas.
- Keyboard precedence and mouse ordering still match the Goal. `Ui` still
  owns all five independent owners; `Layer` also owns reveal/closing state.
- Detection twins remain theme 10, graphics 8, lib 1. M124 adds three
  unrelated instrumentation twins; `transcript::saying` is also unrelated.
- Added actual consumers (selection, view, pager, opening, clock and layout
  helpers) to Files. Shared M134 files receive only necessary local edits.
- `Render` derives ASCII from its theme glyphs instead of duplicating it;
  environment/probe inputs remain run-owned for late replies. No dependency.
- Preserve the rail and panel until M148 (ADR-0081); no redesign here.
