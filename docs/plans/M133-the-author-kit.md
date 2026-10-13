# M133 — The author kit

## Goal

ADR-0075's protocol 2 gets its first outside speaker. A TypeScript SDK
whose wire types are generated from `schema/plugin.json`, two commands an
author runs — `bingo plugins validate <dir>` and `bingo plugins test
<dir>` — and an example, `examples/plugins/ts-status`, that does Phase 5's
acceptance test from a directory outside the workspace: it places a status
entry, intercepts a tool call, and keeps its state across a hot reload.
No JS engine is embedded; the SDK runs under node, deno or bun.

## Bricks, in build order

1. **Contract: the scenario file and the generated wire, tests first.**
   `bingo plugins test` reads `<dir>/scenarios/*.json`: `{model, steps}`,
   where `model` is a `bingo-provider-fake` script and each step is one of
   `submit {text}`, `reload {}`, or `expect` over what the session showed
   — `slot {slot, contains}`, `tool {name, contains}`, `denied {name}`,
   `notice {contains}`. A Rust type with its schema committed as
   `schema/plugin-scenario.json` (the `BINGO_UPDATE_SCHEMA` test of
   `schema.rs`). In `bingo-plugin-rpc`, `wire_ts.rs` renders
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
   - `plugin.ts` — `definePlugin({name, tools, commands, links})`; the
     handshake with protocol 2 and its capabilities; `tools/changed`.
   - `link.ts` — `before(point, matcher, (host, input) => …)` answers in
     one phase; `around(point, matcher, async (host, input, next) => …)`
     declares `after`, suspends at `await next(input)` until the
     `/after` phase and answers with what the handler returns;
     `intercept/cancel` rejects the pending `next` with a cancellation.
   - `host.ts` — `place`, `extend`, `deliver`, `state.get/set` with
     `ifVersion`, `complete`, `service`.
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
6. **`examples/plugins/ts-status`.** Counts the tool calls it has seen in
   `host/state` (`global`), shows the count in the status slot, and at
   `tool.gate` denies `Bash` whose command starts with `rm -rf` — the
   `tighten` tier's own right, no trust grant needed.
   `scenarios/acceptance.json` reads a file twice, reloads, reads again,
   expects `3` in the slot, then sees `rm -rf` denied.
7. **The acceptance run.** A black-box test copies `ts-status` and
   `sdk/typescript/src` into a temporary directory, keeping their relative
   paths, and runs `bingo plugins validate` and `bingo plugins test` there:
   nothing it runs is a workspace file. It is skipped with a printed reason
   when `node --version` is absent or older than type stripping by default.

## Files

- `sdk/typescript/{package.json (private, no dependencies),README.md}`
- `sdk/typescript/src/{wire.ts (generated),rpc.ts,plugin.ts,link.ts,host.ts,index.ts}`
- `crates/bingo-plugin-rpc/src/{wire_ts.rs,validate.rs}` (new), `lib.rs`, `schema.rs`
- `crates/bingo/src/plugins.rs` → `plugins/{mod.rs,switch.rs,validate.rs,scenario.rs}`
- `crates/bingo/src/main.rs` (`plugins::Action::{Validate,Test}`)
- `schema/plugin-scenario.json` (new)
- `examples/plugins/ts-status/{plugin.json,main.ts,scenarios/acceptance.json,README.md}`
- `crates/bingo/tests/plugin_kit.rs` (new)
- `.github/workflows/ci.yml` — `actions/setup-node`, pinned, Linux job
  only, so the acceptance run is not skipped there (this line justifies it)

## Exit criteria

- [ ] `cargo test -p bingo-plugin-rpc wire_ts::` passes; editing
      `wire.ts` by hand makes it fail (shown once, pasted).
- [ ] `cargo test -p bingo-plugin-rpc validate::`: a missing entry, a bad
      name and a protocol-1 plugin each fail with the field named.
- [ ] `cargo test -p bingo --test plugin_kit`: `validate` exits 0 on
      `wordcount` and `ts-status`; `test` exits 0 on `ts-status`'s
      acceptance scenario from a temporary directory; a scenario with a
      wrong `expect` exits 1 naming the step.
- [ ] CI's Linux job log shows the acceptance run ran, not skipped.
- [ ] `node sdk/typescript/src/index.ts` exits 0 with no output on the
      pinned node (the SDK parses under type stripping), pasted.
- [ ] All gates of the protocol; `scripts/budget.sh` unchanged in crates.

## Non-goals

- Publishing the SDK to a registry; `bingo plugins new`; a template.
- Type-checking the SDK in CI (type stripping checks nothing; `tsc` is a
  dependency); the example running is the test.
- An SDK in another language; an in-process JS engine (refused).
- Expectations over plugin state directly: the example shows its state
  in its slot, which is what a person would see.

## Risks

- Node's default type stripping arrived in 23.6 and was backported to
  22.18 (believed; verify both, and what deno and bun need) — the skip
  reads the version, not the name.
- `around` holds a pending promise across two requests; a plugin that
  never calls `next` and never returns holds a chain until the `HOOK`
  deadline. The deadline is the host's (ADR-0074 §4), not the SDK's.
- `FakeProvider` in a user-facing command: it is already in the bin;
  if M111 or M118 moved it, `plugins test` follows it there.
- Re-verify at start: M132's verbs and capability names as landed (the
  generator reads them, it does not restate them); ADR-0070 slots as
  M121/M123 built them (what `expect slot` reads off `SessionState`);
  M128's `bingo-process` for `validate`'s spawn; ADR-0068/M116 for the
  esc test of `around`; M111's test host for the bin's unit tests.
