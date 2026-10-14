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
   `Contribution::Interceptor` (`plugin::Placement` avoids the existing
   contributor `Placement`; its serde/default value is `Outer`; the
   contribution itself is in-process and has no serde encoding). The chain
   orders outside links, in-process
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
   key runs the command line as if typed. `keys.rs` keeps existing static
   keys until their screens move; bindings derive from the catalogue.
   `TestBackend` test: a fixture command's `ctrl+x` runs
   it; a `keys` setting rebinds it.
4. **The open site owns its keys.** `View::Box` gains `onKey` (a minted
   handler, ADR-0079 §6) and validate allows one per tree, at the
   root. While a site whose tree carries `onKey` holds focus, the
   surface sends each key except esc/ctrl+c as `{ key }` to it and treats
   `{ handled: false }` as unhandled. Pane controls keep working; a
   pane is a site with focus like any other. Contract test on the wire
   shape; `TestBackend` test with a fixture.
5. **Immediate in-process rendering.** Poll render futures on the calling
   thread and redraw ready answers before painting; pending futures continue
   asynchronously. `onKey` keeps the supervised `Input::Action` mailbox path:
   the existing host has no synchronous command door, and bypassing dispatch
   would violate ADR-0086. Add `CommandOutcome::Handled` because the old
   outcome could not convey `{handled:false}` through an acknowledgement;
   `ui.press` now returns null or the typed handled result.
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

- `crates/bingo-sdk/src/{plugin.rs,command.rs,render.rs,catalogue.rs,view/}`,
  new `keys.rs`; `schema/`, `sdk/typescript/`; empty keys in command literals.
- `crates/bingo-core/src/{chain/mod.rs,host/registry.rs,settings.rs,
  settings/merge.rs,turn/config.rs,session.rs,session/inputs.rs}`.
- `crates/bingo-surface-tui/src/{keys.rs,input.rs,run.rs,pane/mod.rs}`.
- `crates/bingo-mods/tests/`, `sdk/typescript/src/ui.ts` (JS key handlers),
  `crates/bingo-plugin-host/src/host.rs` (handled callback acknowledgements).
- New `crates/bingo-mod-hello/`; `scripts/check_discipline.sh`,
  `scripts/budget.toml`; `docs/guide/`; fixture edges and patch in `Cargo.lock`.

## Exit criteria

- [x] Ordering table test; a JS mod in a temp plugin dir wraps the
      fixture's `Toast` with `next` and receives its tree; a JS mod that
      answers alone replaces it.
- [x] `enabledPlugins: {"hello": false}`: `/hello` is gone, `ctrl+x`
      does nothing, the `Toast` is the surface's own drawing.
- [x] Bindings table test; `TestBackend`: default key, rebind, refused
      static key with its notice.
- [x] `onKey` round trip for a Rust fixture and a JS fixture; esc and
      ctrl+c never reach it.
- [x] Gates: fmt, check, clippy -D warnings, test, check_discipline,
      budget, cargo deny, `scripts/tui-smoke.sh`.
- [ ] Windows check of every touched crate (record C-toolchain deferrals).

## Decisions

- `BoxStyle.on_key` supplies the flattened `Box.onKey` wire prop, following
  the existing Button/Input prop pattern; validation permits it only at root.
- Focused sites receive editing/navigation keys as existing pane controls do;
  static bindings protect the composer, while esc/ctrl+c always stay outside.
- Accepted settings rebind an exact command line and suppress its defaults;
  invalid static settings do not remove working defaults. `keys` replaces
  objects explicitly because the existing merger normally merges object fields.
- No external dependencies. The fixture's sdk/serde_json/async-trait edges
  and mods/TUI dev-dependency on the fixture exercise the shared contract.
  The member counts in the normal-tree budget: 411 → 412 (ADR-0086).
- Keep the existing static screens and their keys until their redesigns;
  moving them now would contradict this milestone's non-goal and snapshots.

- The deny gate found RUSTSEC-2026-0346–0349 in existing rustls 0.23.45;
  update only that package to 0.23.46, with no new dependencies or ignores.

- Verification exposed a CLI helper that chose the first frame as the root;
  identify the person's print input so child wake timing cannot choose it.

## Non-goals

Moving an existing screen (each moves with its redesign milestone);
chord sequences (`ctrl+k ctrl+s`); per-mode keymaps; mouse bindings;
the desktop's key routing (it gets bindings from the same catalogue
later).

## Risks

- Handler acknowledgements retain actor supervision. JS callbacks add a
  runtime thread; ready Rust renders draw immediately, pending renders later.
- A binding that shadows a terminal's own key (alt+arrow in some
  terminals): the static list covers the editing keys; document the
  rest.
- Two built-ins claiming one key is a build-time mistake; the notice
  names both so it is caught in the first drive.

## Verified — 2026-10-15

Sequential gate output summaries (all native commands exited 0):
```text
cargo fmt --all -- --check: no output
cargo check --workspace --all-targets --locked: Finished dev profile
cargo clippy --workspace --all-targets --locked -- -D warnings: Finished dev profile
cargo test --workspace --locked: 5508 passed; 0 failed; 5 ignored (aggregate)
bash scripts/check_discipline.sh: discipline ok (existing warnings)
bash scripts/budget.sh: 412/412 dependencies; core 0s; isolation 0; budget ok
cargo deny check: advisories ok, bans ok, licenses ok, sources ok
bash scripts/tui-smoke.sh: tui-smoke ok (isolated socket and temporary HOME)
cd sdk/typescript && npm test: tests 31; pass 31; fail 0
```
Generators passed with `BINGO_UPDATE_SCHEMA=1 cargo test --locked`: sdk;
plugin-rpc `--lib`; surface-rpc `--lib schema`; store-jsonl `--lib journal`;
mods `--lib schema_and_types_are_generated_from_the_same_rows`. No snapshot changes.
Windows: ran `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc --locked`
for all 20 touched crates. Exit 0: bingo-{sdk,core,mod-hello,agents,checkpoints,
context,demo-ui,experience,plugin-host,rooms,schedule,skills,tasks,tool-bash}.
Exit 101: bingo, bingo-{mcp,mods,plugin-rpc,surface-tui}: aws-lc-sys missing
Windows C headers; bingo/mods also rquickjs-sys MSVC flags rejected by clang.
Permissions also stops in tree-sitter/tree-sitter-bash: stdio.h/stdlib.h absent.
Windows criterion remains unchecked: these C prerequisites block Rust checking.
One gateway lock test passed its isolated retry; final full suite passed normally.
First smoke lost the last character of Ada; isolated counter and full retry passed.
Budget warned at 11 GB of debug output; final cleanup is `cargo clean`.
