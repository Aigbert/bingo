# M184 — The KeyBar

ADR-0095 §1, §2, §8; ADR-0084 §1, §4. One right-aligned row that says what the
keys do now, and the place toasts appear.

## Goal

No pane draws key hints in its body; every state's pairs appear in one row,
`key` in bold glow, the action dim, ` · ` in ash, matching the prototype's
`_keys.txt` per state. Toasts sit on its left for 2.8 s and nothing moves.

## Bricks

1. **sdk props**: `KeyBar { pairs, toast }`; a pane's key pairs as props on its
   seat (`Slot::Pane` hints). Fixture tests first.
2. **Built-in owner** `bingo-mod-keybar` draws the row from the focused site's
   pairs and the keymap; sealed while an ask is open (ADR-0084 §4): the host
   draws the ask's pairs.
3. **Mods declare pairs** instead of drawing them: models, palette, status,
   plugins, sessions, browse, crew, memory, schedules, ask.
4. **Composer tag** (`? `, `⟲ `, `/model › sign in`, …) and placeholder as
   `Composer` props; the stray `/model` echo under a pane goes.
5. **Toasts** move into the bar (one at a time, wall clock, dropped on expiry);
   pending other sessions and repo waiting become pairs (`ctrl+g 2 need you`).
6. **Narrow widths** drop pairs from the left; the toast wins over pairs.

## Files

- `crates/bingo-sdk` (component props), new `crates/bingo-mod-keybar`.
- `crates/bingo-surface-tui/src/{frame.rs,view.rs,toast.rs,keys.rs,sealed.rs}`.
- Each mod's `draw.rs` that drew hints.

## Exit criteria

- [ ] Snapshot per state from the prototype's `_keys.txt` list.
- [ ] A test per mod: no hint text inside its body.
- [ ] tmux drive of five states with captures in Verified.
- [ ] Gates as in M182.

## Non-goals

Changing what any key does. Keymap remapping UI.

## Risks

- Sealed ask pairs must stay the host's: a mod's pairs never draw over an open ask.
- Width churn: the bar's pairs change with focus; the row height never changes.
