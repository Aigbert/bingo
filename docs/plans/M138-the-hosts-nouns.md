# M138 — The host's nouns

## Goal

ADR-0078's `$`, part I (mods-parity §3). The nouns the host already has
doors for reach a mod: `session`, `turn`, `prompt`, `tool`, `command`,
`config`, `model`, `state`, `store`, `clock` and `plugin`. Each method is
raised as `<noun>.<method>` before the host performs it, so a link may
rewrite, deny or answer it. Every method translates onto an existing door,
and none opens a new one. One table defines `$`, and both transports read
it: natives in process, and M133's `dollar.ts` under node.

## Bricks, in build order

0. **Node re-entry prerequisite (authorized 2026-10-14).** Preserve the shared fixture and bounded assertion; carry per-invocation hook identity across reverse RPC. Bind it at the bridge, isolate concurrent calls, and land a separate fix before runtime features.

1. **The table, as a contract.** `crates/bingo-mods/src/dollar/table.rs` has one row per method: `(noun, method, door, unsafe, input, result)`. A door is `Kernel(&'static Entry)`, whose door already raises a chain of this meaning, so `$` calls it and raises nothing more; or `Face`, a method of a face the runtime registers, raised through the service-wire door by M136's wrapper (ADR-0078 §1). M136's one-namespace rule keeps a name to one owner. A `BINGO_UPDATE_SCHEMA` test writes `schema/dollar.json`, and M133's `render` turns it into `sdk/typescript/src/dollar-types.ts`. Failing test: on M111's test host, a fixture mod calls every row once, a link hooked at each name sees exactly one call, and a row raised twice fails it.
2. **Kernel doors.** `prompt.submit` is `submit`; `session.append` is `extend`; `session.send` is `HostApi::deliver`, which raises it, and the target's actor then raises `session.receive` (M136); `session.compact` and `command.run` submit `Input::Action`; `tool.call` is `HostApi::invoke`, which runs `tool.check` and `AfterTool` (M131). `config.set` is `HostApi::reconfigure`, routed through M136's `config::set`, so the chain is raised once wherever a setting changes. `model.complete` joins the catalogue as a chain at `HostApi::complete`, so a process plugin's `host/model/complete` and a mod's call raise it alike, inside a turn or outside one (M132).
3. **Reads from the fold.** The runtime is a client (ADR-0002). For each session it serves, it opens the stream lazily as `ClientIdentity` `mods` and folds it with `SessionState::apply`. A pure `read(&state, method) -> Value` answers, with table tests: `session.id`, `cwd`, `model`, `turns`, `usage` (context and last-turn usage); `config.list`/`read` (`ConfigView`); `prompt.read` (queue and last submitted text). `session.messages` pages through `history`; `session.surfaces` folds attach/detach; `session.root` is discovery's project directory; `session.version` is the workspace version. Nothing is kept beside the fold.
4. **Face writes.** `session.authorize` asks through `HostApi::ask`; `tool.list` and `command.list` are `catalog(Tools)` and `catalog(Commands)`. `turn.abort` interrupts; a person's `esc` raises nothing and waits on no link (ADR-0068), only a mod's abort is an event.
5. **`$.plugin`.** `{ name, root }`, from the mod's manifest, frozen and set at load. It is a property, not a method: it has no row and raises nothing. Under node, `dollar.ts` builds it from `initialize`, whose params already carry `plugin_name` beside `plugin_root` (M137).
6. **`tool.register` and `command.register`.** The runtime's `ToolSource` names each tool with `bingo-plugin-host`'s `tool_name` (`plugin__<mod>__<name>`); an undeclared property fails closed (not read-only, not concurrency-safe). A call runs M137's reload check, then the handler on the mod's thread under the hook budget; a throw is the tool's error. The `CommandSource` registers commands unprefixed, as plugin-rpc's are; the later duplicate drop and its notice stay.
7. **`state` and `store` (ADR-0078 §6).** A face over M137's library store: session/global scope, versioned, with `ifVersion` returning the held version on conflict; 4 MiB per plugin in all. The door binds and pins `plugin` for modules and processes: others may rewrite `state.set`, but a direct foreign write is refused. `host/state/*` calls the face when present. Each hook's read log records `(plugin, scope, key, version)` for M140's render subscriptions.
8. **`clock`.** `now` is `jiff`'s timestamp; `sleep`, `after` and `every` run on tokio time, paused in tests (M111). A callback runs as a hook under the budget; timers drop at a reload and at `session.end`; `every` has a floor of 1 s, a setting.
9. **`model.classify` and `model.fork`**, pure bricks over `complete`: `classify_request(text, labels)` and `pick(answer, labels) -> Option<label>` (no label is refused in words); `fork_request(history, prompt)` from `history`. Like `complete`, both need the `model` grant through `admits` and are billed by `UsageCharged` (M132): to the running turn, or to the session when none runs.
10. **Node parity.** `dollar.ts` calls each `Face` row as `service/call` to the runtime's face keys, and each `Kernel` row by its protocol-2 verb. A fixture mod calls one method of every noun, and gives the same output under both transports.

## Files

- `crates/bingo-mods/src/dollar/*.rs` (new), `src/{lib,link,chain,engine,thread,module,manager,plugin,load,config}.rs`, `src/{thread.js,manager/reload.rs}`, `tests/{contracts,plugin}.rs`, `tests/mods/nouns/*` (new), `guide.md`, `Cargo.toml`
- `crates/bingo-sdk/src/{catalogue.rs,catalogue/payload.rs}`; `crates/bingo-core/src/{host.rs,config.rs,commands/model.rs,commands/think.rs,host/completion.rs,host/tests/complete.rs,host/tests/catalogue.rs}`
- `crates/bingo-plugin-host/src/{state,ops,host,invocation,lib}.rs`
- `crates/bingo-plugin-rpc/src/{host,service,bridge,schema,wire,wire_ts,context,connection,intercept,lib}.rs`
- `schema/{dollar,plugin,events}.json`, `sdk/typescript/src/{cross,rpc,dollar,dollar-types,plugin,wire,events}.ts`, SDK noun tests
- `crates/bingo/tests/{plugin_kit.rs,support/reentry.rs}`, noun fixture modules and mechanical schema/fixture consumers
- Existing workspace dependencies only: `jiff` for the specified timestamp, and `bingo-schema-ts` as a dev dependency for the generated dollar types; measure with `scripts/budget.sh`.

## Exit criteria

- [ ] `cargo test -p bingo-mods dollar::`: every row answers on M111's test host, and a link at each name sees exactly one call. A row raised twice fails the test (shown once, pasted).
- [ ] Drift tests fail on a hand edit of `dollar.json` or `dollar-types.ts`.
- [ ] Every `read` answer equals the field of the same `SessionState`.
- [ ] A mod's `$.session.send` is seen once at `session.send` and once at `session.receive`. `$.plugin` names the fixture and its root under both transports, and assigning to it throws.
- [ ] A registered mod tool is offered to the fake model as `plugin__<mod>__<name>` and runs. When the mod is stale, it reloads first.
- [ ] A stale `ifVersion` gets the held version; a foreign key's write is refused; the read log names each key a hook read.
- [ ] Clock: under paused time, `every` fires once per period, and no timer survives a reload.
- [ ] `model.classify` and `model.fork` show as `UsageCharged` in the turn's usage; called from a timer, in the session's. Without the `model` grant, each is refused in words.
- [ ] `cargo test -p bingo --test plugin_kit`: the nouns fixture gives the same output in process and under node.
- [ ] All gates pass, including `cargo check --target x86_64-pc-windows-msvc` for core and plugin-rpc.

## Non-goals

- `fs`, `http`, `process`, `env`, `mcp` and `agent` (M139). `ui` and `prompt.fill`/`suggest` (M140, M141).
- The gaps of mods-parity §8, each kept out by ADR-0078's ratchet with its reason: `tool.check` without a run (no door judges a call without making it; a mod hooks `tool.check`); `prompt.compose` as a read (no composed prompt is published); `prompt.edit` and unsent text (the surface's); `session.repo` (no VCS fact is kept; `$.process` asks git).

## Risks

- `model.fork` lacks contributor pieces; document the different context.
- Timer model calls require the model grant and charge the session without a turn.
- With mods off, process state falls back to the library without noun chains.
- Re-verified at `c1d95e8c` on 2026-10-14 (M108–M137, M147–M150 landed).
- Reload lives in `mods/src/manager/reload.rs`; shared faces live in `plugin-host/src/ops.rs`. Remove its `model.complete` face when the kernel claims the name. `initialize.plugin_name` already exists; preserve its shape.
- Preserve grants: processes=tighten, user/explicit modules=rewrite, project=tighten.
- `core/src/chain/service.rs` wraps faces; SDK `service.rs` checks unique names. `HostApi::complete` only grants and mails. Move the model/think command's `config::set` calls into `reconfigure` to avoid duplicate chains.
- The fold has config, queue, transcript and usage. Attach/detach are observations, not frames; project surfaces through the existing plugin signal door.
- Use M111's `TestHost` for door answers, and the real offline kernel for chain counts: the test host does not dispatch interception chains.
- The Node re-entry prerequisite was repaired here as the authorized first brick; ADR-0078 §4 remains unchanged.

- Prerequisite Windows checks: plugin-host and core pass; mods (rquickjs-sys), plugin-rpc and bingo (aws-lc-sys) await Windows CI because this Mac lacks Windows SDK C headers.

## Verified (2026-10-14)

```text
node --test sdk/typescript/src/dollar.test.ts
Error: the calling hook re-entered through its own dollar call
2 !== 1
ℹ tests 1
ℹ pass 0
ℹ fail 1
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk -p bingo-plugin-rpc --lib --locked
test result: ok. 130 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 136 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.00s
node --test sdk/typescript/src/*.test.ts
ℹ tests 15
ℹ pass 15
ℹ fail 0
cargo test -p bingo --test plugin_kit --locked node_reentry -- --nocapture
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out; finished in 0.09s
cargo test -p bingo-mods --test contracts --locked a_reentrant_call -- --nocapture
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.02s
cargo fmt --all -- --check
exit 0 (no output)
cargo check --workspace --all-targets --locked
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.59s
cargo clippy --workspace --all-targets --locked -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.93s
cargo test --workspace --locked -- --skip a_proxy_with_a_token_this_run_never_minted_gets_nothing
exit 0
cargo test -p bingo --test acp_bridge --locked a_proxy_with_a_token_this_run_never_minted_gets_nothing -- --exact --test-threads=1
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.04s
scripts/check_discipline.sh
discipline ok
scripts/budget.sh
dependencies (unique, normal): 410 (max  410)
budget ok
cargo deny check
advisories ok, bans ok, licenses ok, sources ok
cargo check -p bingo-plugin-host --all-targets --target x86_64-pc-windows-msvc --locked
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.66s
CC_x86_64_pc_windows_msvc='clang --driver-mode=cl' cargo check -p bingo-mods --all-targets --target x86_64-pc-windows-msvc --locked
rquickjs-sys: fatal error: 'stdlib.h' file not found
CC_x86_64_pc_windows_msvc='clang --driver-mode=cl' cargo check -p bingo-plugin-rpc --all-targets --target x86_64-pc-windows-msvc --locked
aws-lc-sys: fatal error: 'windows.h' file not found
CC_x86_64_pc_windows_msvc='clang --driver-mode=cl' cargo check -p bingo --all-targets --target x86_64-pc-windows-msvc --locked
aws-lc-sys: fatal error: 'stdlib.h' file not found
cargo check -p bingo-core --all-targets --target x86_64-pc-windows-msvc --locked
Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.12s
cargo test -p bingo-core --lib --offline a_completion_crosses_its_kernel_chain_once
left: 0; right: 1
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 498 filtered out; finished in 0.04s
cargo test -p bingo-core --lib --locked a_denied_setting_changes_neither_the_session_nor_the_file
direct reconfigure bypassed config.set
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 498 filtered out; finished in 0.05s
cargo test -p bingo-mods --lib --locked dollar::read
unknown folded read: session.id
cargo test -p bingo-mods --locked
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
```
