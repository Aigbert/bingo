# M160 — The sealed sites

ADR-0082 §6.

## Goal

The permission prompt and the sign-in card become sites a mod may draw around,
never draw: the host's own drawing of who asks, which tool, the full input and
the choices is always on screen whole, or the mod's answer is not used.

## Bricks

1. **Fact props.** `Permission` from `InteractionKind` permission
   interactions: tool, input, the choices (`AnswerSpec`s) and the
   interaction id; `SignIn` from `InteractionKind::Login`: provider and the
   `LoginFlow` facts (browser url, device url and code, paste). No key names,
   no preformatted text. JSON fixture and round-trip test first.
2. **Sealed check, host side.** `Component::sealed()` is true for both. In
   `view::answer_ok` (core `chain/render.rs` path), a sealed component's
   answer must contain exactly one `Engine` node for the asked component, and
   its props must equal the request's props. Anything else is answered
   `Engine` alone, with the ADR-0079 §5 notice line naming the plugin. Pure
   function with table tests: zero nodes, two nodes, changed props, a
   different component's `Engine`, a wrapping `Box` that passes.
3. **Sealed check, surface side.** The TUI draws a sealed answer only when the
   `Engine` node's lines land inside the card whole: not clipped by the card's
   height, not cut by width truncation, not hidden by `display: none` or a
   zero-size box. Otherwise it draws its own card. Pure function on the laid
   out lines plus a marker for the engine node's span (`views::Marks` exists).
4. **Keys are the host's.** A sealed card's keys (y/n/a, enter, esc, paste)
   are dispatched by the TUI exactly as today, whatever the mod drew. Buttons
   a mod adds around the card fire their own actions and cannot answer the
   interaction.
5. **Text and desktop.** `surfaces()` lists `terminal`, `desktop`, `text`.
   The `text` surface (print, channels) keeps drawing folds; nothing changes
   there in this plan.

## Files

- `crates/bingo-sdk/src/{render.rs,component/props.rs,view/*}` (answer check).
- `crates/bingo-core/src/chain/render.rs`.
- `crates/bingo-surface-tui/src/{dialog.rs,view.rs,component.rs}` and a new
  small module for the whole-drawing check.
- `crates/bingo-mods/src/{guide.md,validate*}`; regenerated `schema/`,
  `sdk/typescript/`.
- Fixture mods: one that frames the permission card (accepted), one that
  rewrites its props, one that omits the engine node, one that clips it.

## Exit criteria

- [ ] Fixture tests for `Permission` and `SignIn` props.
- [ ] Table tests for the host check and the surface whole-drawing check.
- [ ] Black-box: framing mod shows its frame and the host card; each hostile
      fixture yields the built-in card and one notice line; `y` still answers
      the interaction in every case.
- [ ] No snapshot changes when no mod matches.
- [ ] tmux drive in a harness-owned session: permission card with the framing
      mod at 80x24 and 200x50, answered with `y` and with `esc`.
- [ ] Gates: fmt, check, clippy -D warnings, test, check_discipline, budget,
      cargo deny, `scripts/tui-smoke.sh`, Windows check (or recorded deferral).

## Non-goals

Mods answering interactions; changing permission policy or the order of
choices; channel (Feishu) permission cards; desktop drawing; new
dependencies.

## Risks

- A mod could imitate a permission card elsewhere (a pane drawn to look like
  one). Out of scope; recorded for a later look at pane chrome.
- Clipping detection must use the laid-out result, not the requested size, or
  a short terminal lets a clipped card through.
- Equality of props must be structural JSON equality after normalising both
  sides, so key order never fails an honest answer.
