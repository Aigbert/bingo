# M133 — The author kit

## Goal

Deliver ADR-0075's first outside protocol-2 speaker: generated TypeScript
wire types, a dependency-free SDK, `bingo plugins validate/test <dir>`,
and `examples/plugins/ts-status`. From outside the workspace it places a
status entry, intercepts a tool, and keeps state across reload. Its
ADR-0078 §2 `register(on, options)` module is also M137's input.

## Bricks, in build order

1. Contracts first: `{model, steps}` scenarios use the fake provider's
   own script type. Steps are `submit {text}`, `reload {}`, or `expect`
   with `slot {slot, contains}`, `tool {name, contains}`, `denied {name}`,
   or `notice {contains}`. Generate `schema/plugin-scenario.json` from
   Rust; drift tests and matcher/continuation fixtures fail before code.
2. A pure `bingo-schema-ts::render(&Value) -> String` renders erasable
   aliases, unions, objects, references and arrays. It generates `wire.ts`
   from the plugin schema; drift is checked, never repaired by hand.
3. SDK: `rpc.ts` handles NDJSON, ids, pending calls and METHOD_NOT_FOUND;
   `plugin.ts` loads register, negotiates capabilities and declares links;
   `on.ts` handles exact/ns.*/ */! patterns, nested/scalar/RegExp/array
   matchers and one-use `next`. `/after` resolves it; cancel rejects it.
   `dollar.ts` maps status, session/global state, model complete, session
   deliver/extend and service nouns to protocol-2 doors. Only `node:`
   built-ins; `.ts` imports and `import type`; no npm dependencies.
4. Validate parses name/entry/config-schema structure, uses the host's
   own process spawner, handshakes, lists tools/commands/link points and
   stops the process. Findings are lines; exit status is 0 or 1.
5. Test runs a real host with FakeProvider and explicit plugin discovery.
   Each scenario has an isolated home and project. Fold frames with
   `SessionState::apply` (ADR-0002); read `slot::placed` (ADR-0070).
   Explicit reload shares the watcher's Bridge path; no clock sleeps.
   Failures name the step and show the folded session.
6. `ts-status/hooks/register.ts` counts calls in global store, places
   status, and denies `rm -rf` Bash commands at tighten tier. Its two-line
   process entry imports the module; M137 can load register itself.
7. Black-box tests copy example and SDK into a temporary directory, then
   validate and run acceptance there. They also validate wordcount and
   check failure status/stdout/diagnostics. Unsupported Node prints a
   skip reason; pinned Linux CI requires Node and prints acceptance proof.

## Files

- `sdk/typescript/{package.json,README.md,src/{wire,rpc,plugin,on,dollar,index}.ts}`
  and adjacent `*.test.ts` (wire is generated).
- New library `crates/bingo-schema-ts/{Cargo.toml,src/lib.rs}`.
- `crates/bingo-plugin-rpc/src/{wire_ts,validate,lib,manager}.rs`.
- Bin `plugins.rs` becomes `plugins/{mod,switch,validate,scenario,schema,expect}.rs`;
  `crates/bingo/src/main.rs`, `crates/bingo/tests/plugin_kit.rs`.
- `crates/bingo-provider-fake/{Cargo.toml,src/lib.rs}`: schema derives only.
- Generated `schema/plugin-scenario.json`; examples `ts-status/{plugin.json,
  main.ts,hooks/register.ts,scenarios/acceptance.json,README.md}`.
- Workspace/affected Cargo manifests and lock. Existing serde/schemars/futures
  edges support typed scenarios; existing tempfile supplies disposable homes.
  `scripts/budget.toml`: 353 → 355 (renderer member and tempfile promoted
  from dev to normal). No newly resolved external crate.
- `.github/workflows/ci.yml`: setup-node pinned by SHA, Node 24.17.0,
  Linux SDK tests and required, visible acceptance execution.

## Exit criteria

- [ ] Renderer and wire drift tests pass; deliberate manual drift fails once.
- [x] Validate tests name missing entry, bad name and protocol-1 failures.
- [x] External plugin_kit validates wordcount/ts-status, passes acceptance,
  and reports a wrong expectation with exit 1 and the failing step.
- [ ] Linux CI log shows acceptance ran (pending CI; no push authorized).
- [x] `node sdk/typescript/src/index.ts` exits 0 with no output; SDK tests pass.
- [ ] Protocol gates pass, with Windows C-header exceptions recorded.

## Non-goals

Registry publishing, scaffolding, another language SDK, tsc in CI, M137's
embedded runtime, M142's *.test.ts harness, and private-state expectations.

## Risks and re-verification (2026-10-13, e9e45bb9)

- M108–M123/M127–M132 are present. `plugin-2.json` is frozen; no protocol
  shape changes. Capabilities are method names; completion is `turn.complete`.
- Scenario schema belongs in the bin: plugin-rpc cannot import FakeProvider.
  Fake script schema derives avoid a duplicate representation.
- `Connection::spawn` already uses bingo-process (ADR-0072). Explicit
  `Manager`/`PluginRpcPlugin::reload` calls `Bridge::reload`, as the poll does.
- Pending next lives across two requests; the host owns the HOOK deadline
  and cancellation (ADR-0074 §4 / ADR-0068), with SDK cancel fixtures.
- Missing state reads null; reload notices are transcript items. `denied`
  checks error output containing “denied”: the stream has no separate verdict.
- Node 22.18/23.6 enable default stripping; tested locally on pinned 24.17.0.
  Deno/Bun guidance cites their APIs, but acceptance execution uses Node.
- M136 can reuse the renderer and extend on.ts without a hard-coded catalogue.

### Stop: Execution protocol step 6

The workspace gate and an isolated rerun both fail the existing ACP test
`crates/bingo/tests/cli/acp/bridge.rs:246`. It expects “no turn is in flight”
and receives “[Request interrupted by user for tool use]”. Its assertion
and ACP behavior are outside M133 and overlap M134; neither was changed.
Options: resume after the ACP owner resolves it, or explicitly authorize
coordinated investigation outside M133. No further gates ran after the repeat.
Review follow-ups on resume: an omitted matcher must accept null-input events
(currently it defaults to `{}`); add cancellation-before-next cleanup coverage.
The added startup-refusal black-box test compiled but has not run yet.

## Verified (2026-10-13; incomplete, stopped)

Commands and trimmed output, in execution order:
```text
cargo test -p bingo-schema-ts                         # contracts first
FAILED. 0 passed; 2 failed
cargo test -p bingo-plugin-rpc wire_ts::              # contracts first
FAILED. 0 passed; 1 failed: renderer must emit the protocol types
cargo test -p bingo committed_scenario_schema        # contracts first
FAILED. 0 passed; 1 failed: committed schema absent
node --test sdk/typescript/src/on.test.ts             # contracts first
ERR_MODULE_NOT_FOUND: sdk/typescript/src/on.ts
cargo test -p bingo-schema-ts                        # implementation
ok. 2 passed; 0 failed
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo --bin bingo committed_scenario_schema
ok. 1 passed; 0 failed
cargo test -p bingo-plugin-rpc wire_ts::              # deliberate drift
FAILED. 0 passed; 1 failed: wire.ts drift
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc wire_ts::
ok. 1 passed; 0 failed                               # regenerated
cargo test -p bingo-plugin-rpc validate::
ok. 2 passed; 0 failed
cargo test -p bingo --test plugin_kit -- --nocapture
external TypeScript acceptance ran (validate, state, reload, slot, denial)
ok. 4 passed; 0 failed
node --test sdk/typescript/src/*.test.ts
pass 7; fail 0
node --version                                      # v24.17.0
node sdk/typescript/src/index.ts                     # exit 0, no output
cargo fmt --all -- --check                           # exit 0, no output
cargo check --workspace --all-targets --locked
Finished `dev` profile [unoptimized + debuginfo] target(s) in 27.24s
cargo clippy --workspace --all-targets --locked -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 31.96s
RUST_TEST_THREADS=1 cargo test --workspace --locked
FAILED. 252 passed; 1 failed (bingo --test cli, ACP assertion above)
cargo test -p bingo --test cli acp::bridge::a_call_with_no_turn_in_flight_is_refused_with_a_reason --locked -- --exact --nocapture --test-threads=1
FAILED. 0 passed; 1 failed; 252 filtered out (same assertion)
```
Remaining: current renderer/wire checks without update mode; full workspace
suite; discipline; budget; deny; MSVC for schema-ts/plugin-rpc/provider-fake/bin;
Linux CI acceptance evidence. Protocol 2's frozen schema is unchanged.
