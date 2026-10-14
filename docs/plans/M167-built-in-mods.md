# M167 — Built-in mods

ADR-0086. Comes before M166, whose picker is the first built-in mod.

## Goal

A Rust plugin can own a feature whole the way an outside mod does:
its drawing sits innermost in the `ui.render` chain, its commands carry
default keys the person can rebind, and its open site receives the
keys. An outside mod wraps or replaces it; switching it off leaves the
surface's plain drawing.

## Bricks

1. **Placement.** `bingo-sdk`: `Placement::{Outer, Builtin}` on
   `Contribution::Interceptor` (serde default `Outer`, so existing
   plugins are unchanged). `chain` orders outside links, in-process
   `Outer`, `Builtin` in plugin list order, then the `Engine` tail.
   Pure ordering function, table test.
2. **Default keys.** `CommandSpec.keys: Vec<String>` (additive, empty by
   default; schema and TypeScript regenerated). Kernel settings key
   `keys` (`{"alt+m": "/model next"}`, merge `Replace`, `KERNEL_KEYS`).
   Pure `bindings(commands, settings, static_keys) -> Bindings` in
   `bingo-sdk`: settings first, then command defaults in plugin order;
   a static key is refused; a second claim on a key is dropped with a
   notice naming both. Table test.
3. **The surface runs bindings.** `bingo-surface-tui` reads
   `catalog(Commands)` and settings, builds `Bindings`, and on a bound
   key runs the command line as if typed. `keys.rs` keeps only the
   static keys. `TestBackend` test: a fixture command's `ctrl+x` runs
   it; a `keys` setting rebinds it.
4. **The open site owns its keys.** `View::Box` gains `onKey` (a minted
   handler, ADR-0079 §6) and validate allows one per tree, at the
   root. While a site whose tree carries `onKey` holds focus, the
   surface sends each non-static key as `{ key }` to it and treats
   `{ handled: false }` as unhandled. Pane controls keep working; a
   pane is a site with focus like any other. Contract test on the wire
   shape; `TestBackend` test with a fixture.
5. **Synchronous in-process answer.** A `Builtin` link answers
   `ui.render` and `onKey` on the calling thread with no channel hop;
   a test asserts the frame is drawn in the same pass the request was
   made.
6. **Fixture built-in.** `crates/bingo-mod-hello` (test-only, not in the
   release list): one command `/hello` with default key `ctrl+x`, one
   site drawing (`Toast`). Used by the tests below and as the template
   for `bingo-mod-<noun>` crates.
7. **Discipline.** `scripts/check_discipline.sh`: a `bingo-mod-*` crate
   is a plugin (no surface, no ratatui/crossterm, no other plugin);
   the TUI-names-no-plugin check covers it.
8. **Docs.** `docs/guide/` mods page: placement, `keys`, `onKey`, how to
   replace a built-in; ADR-0084's site list notes the owner.

## Files

- `crates/bingo-sdk/src/{plugin.rs,command.rs,view/mod.rs,
  view/validate.rs}`, new `keys.rs`; `schema/`, `sdk/typescript/`.
- `crates/bingo-core/src/{chain/mod.rs,host/registry.rs,settings.rs,
  settings/merge.rs}`.
- `crates/bingo-surface-tui/src/{keys.rs,input.rs,run.rs,pane/mod.rs}`.
- `crates/bingo-mods/src/` (`onKey` for JS trees).
- New `crates/bingo-mod-hello/`; `scripts/check_discipline.sh`;
  `docs/guide/`.

## Exit criteria

- [ ] Ordering table test; a JS mod in a temp plugin dir wraps the
      fixture's `Toast` with `next` and receives its tree; a JS mod that
      answers alone replaces it.
- [ ] `enabledPlugins: {"hello": false}`: `/hello` is gone, `ctrl+x`
      does nothing, the `Toast` is the surface's own drawing.
- [ ] Bindings table test; `TestBackend`: default key, rebind, refused
      static key with its notice.
- [ ] `onKey` round trip for a Rust fixture and a JS fixture; esc and
      ctrl+c never reach it.
- [ ] Gates: fmt, check, clippy -D warnings, test, check_discipline,
      budget, cargo deny, Windows check of sdk, core and the TUI;
      `scripts/tui-smoke.sh`.

## Non-goals

Moving an existing screen (each moves with its redesign milestone);
chord sequences (`ctrl+k ctrl+s`); per-mode keymaps; mouse bindings;
the desktop's key routing (it gets bindings from the same catalogue
later).

## Risks

- `onKey` round trips through a JS thread add latency to a focused
  outside-mod site; built-ins are synchronous, and a JS site that
  misses a frame draws on the next.
- A binding that shadows a terminal's own key (alt+arrow in some
  terminals): the static list covers the editing keys; document the
  rest.
- Two built-ins claiming one key is a build-time mistake; the notice
  names both so it is caught in the first drive.
