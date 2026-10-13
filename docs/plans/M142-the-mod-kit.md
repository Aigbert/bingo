# M142 — The mod kit

## Goal

ADR-0078 §8 for a mod. `bingo plugins validate` reads the module the way
the runtime will (oxc), lists what it hooks and calls, and refuses what
cannot be checked statically. `bingo plugins test` runs `*.test.ts` in
process against the real host and the fake provider; `bingo/testing`
supplies `test`, `expect`, `mock.*` and `$.ui.mount`. Types land in
`.bingo/types/` on load: `'bingo'` exports `Register`, each event's types
under Claude Code's names, and the helpers M141 shipped. M133's
scenarios give way to tests. After M137 and M140; proven on M141's
examples, which gain their tests here.

## Bricks, in build order

1. **Contract: findings and reports, tests first.** `bingo-mods`
   `validate/` answers `Finding { file, line, column, rule, text }`, rule
   one of `on-literal`, `dollar-static`, `state-key`, `env-name`, `uses`,
   `state-contract`, `prop-subset`, `import`. One fixture mod per rule in
   `crates/bingo-mods/tests/fixtures/validate/<rule>/` with its
   `expected.json`; M141's examples and M140's `counter` come out clean.
   A test run answers `Report { file, test, outcome: Pass | Fail {
   message, at } | Skip }`, one line each or NDJSON (`--json`). All fail
   first.
2. **The reader, pure over the AST.** One walker per rule, over oxc's AST
   at M137's pinned version, reading every file of the module graph:
   `hooks.rs` (each `on(…)` in `register`: a literal pattern or a `const`
   bound to one, a literal matcher), `dollar.rs` (every `$` chain static;
   no `$[x]`; `$` never aliased to a variable; passing `$` to a function
   is allowed, since that function's file is read too), `keys.rs` (the
   first argument of `$.state.*`, `$.store.*`, `$.env.*` and `atom(…)` a
   literal), `uses.rs` (an unsafe noun of ADR-0078 §5 called but not in
   the manifest's `uses` is an error; a `uses` never called, a warning),
   `contract.rs` (`types/index.d.ts`'s `interface PluginState { '<plugin>':
   {…} }` holds every literal state key; no file, a warning), `props.rs`
   (a JSX prop outside M140's drawn subset, read from `bingo_sdk`'s table,
   is named as a warning: the surface ignores it). Alongside the findings
   it lists hooked events, `$` calls, keys and `uses`.
3. **`bingo plugins validate <dir>`** dispatches on the manifest:
   `command` keeps M133's spawn-and-handshake, `module` runs brick 2 and
   spawns nothing. One `file:line:col rule text` line per finding, then
   the listing; exit 0 or 1.
4. **`bingo/testing`.** A built-in module of the runtime (beside M137's
   `bingo`), `crates/bingo-mods/js/testing.ts` embedded with
   `include_str!`, its host half in `testing.rs`. `test`, `describe`;
   `expect(v)` with `toBe`, `toEqual`, `toMatchObject`, `toContain`,
   `toThrow`, `rejects`, `not`. `mock.clock` (`set`, `advance`: tokio's
   paused time, which M138's `$.clock` runs on), `mock.store` (seed, read back), `mock.env` (the map
   `$.env` reads under test), `mock.session({ model, cwd })` (the
   `bingo-provider-fake` script and the session's facts), and `reload()`
   (M137's reload path, now, so no test waits on a poll). The module
   exists only under `plugins test`; a session never sees it.
5. **`$.ui.mount`**, in `mount.rs`: `mount(component | { pane: id }, props?,
   { columns = 80, rows = 24 })` asks `HostApi::render` as the `terminal`
   surface and answers a handle: `tree()`, `drawn()`, `find(text | key)`,
   `press(key)`, `input(key, text)`, `select(key, value)`, `advance(ms)`,
   `resize(columns, rows)`. A press fires the minted `Action` through
   `Input::Action`, as the TUI does, then renders again on
   `Invalidated`. `drawn()` is the TUI's own drawing as plain text: the
   TUI crate exports `pub fn drawn(&View, width) -> Vec<String>` (no
   ratatui type crosses), and the bin hands it to `bingo-mods` as a
   function, so the plugin imports no surface. An `Engine` node draws as
   `⟨Component⟩`, so a test can see what a hook wrapped.
6. **`bingo plugins test <dir> [--json]`.** The bin's `plugins/test.rs`
   builds the real host with `FakeProvider` as M133's scenario runner
   did, finds `**/*.test.{ts,tsx}` outside `.bingo/`, and runs each file
   in a fresh `Runtime` and session. Budgets hold as in a session: a hung
   test dies at the hook budget, named. Exit 0, or 1 naming each failing
   test with `file:line`.
7. **Scenarios go.** `ts-status`'s `scenarios/acceptance.json` becomes
   `hooks/register.test.ts`, run in process though the example still
   ships its node entry (one module, two transports). Deleted:
   `crates/bingo/src/plugins/scenario.rs`, `schema/plugin-scenario.json`
   and their cases in `tests/plugin_kit.rs`, which now runs `plugins
   test`.
8. **Types on load.** `types.rs` renders through `bingo-schema-ts`
   (M133), writing only when the bytes differ:
   `.bingo/types/bingo/index.d.ts` — `'bingo'` exports `Register`, each
   event's types under Claude Code's names (the live catalogue's kernel,
   op and noun events), `$`, the elements with M140's drawn subset, and
   `atom`/`derive`/`read`/`update`/`memberOf`, as M137 brick 6 says;
   `bingo/testing.d.ts`; `bingo-tools/index.d.ts` (this host's tool
   inputs, from their schemas); `.bingo/tsconfig.json` (`jsxFactory: h`,
   `jsxFragmentFactory: Fragment`, `paths` for `bingo`, `bingo-tools`);
   `.bingo/.gitignore` of `*`. A root `tsconfig.json` extending it only
   when the mod has none. A failed write is one notice, never a failed
   load.
9. **Tests for what exists.** `sdk/typescript/src/state.test.ts` tests
   M141's helpers under `bingo plugins test sdk/typescript` (the
   `ifVersion` retry, `derive`'s memo, `memberOf`); `tool-call`, `band`
   and `pane` each gain a `*.test.ts` that M146 runs again.

## Files

- `crates/bingo-mods/src/validate/{mod,hooks,dollar,keys,uses,contract,props}.rs`,
  `{testing,mount,types}.rs`, `js/testing.ts` (new); `tests/fixtures/validate/`
- `crates/bingo/src/plugins/{validate.rs,test.rs (new)}`, `scenario.rs` (deleted), `main.rs`
- `crates/bingo-surface-tui/src/lib.rs` (`pub fn drawn`)
- `schema/plugin-scenario.json` (deleted)
- `sdk/typescript/src/state.test.ts` (new)
- `examples/plugins/ts-status/` (`hooks/register.test.ts` new, `scenarios/` deleted);
  `examples/plugins/{tool-call,band,pane}/` (one `*.test.ts` each)
- `crates/bingo/tests/plugin_kit.rs`

## Exit criteria

- [ ] `cargo test -p bingo-mods validate::`: each rule's fixture yields
      its `expected.json`; the examples and `counter` yield none.
- [ ] `bingo plugins validate examples/plugins/pane` exits 0 with its
      listing; the `on-literal` fixture exits 1 at `file:line:col`. Pasted.
- [ ] `bingo plugins test` exits 0 on `band`, `pane`, `tool-call`,
      `ts-status` and `sdk/typescript`; a wrong `expect` exits 1 naming
      the test and `file:line`; `--json` output parses line by line.
      Pasted.
- [ ] `pane.test.ts` mounts the pane, runs two tool calls through the
      fake provider, presses, and asserts `drawn()`. Pasted.
- [ ] A second load of a mod leaves `.bingo/types` untouched (mtimes
      pasted); `tsc --noEmit -p examples/plugins/pane` passes on a local
      run, pasted once.
- [ ] `rg -n 'plugin-scenario|scenario::' crates schema` prints nothing.
- [ ] Every gate; `scripts/budget.sh` unchanged in crates (oxc came with
      M137); the Windows check of `bingo-mods` and `bingo`.

## Non-goals

- Hot reload itself (ADR-0078 §7, M137); `bingo plugins new`; publishing.
- Snapshot files (`toMatchSnapshot`), coverage, watch mode, npm imports.
- `mount` drawing the desktop table; `tsc` in CI (it is a dependency).
- Tests for a process plugin in another language: it tests itself with
  its own tools, and the scenarios are not kept for it.

## Risks

- **validate is a lint, not a sandbox.** `$` passed into a function and
  used dynamically there is caught only because that file is read too;
  an escape through data (`$` stored in an object) is refused as an
  alias. The grants (ADR-0078 §5) still enforce at run time.
- **`drawn()` drops style.** A test about colour asserts `tree()`.
- **`mock.clock` and budgets.** Budgets count a hook's own time through
  the interrupt handler, not tokio time, so `advance` cannot trip
  one; a test pins that.
- **Writing into a project.** A project mod gains `.bingo/` on load; its
  `.gitignore` keeps it out of commits.
- Re-verify at start: M137's built-in module mechanism, pinned oxc and
  `bingo-plugin-host`; M133's `validate`, scenario runner and
  `plugin_kit.rs` as landed; `bingo-schema-ts` and M136's `events.ts`; M140's
  handlers, `HostApi::render` and `Invalidated`; M141's example paths;
  M138's clock; whether the TUI can export `drawn` and keep `views`
  private.
