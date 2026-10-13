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
9. **Grants.** Pure library grant(origin,trust,uses) and admits(grant,noun,
   method). User/explicit directories default rewrite with declared uses;
   project defaults tighten/safe nouns; user-only pluginTrust lifts grants
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
- `crates/bingo-sdk/src/{plugin,intercept,catalogue}.rs`, catalogue/payload.rs;
  `crates/bingo-core/src/{plugins,host}.rs`, turn/late.rs, chain/*,
  host/{registry,catalog,interception}.rs: registration and granted catalogue.
- `crates/bingo/{Cargo.toml,src/main.rs,src/plugins/*,tests/plugin_kit.rs}`:
  runtime selection, isolated scenario host and runtime listing column.
- `sdk/typescript/src/{on,cross,plugin,index,events,wire}.ts`, on.test.ts;
  `examples/plugins/ts-status/{plugin.json,hooks/register.ts}`.
- Cargo.toml, Cargo.lock, deny.toml, scripts/budget.toml; generated schema/*.
  Regenerate schemas/fixtures/events.json/wire.ts with BINGO_UPDATE_SCHEMA=1.

## Exit criteria

- [ ] Shared library contracts pass; plugin-rpc's existing assertions unchanged.
- [ ] Mod tests cover strip file:line, all six fixtures and one notice each;
      thrower's catch sees kind=throw.
- [ ] Esc during loop at tool.check ends Interrupted within M116's bound.
- [ ] Project model call refused, then allowed by pluginTrust; denied
      plugin.register switches off the mod and plugins list shows why.
- [ ] Mid-turn edit reloads after turn.complete; store survives; no sleeps.
- [ ] plugin_kit runs ts-status in process and node (M133's node skip rule).
- [ ] Before/after budget, all serial gates and each touched crate's MSVC
      check; C-header-only failures deferred to Windows CI per step 7.

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
- Current catalogue has only enabled/reason/from; plugin.register is absent.
  Shared state uses bingo-jsonrpc library errors, never a plugin dependency.
- M148 owns SDK state.rs and TUI/demo/rooms/schedule; leave them untouched.
- Resolved 2026-10-14 by the person's delegate: one Manifest, exactly one
  entry/module, named refusals for both/neither; amend only its pre-release
  baseline in the same commit as generated plugin.json/plugin-2.json.
  Process reload fixture constructors change mechanically with Launch.

## Verified (2026-10-14)

Branch: `m137-the-mod-runtime`, base `23810b23`. No push, merge or PR.
The following are partial verification, not milestone completion.

Before extraction, `scripts/budget.sh` (exit 0):
```text
dependencies (unique, normal): 355 (max  355)
warm cargo check -p bingo-core: 0s (max  20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
target/debug: 0 GB (soft max  5)
test binaries: 0
budget ok
```

`cargo test -p bingo-plugin-host -p bingo-plugin-rpc` (exit 0, before new
failing contracts): library, RPC unit, RPC integration, then two doc-test totals:
```text
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 124 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.31s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo fmt --all` (exit 0, no output); after extraction,
`scripts/budget.sh` (exit 0):
```text
dependencies (unique, normal): 356 (max  356)
warm cargo check -p bingo-core: 0s (max  20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
target/debug: 0 GB (soft max  5)
test binaries: 3
budget ok
```

`cargo test -p bingo-plugin-host --test module_manifest` (exit 101):
```text
test a_module_manifest_needs_no_process_entry ... FAILED
ADR-0078 permits a module without a process entry: Error("missing field `entry`", line: 5, column: 1)
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
error: test failed, to rerun pass `-p bingo-plugin-host --test module_manifest`
```

`cargo test -p bingo-plugin-rpc schema::tests::protocol_two_can_describe_a_module_without_a_process_entry -- --exact` (exit 101):
```text
test schema::tests::protocol_two_can_describe_a_module_without_a_process_entry ... FAILED
ADR-0078 module-only manifests conflict with the ADR-0075 frozen required entry
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 124 filtered out; finished in 0.02s
error: test failed, to rerun pass `-p bingo-plugin-rpc --lib`
```

`cargo test -p bingo-plugin-host --test module_manifest` (before implementation):
```text
both_and_neither_name_the_plugin_and_launch_forms ... FAILED
exactly one launch form: Manifest { name: "hello", version: "0.1.0", entry: Entry { command: "node", args: [], env: {} }, config: None, call_timeout: None }
entry_only_and_module_only_round_trip ... FAILED
one launch form: Error("missing field `entry`", line: 5, column: 1)
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

`BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc schema::tests::the_committed_schema_is_this_document -- --exact`:
```text
test schema::tests::the_committed_schema_is_this_document ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 124 filtered out; finished in 0.03s
```

`BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc wire_ts::`:
```text
test wire_ts::tests::committed_wire_is_generated ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 124 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 49 filtered out; finished in 0.00s
```

`cargo test -p bingo-plugin-host -p bingo-plugin-rpc`:
```text
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 125 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.30s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo test -p bingo-mods` (compiler contracts before implementation):
```text
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test typescript_and_jsx_compile_without_a_runtime_import ... FAILED
test imports_stay_inside_the_module_root_or_bingo ... FAILED
test strings_and_comments_do_not_count_as_dynamic_imports ... FAILED
called `Result::unwrap()` on an `Err` value: StripError("view.tsx:1: module compiler unavailable")
called `Result::unwrap()` on an `Err` value: StripError("register.ts:1: module compiler unavailable")
called `Result::unwrap()` on an `Err` value: StripError("register.ts:1: module compiler unavailable")
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

`cargo test -p bingo-mods --test strip`:
```text
test strings_and_comments_do_not_count_as_dynamic_imports ... ok
test typescript_and_jsx_compile_without_a_runtime_import ... ok
test imports_stay_inside_the_module_root_or_bingo ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`scripts/budget.sh` (first engine measurement):
```text
dependencies (unique, normal): 410 (max  409)
FAIL: too many dependencies
warm cargo check -p bingo-core: 0s (max  20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
target/debug: 1 GB (soft max  5)
test binaries: 8
budget FAILED
```

`scripts/budget.sh` (measured cap):
```text
dependencies (unique, normal): 410 (max  410)
warm cargo check -p bingo-core: 1s (max  20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
target/debug: 1 GB (soft max  5)
test binaries: 8
budget ok
```

`cargo deny check`:
```text
advisories ok, bans ok, licenses ok, sources ok
```

`cargo test -p bingo-mods --test sandbox (before implementation)`:
```text
test globals_are_available_and_encode_real_bytes ... FAILED
test trusted_module_loading_survives_without_dynamic_code ... FAILED
called `Result::unwrap()` on an `Err` value: Exception
escaped with eval('1')
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

`cargo test -p bingo-mods --test sandbox`:
```text
test trusted_module_loading_survives_without_dynamic_code ... ok
test globals_are_available_and_encode_real_bytes ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo test -p bingo-mods`:
```text
test budget::tests::only_resumed_intervals_spend_the_budget ... ok
test load::tests::nested_imports_resolve_inside_the_root ... ok
test load::tests::a_symlink_cannot_import_an_outside_file ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test trusted_module_loading_survives_without_dynamic_code ... ok
test globals_are_available_and_encode_real_bytes ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test strings_and_comments_do_not_count_as_dynamic_imports ... ok
test typescript_and_jsx_compile_without_a_runtime_import ... ok
test imports_stay_inside_the_module_root_or_bingo ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`node --test sdk/typescript/src/on.test.ts` (before implementation):
```text
✖ registration rejects duplicate hooks and duplicate catch handlers (0.114041ms)
✖ a hook catch receives the original error and continuation state (0.091833ms)
ℹ tests 9
ℹ pass 7
ℹ fail 2
✖ registration rejects duplicate hooks and duplicate catch handlers (0.114041ms)
  TypeError: Cannot read properties of undefined (reading 'catch')
✖ a hook catch receives the original error and continuation state (0.091833ms)
  TypeError: Cannot read properties of undefined (reading 'catch')
```

`node --test --test-reporter=tap sdk/typescript/src/*.test.ts`:
```text
ok 1 - matchers are partial, nested, scalar, regex or alternatives
ok 2 - patterns do not depend on the current event catalogue
ok 3 - multiple hooks wrap once, preserving after order
ok 4 - short circuit answers without next and cancellation rejects next
ok 5 - an omitted matcher accepts null and scalar event inputs
ok 6 - cancel before next rejects before and clears the chain
ok 7 - cancel while the after hook is waiting rejects after
ok 8 - registration rejects duplicate hooks and duplicate catch handlers
ok 9 - a hook catch receives the original error and continuation state
ok 10 - RPC pairs ids, rejects unknown requests and closes all pending calls
ok 11 - handshake advertises matching points and intersects host capabilities
ok 12 - protocol one is refused before register runs
ok 13 - catalogue subscriptions wrap a service event added after initialization
# tests 13
# pass 13
# fail 0
# skipped 0
```

`cargo test -p bingo-mods --test engine`:
```text
test a_fixture_registers_on_its_own_runtime ... FAILED
called `Result::unwrap()` on an `Err` value: Fault { kind: "load", message: "module thread is not implemented" }
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

`cargo test -p bingo-mods -p bingo-plugin-host`:
```text
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

`cargo clippy -p bingo-mods -p bingo-plugin-host --all-targets --locked -- -D warnings`:
```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.21s
```
