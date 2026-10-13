# M137 — The mod runtime

## Goal

ADR-0078 §2–5 and §7. A plugin whose manifest names a `module` runs in
process: one QuickJS-NG runtime per mod, TS stripped with oxc,
`register(on, options)` and hooks at M136's catalogue entries. Shared
manifest, discovery and state live in `bingo-plugin-host`. M133's
`ts-status` must run in process and under node with the same results.

Re-verified at `23810b23` on 2026-10-14; M108–M136 and M147 are landed.
Current paths and required adapters replace the original plan's stale paths.

## Bricks, in build order

1. **Shared library.** Move manifest, discovery, tool names, file stamps,
   state (`swap`, shelf layout, `StateValue`) and discovery notices into
   library-tier `bingo-plugin-host`. Preserve RPC re-exports and existing
   tests. Require exactly one of `module` or `entry`; pure
   `runtime_of(manifest, mods_stand)` selects exactly one runtime per name.
2. **Failing contracts.** Pin module manifests and runtime selection first.
   Fixture mods: `hello` answers `tool.call`; `loop` loops; `hog` exceeds
   memory; `thrower` throws with `.catch`; `reentrant` calls the event it
   hooks; `escaper` attempts eval, Function, import() and require.
3. **Engine and cost.** Pin `rquickjs = "=0.14.0"` with futures, loader,
   parallel and no bindgen; oxc crates `=0.152.0` (accepted 2026-10-13).
   Each mod's thread owns its AsyncRuntime on current-thread tokio; calls
   cross channels, so JS never occupies the worker handling esc. Explain
   the C-engine exception by the onig ban in deny.toml. Measure budget
   before/after; increase dependencies by measured families; cargo deny.
   Core is a dev dependency only, for offline source-registration contracts;
   the shared host uses existing async-trait for operation faces.
4. **Strip/load.** Pure `strip(path, source)` uses oxc parser, semantic,
   transformer and codegen; strip TS and compile JSX to h/Fragment.
   Refuse import(), require and specifiers other than bingo or relative
   files inside the mod root, naming file:line. Loader enforces the same.
5. **Sandbox.** Custom context with only required intrinsics; hide eval/Function and all
   function constructors before mod code. No Node, DOM,
   WebAssembly or timers. Install h/Fragment JSON nodes, URL (url), UTF-8
   TextEncoder/Decoder, AbortController, randomUUID (existing getrandom), SHA-256
   (bingo-digest) and performance.now. Memory defaults to mods.memoryMiB=64;
   cap the stack. Escaper fails in words; hog fails without killing host.
6. **One bingo module.** Split pure registration/matching in on.ts from
   node intercept transport in cross.ts. Export Register and event types
   under Claude Code names; include/strip on.ts in process. Duplicate
   registration throws; one catch per hook; options is validated config.
   State helpers/types remain M141/M142.
7. **Links.** Mods implements InterceptorSource, one Interceptor per mod,
   matched by patterns/prefilter. Run on its thread with deeply frozen e;
   next runs once, exposing called, signal, origin {plugin,tier}, budget
   {ms,remainingMs}. Observation return values are ignored.
8. **Budgets/fail-open.** Pure Budget resume/suspend/spent with fake-time
   table tests. Count own time only, suspended around next and dollar.
   Interrupt after 10 s (setting; tests use 50 ms and assert outcome).
   Catch gets 1 s with next.error {kind,message}, kinds throw/timeout/
   re-entry/shape, and next.called. Skip a failed hook or keep its last
   next result; one notice names plugin/event/reason. Re-entry leaves out
   only the calling hook. Dollar awaits have wall bounds, tested with
   paused tokio; dropped chains interrupt JS and abort signal.
9. **Grants.** Pure library grant(runtime,origin,trust,uses) and admits(grant,noun,
   method). Processes default tighten; user/explicit modules default rewrite
   with declared uses; project modules tighten/safe nouns. pluginTrust lifts grants
   and alone gives full. Refuse unsafe process/http/env/model, external fs
   writes and unsafe face methods without grants. Uses includes host and
   service nouns. Core raises plugin.register on first source report and
   reload, with name/version/origin/runtime/tier/uses. Links may only lower
   tier, drop uses or deny with reason. Both runtimes read granted uses
   from catalog(Plugins), including RPC model/complete admission.
10. **Reload.** Poll explicit directories on tokio time; changed marks stale.
    Reload after turn.complete (own link calls next) or before own tools/
    commands (M138 registers them). Drop/join old thread, fresh register,
    session.start for every open session; keep host state. No test sleeps.
11. **Parity.** Until M138, dollar maps M133's status, state/store, model
    complete, session deliver/extend and service faces to existing doors.
    ts-status adds module hooks/register.ts and imports bingo. Run M133's
    scenario in process and under node using a process-manifest fixture.

## Files

- New `crates/bingo-plugin-host/{Cargo.toml,src/*,tests/*}` and
  `crates/bingo-mods/{Cargo.toml,src/*,tests/*}`; runtime guide.md.
- `crates/bingo-plugin-rpc/Cargo.toml`, `src/{manifest,discovery,notice,tool,
  reload,state,host,lib,manager,bridge,source,service,validate,schema,
  wire_ts,events_ts}.rs`: shared mechanisms and runtime/grant adapters.
  `tests/plugin/*.rs`: the integration harness and the fixtures it drives;
  register, publish grants and declare uses; preserve every assertion.
- `crates/bingo-sdk/src/{lib,plugin,intercept,catalogue}.rs`, catalogue/payload.rs,
  tests/plugin_registration.rs;
  `crates/bingo-core/src/{plugins,host}.rs`, turn/late.rs, chain/*,
  host/{registry,catalog,interception}.rs: registration and granted catalogue.
  session/running.rs and session/tests/owned.rs: bounded diagnosis; change
  only if supervision adds a real extra hop (authorized 2026-10-14).
- `crates/bingo/{Cargo.toml,src/main.rs,src/composition.rs,src/plugins/*,tests/plugin_kit.rs}`:
  runtime selection, isolated scenario host and runtime listing column.
  `tests/plugin_rpc.rs`: declare only the side fixture's session use;
  preserve every assertion. `tests/cli/plugins.rs` stays unchanged.
- `sdk/typescript/src/{on,cross,plugin,index,events,wire}.ts`, on.test.ts;
  `examples/plugins/ts-status/{plugin.json,hooks/register.ts}`.
- Cargo.toml, Cargo.lock, deny.toml, scripts/budget.toml; generated schema/*.
  Regenerate schemas/fixtures/events.json/wire.ts with BINGO_UPDATE_SCHEMA=1.

## Exit criteria

- [ ] Shared library final suite pending; all 49 existing RPC assertions pass.
- [x] Mod tests cover strip file:line, all six fixtures and one notice each;
      thrower's catch sees kind=throw.
- [x] Esc during loop at tool.check ends Interrupted within M116's bound.
- [ ] Project/model, registration denial and runtime-specific grant tests;
      the existing process CLI default remains tighten.
- [x] Mid-turn edit reloads after turn.complete; store survives; no sleeps.
- [x] plugin_kit runs ts-status in process and node (M133's node skip rule).
- [ ] Final serial gates, before/after budget and each touched crate's MSVC
      check. C-header-only failures defer per step 7.

## Non-goals

Dollar beyond M133 (M138/139), elements (M140), module validate/test (M142),
npm imports, mod-owned threads/timers, shared JS runtimes, Client modules,
file-watcher dependencies, watching non-explicit directories; no live tests.

## Risks

- QuickJS parallel/exotic-class issue #764: natives are plain functions and
  objects, host work is asynchronous and bounded. MSVC builds are checked
  locally where possible and in Windows CI; never cfg-gate failures away.
- Pin weekly oxc; measure 50 idle fixture mods' RSS without an assertion.
- rquickjs requires Eval internally for safe module compilation. Disable
  JavaScript evaluation entry points before loading; no unsafe bytecode API.
- Core's current external deadline is 5 s: preserve it for process links
  while allowing mods to own their 10 s active-time budget.
- Shared state uses bingo-jsonrpc library errors, never a plugin dependency.
- M148 owns SDK state.rs and TUI/demo/rooms/schedule; leave them untouched.
- Resolved 2026-10-14: one Manifest, exactly one entry/module; generated
  plugin.json/plugin-2.json and amended baseline committed together.
  Mechanical constructors: process reload Launch and core commands/plugins.rs.
- Resolved 2026-10-14: RPC integration fixtures register and publish grants;
  all assertions, including default model refusal, stay; no new dev dependency.
  Do not merge or rebase dev.
- Root cause (2026-10-14): pausing after JS startup exposes Tokio's ≤1 ms deadline rounding; the actor adds no hop and the grace remains 2 s.

- Resolved 2026-10-14 by the person: runtime is an input to the one grant
  function. ADR-0074 process defaults remain tighten; ADR-0078's rewrite
  default applies only to user/explicit modules. Table-test runtime × origin
  × trust. Safe declared session calls need no tier lift. The RPC harness
  supplies explicit fixture trust where its model/service tests need it.
  Include only bin tests/plugin_rpc.rs for the session-use declaration;
  leave the CLI default-tier assertion and all other assertions unchanged.

## Verified (2026-10-14)

```text
scripts/budget.sh # baseline before extraction
dependencies (unique, normal): 355 (max  355)
warm cargo check -p bingo-core: 0s (max  20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
budget ok
```

```text
cargo test -p bingo-mods --locked
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.90s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```text
cargo test -p bingo-mods --test engine fifty_idle_modules_rss --locked -- --ignored --nocapture
idle modules: 50; RSS KiB before=3232; after=29792
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.77s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.20s
```

```text
cargo test -p bingo --test plugin_kit --locked
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.63s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.36s
```

```text
cargo test -p bingo-plugin-rpc --locked
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.38s
test result: ok. 128 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.29s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```text
cargo fmt --all -- --check
exit 0 (no output)
```

```text
cargo check --workspace --all-targets --locked
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.54s
```

```text
cargo clippy --workspace --all-targets --locked -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.42s
```

```text
cargo test --workspace --locked
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1m 03s
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.34s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.07s
test result: FAILED. 258 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 21.26s
plugins::an_external_plugins_tier_is_listed_from_user_settings_only ... FAILED
panicked at crates/bingo/tests/cli/plugins.rs:247:5:
assertion `left == right` failed
  left: "rewrite"
 right: "tighten"
error: test failed, to rerun pass `-p bingo --test cli`
exit 101
```

```text
scripts/check_discipline.sh
Not run: protocol step 6 stop.
```

```text
scripts/budget.sh
Not run: protocol step 6 stop.
```

```text
cargo deny check
Not run: protocol step 6 stop.
```

```text
cargo check -p bingo-sdk --all-targets --target x86_64-pc-windows-msvc
Not run: protocol step 6 stop.
```

```text
cargo check -p bingo-core --all-targets --target x86_64-pc-windows-msvc
Not run: protocol step 6 stop.
```

```text
cargo check -p bingo-plugin-host --all-targets --target x86_64-pc-windows-msvc
Not run: protocol step 6 stop.
```

```text
cargo check -p bingo-plugin-rpc --all-targets --target x86_64-pc-windows-msvc
Not run: protocol step 6 stop.
```

```text
cargo check -p bingo-mods --all-targets --target x86_64-pc-windows-msvc
Not run: protocol step 6 stop.
```

```text
cargo check -p bingo --all-targets --target x86_64-pc-windows-msvc
Not run: protocol step 6 stop.
```
