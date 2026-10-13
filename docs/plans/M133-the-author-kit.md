# M133 — The author kit

## Goal

ADR-0075's protocol 2 gets its first outside speaker. A TypeScript SDK
whose wire types are generated from `schema/plugin.json`, two commands an
author runs — `bingo plugins validate <dir>` and `bingo plugins test
<dir>` — and an example, `examples/plugins/ts-status`, that does Phase 5's
acceptance test from a directory outside the workspace: it places a status
entry, intercepts a tool call, and keeps its state across a hot reload.
The module is ADR-0078 §2's `register(on)`: the SDK runs it under node,
deno or bun over protocol 2, and M137's in-process runtime runs the same
file unchanged.

## Bricks, in build order

1. **Contract: the scenario file and the generated wire, tests first.**
   `bingo plugins test` reads `<dir>/scenarios/*.json`: `{model, steps}`,
   where `model` is a `bingo-provider-fake` script and each step is one of
   `submit {text}`, `reload {}`, or `expect` over what the session showed
   — `slot {slot, contains}`, `tool {name, contains}`, `denied {name}`,
   `notice {contains}`. A Rust type with its schema committed as
   `schema/plugin-scenario.json` (the `BINGO_UPDATE_SCHEMA` test of
   `the bin’s `plugins/schema.rs`). A new library crate, `bingo-schema-ts`, renders
   `sdk/typescript/src/wire.ts` from the plugin schema the same way, and a
   test fails when the committed file differs. Both fail first.
2. **The TS renderer as a pure brick.** `render(schema: &Value) -> String`
   emits erasable TypeScript only — `type` aliases and string-literal
   unions, no `enum`, no `namespace`, no parameter properties — so a
   type-stripping runtime runs it unchanged. Table tests over small
   schemas (object, optional field, `oneOf`, string enum).
3. **The SDK runtime**, hand-written, no dependencies, `node:` modules
   only (`readline`, `process`):
   - `rpc.ts` — one JSON-RPC message per line, ids, the pending table,
     `METHOD_NOT_FOUND` for a method it has no handler for.
   - `plugin.ts` — the entry a manifest's `command` runs: imports the
     author's module, calls `register(on, options)`, declares each hooked
     event at the handshake (protocol 2 capabilities); `tools/changed`.
   - `on.ts` — `on(pattern, matcher?, hook)` with ADR-0078 §2's patterns;
     a pure `matches(matcher, e)` (scalar, RegExp, array, nested) with
     table tests. A hook `($, e, next)` crosses as `intercept/<event>`
     with `after` declared: returning without `next` answers in one
     phase; `next(e2)` answers `{next: e2}` and resolves with the
     `/after` result; `intercept/cancel` rejects the pending `next`.
   - `dollar.ts` — `$` over the protocol-2 verbs, named as ADR-0078's
     `$` table names them: `$.ui.status` (`host/place`), `$.state` and
     `$.store` (`host/state` session and global), `$.model.complete`,
     `$.session.deliver`/`extend`, `$.<noun>.<method>` (`service/call`).
     A method this transport lacks is absent from its type.
   - `index.ts` exports; imports use `import type` and `.ts` extensions.
4. **`bingo plugins validate <dir>`.** In `bingo-plugin-rpc`,
   `validate.rs`: parse the manifest (name rules, entry, `config` is a
   JSON Schema), spawn the entry through the host's own spawner, handshake,
   list what it contributes (tools, commands, links and their points),
   stop it. The bin prints one line per finding; exit 0 or 1.
5. **`bingo plugins test <dir>`.** In the bin, `scenario.rs`: build the
   real host with `FakeProvider` and the scenario's script, the plugin
   added as a `--plugin-dir` would add it, and fold the stream with
   `SessionState::apply` (ADR-0002) to check each `expect`. A `reload`
   step asks the plugin host to reload now — the path the poll takes — so
   no step waits on a clock. Exit 0, or 1 with the failing step and what
   the session showed instead.
6. **`examples/plugins/ts-status`.** `hooks/register.ts` counts the tool
   calls it has seen in `$.store`, shows the count with `$.ui.status`, and
   at `tool.check` (`{ tool: 'Bash' }`) denies a command starting with
   `rm -rf` — the `tighten` tier's own right, no grant needed. `main.ts`
   is the two-line node entry; M137 loads `register.ts` itself.
   `scenarios/acceptance.json` reads a file twice, reloads, reads again,
   expects `3` in the slot, then sees `rm -rf` denied.
7. **The acceptance run.** A black-box test copies `ts-status` and
   `sdk/typescript/src` into a temporary directory, keeping their relative
   paths, and runs `bingo plugins validate` and `bingo plugins test` there:
   nothing it runs is a workspace file. It is skipped with a printed reason
   when `node --version` is absent or older than type stripping by default.

## Files

- `sdk/typescript/{package.json (private, no dependencies),README.md}`
- `sdk/typescript/src/{wire.ts (generated),rpc.ts,plugin.ts,on.ts,dollar.ts,index.ts}`
- `crates/bingo-schema-ts/src/lib.rs` (new, `tier = "library"`, serde_json
  only; the catalogue (M136) renders with it too)
- `crates/bingo-plugin-rpc/src/{wire_ts.rs,validate.rs}` (new; `wire_ts`
  owns drift tests), `lib.rs`, `manager.rs` (explicit reload wrapper)
- `crates/bingo/src/plugins.rs` → `plugins/{mod.rs,switch.rs,validate.rs,scenario.rs,schema.rs,expect.rs}`
- `crates/bingo/src/main.rs` (`plugins::Action::{Validate,Test}`)
- `schema/plugin-scenario.json` (new)
- Workspace and affected Cargo manifests/lock: the renderer uses existing
  `serde_json`; bin adds existing `serde`, `schemars`, `futures`, `tempfile`
  for scenarios and isolated homes. `tempfile` moves from dev to normal;
  `scripts/budget.toml` permits two normal-tree crates: that existing crate
  and the new renderer (353 → 355). Fake script types derive `JsonSchema`
  (existing `schemars` edge) so the scenario contract never copies them.
- `crates/bingo-provider-fake/{Cargo.toml,src/lib.rs}` (schema derives only)
- `examples/plugins/ts-status/{plugin.json,hooks/register.ts,main.ts,scenarios/acceptance.json,README.md}`
- `crates/bingo/tests/plugin_kit.rs` (new)
- `.github/workflows/ci.yml` — `actions/setup-node`, pinned, Linux job
  only, so the acceptance run is not skipped there (this line justifies it)

## Exit criteria

- [ ] `cargo test -p bingo-schema-ts` and `-p bingo-plugin-rpc wire_ts::` pass; editing
      `wire.ts` by hand makes it fail (shown once, pasted).
- [ ] `cargo test -p bingo-plugin-rpc validate::`: a missing entry, a bad
      name and a protocol-1 plugin each fail with the field named.
- [ ] `cargo test -p bingo --test plugin_kit`: `validate` exits 0 on
      `wordcount` and `ts-status`; `test` exits 0 on `ts-status`'s
      acceptance scenario from a temporary directory; a scenario with a
      wrong `expect` exits 1 naming the step.
- [ ] CI's Linux job log shows acceptance ran (pending CI; no push authorized).
- [ ] `node sdk/typescript/src/index.ts` exits 0 with no output on the
      pinned node (the SDK parses under type stripping), pasted.
- [ ] All gates of the protocol; normal-tree budget 355 (+renderer, +existing tempfile).

## Non-goals

- Publishing the SDK to a registry; `bingo plugins new`; a template.
- Type-checking the SDK in CI (type stripping checks nothing; `tsc` is a
  dependency); the example running is the test.
- An SDK in another language; the in-process runtime (M137, whose first
  module is this `register.ts`); `*.test.ts` (M142 replaces scenarios).
- Expectations read displayed slots/transcript, never private plugin state.

## Risks

- Node 22.18 / 23.6 enable default stripping; CI pins 24.17.0.
  Deno/Bun runtime guidance cites their docs; Node runs acceptance.
- A hook holds a pending `next` across two requests; a plugin that
  never calls `next` and never returns holds a chain until the `HOOK`
  deadline. The deadline is the host's (ADR-0074 §4), not the SDK's.
- Scenarios isolate both home and project discovery and use only FakeProvider.
  A missing state key is null; reload notices are transcript items.
  `denied` reads error text containing “denied”; the stream carries no verdict.
- Re-verify at start: M132's verbs and capability names as landed (the
  generator reads them, it does not restate them); ADR-0070 slots as
  M121/M123 built them (what `expect slot` reads off `SessionState`);
  M128's `bingo-process` for `validate`'s spawn; ADR-0068/M116 for the
  esc test of a pending `next`; M111's test host for the bin's unit tests.

## Re-verified (2026-10-13, e9e45bb9)

- M108–M123 and M127–M132 are present; protocol 2 is frozen in
  `schema/plugin-2.json`. No wire shape changes are needed.
- `Bridge::reload(Manifest)` is the poll's path; expose it through
  `Manager` and `PluginRpcPlugin` for clock-free scenario steps.
- Scenario types/schema live in the bin: plugin-rpc cannot depend on the
  fake-provider plugin. `slot::placed` reads the folded `SessionState`.
- `Connection::spawn` already uses `bingo-process`; validate reuses it.
  Manifest names equal directory names; capabilities are method names.
- The landed link spelling is `turn.complete`, with flat tool fields.
  The renderer and matcher stay independent of the seven current points.
- SDK runtime tests live beside sources; the CLI split may use submodules
  for scenario host lifecycle and expectations. No new external crates.
