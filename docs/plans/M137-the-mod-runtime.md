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

- [x] Shared library contracts and all 49 existing RPC assertions pass.
- [x] Mod tests cover strip file:line, all six fixtures and one notice each;
      thrower's catch sees kind=throw.
- [x] Esc during loop at tool.check ends Interrupted within M116's bound.
- [x] Project/model, registration denial and runtime-specific grant tests;
      the existing process CLI default remains tighten.
- [x] Mid-turn edit reloads after turn.complete; store survives; no sleeps.
- [x] plugin_kit runs ts-status in process and node (M133's node skip rule).
- [x] Local serial gates, before/after budget and all six MSVC attempts.
- [ ] Windows CI: RPC/bin await aws-lc-sys headers; mods awaits rquickjs-sys
      headers. SDK/core/shared-host MSVC passed; deferred per step 7.

## Non-goals

Dollar beyond M133 (M138/139), elements (M140), module validate/test (M142),
npm imports, mod-owned threads/timers, shared JS runtimes, Client modules,
file-watcher dependencies, watching non-explicit directories; no live tests.

## Risks

- QuickJS parallel/exotic-class issue #764: natives are plain functions and
  objects; host work is asynchronous and bounded. Windows C-header checks
  defer to CI; installed Clang needs --driver-mode=cl for QuickJS's MSVC flags.
- Pin weekly oxc; 50 idle mods measured 29,792 KiB RSS (3,232 KiB baseline).
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
scripts/budget.sh # before extraction
dependencies (unique, normal): 355 (max  355)
warm cargo check -p bingo-core: 0s (max  20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
budget ok
```

```text
cargo test -p bingo-mods --test engine fifty_idle_modules_rss --locked -- --ignored --nocapture
idle modules: 50; RSS KiB before=3232; after=29792
Finished `test` profile [unoptimized + debuginfo] target(s) in 0.77s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.20s
```

```text
cargo fmt --all -- --check
exit 0 (no output)
```

```text
cargo check --workspace --all-targets --locked
Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.85s
```

```text
cargo clippy --workspace --all-targets --locked -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.29s
```

```text
cargo test --workspace --locked
Finished `test` profile [unoptimized + debuginfo] target(s) in 11.53s
test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.26s
test plugins::an_external_plugins_tier_is_listed_from_user_settings_only ... ok
test schedule::ownership::doctor_fix_preserves_held_and_released_modern_lease_files ... ok
test result: ok. 259 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 22.07s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test module_acceptance_runs_outside_the_workspace ... ok
test typescript_acceptance_runs_outside_the_workspace ... ok
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.32s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.34s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.89s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.59s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.86s
test result: ok. 158 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 103 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.29s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 120 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.81s
test host::tests::controls::a_new_session_preserves_the_answer_without_editing ... ok
test host::tests::controls::a_new_session_preserves_the_previous_streamed_answer ... ok
test result: ok. 104 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test tests::releasing_preserves_the_published_marker_and_inode ... ok
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test a_bare_lock_excludes_until_drop_and_preserves_the_file ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test prompt::tests::overflow_shrinks_only_complete_exchanges_and_preserves_purpose ... ok
test result: ok. 99 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s
test settings::tests::an_empty_picture_object_preserves_the_lower_leaf ... ok
test turn::tests::attribution::a_peer_at_the_barrier_preserves_the_previous_request_prefix ... ok
test result: ok. 498 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.93s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 82 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 50 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 104 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.56s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.87s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test dial::tests::catalogue_order_preserves_duplicate_definitions_and_precedence ... ok
test result: ok. 90 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.08s
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.23s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test an_escaper_load_failure_is_visible_once ... ok
test a_reentrant_call_skips_only_its_calling_hook_and_says_why_once ... ok
test throw_loop_and_hog_fail_open_with_one_named_notice ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 4 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.06s
test registration_can_deny_a_module_with_a_named_reason ... ok
test a_project_model_call_requires_the_users_grant ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test esc_during_a_tool_check_loop_ends_interrupted_within_the_grace ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test reload_keeps_store_and_restarts_every_open_session ... ok
test a_mid_turn_edit_waits_for_turn_complete ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 103 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.79s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 68 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.29s
test grant::tests::runtime_origin_and_user_trust_bound_grants ... ok
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test both_and_neither_name_the_plugin_and_launch_forms ... ok
test entry_only_and_module_only_round_trip ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test host::tests::host_operations_require_the_catalogues_granted_nouns ... ok
test wire::tests::a_declared_compaction_failure_preserves_native_usage_in_its_tagged_fixture ... ok
test result: ok. 128 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.34s
test context::a_remote_declared_compaction_failure_preserves_each_observed_usage_counter_once ... ok
test host::model_completion_reaches_the_hosts_default_refusal ... ok
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 155 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 86 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 230 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 115 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.03s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test compactor::tests::a_failed_summary_preserves_observed_usage_on_the_wire ... ok
test store::checkpoint::tests::checkpoint_codec_preserves_opaque_item_bytes ... ok
test store::checkpoint::tests::checkpoint_fixture_preserves_model_and_client_orders ... ok
test tree::tests::gateway_removal_returns_the_subtree_and_preserves_the_root ... ok
test result: ok. 135 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.22s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 86 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test checkpoint::tests::checkpoint_resume_reads_only_tail_and_preserves_the_fold ... ok
test result: ok. 67 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.63s
test result: ok. 102 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 41 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 8.65s
test result: ok. 1135 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 2.02s
test result: ok. 76 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 149 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.69s
test grep::tests::rough_edges_grep_display_preserves_content_and_shortens_only_paths ... ok
test result: ok. 83 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.64s
test result: ok. 92 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
test result: ok. 47 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

```text
cargo test -p bingo-provider-acp --test bridge token --locked -- --test-threads=1
Finished `test` profile [unoptimized + debuginfo] target(s) in 9.79s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.01s
```

```text
scripts/check_discipline.sh
dependency direction ok
shared service declarations ok
cohesion ok
test host and wall-clock advisory checks complete
catalogue entry declarations ok
discipline ok
```

```text
scripts/budget.sh
dependencies (unique, normal): 410 (max  410)
warm cargo check -p bingo-core: 0s (max  20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
target/debug: 11 GB (soft max  5)
warn: target/debug exceeds the soft limit
test binaries: 190
budget ok
```

```text
cargo deny check
advisories ok, bans ok, licenses ok, sources ok
```

```text
cargo check -p bingo-sdk --all-targets --target x86_64-pc-windows-msvc
Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.12s
```

```text
cargo check -p bingo-core --all-targets --target x86_64-pc-windows-msvc
Finished `dev` profile [unoptimized + debuginfo] target(s) in 11.12s
```

```text
cargo check -p bingo-plugin-host --all-targets --target x86_64-pc-windows-msvc
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.79s
```

```text
cargo check -p bingo-plugin-rpc --all-targets --target x86_64-pc-windows-msvc
warning: aws-lc-sys@0.44.0: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/tests/compiler_features_tests/c11.c:7:10: fatal error: 'stdlib.h' file not found
warning: aws-lc-sys@0.44.0: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
error: failed to run custom build command for `aws-lc-sys v0.44.0`
exit 101; deferred to Windows CI (protocol step 7).
```

```text
cargo check -p bingo-mods --all-targets --target x86_64-pc-windows-msvc
warning: rquickjs-sys@0.14.0: clang: error: no such file or directory: '/DWIN32_LEAN_AND_MEAN'
warning: rquickjs-sys@0.14.0: clang: error: no such file or directory: '/std:c11'
warning: rquickjs-sys@0.14.0: clang: error: no such file or directory: '/experimental:c11atomics'
error: failed to run custom build command for `rquickjs-sys v0.14.0`
exit 101
```

```text
CC_x86_64_pc_windows_msvc='clang --driver-mode=cl' cargo check -p bingo-mods --all-targets --target x86_64-pc-windows-msvc
warning: rquickjs-sys@0.14.0: /Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m137/target/x86_64-pc-windows-msvc/debug/build/rquickjs-sys-5ef90e9da9c37e1f/out/libregexp.c(24,10): fatal error: 'stdlib.h' file not found
error: failed to run custom build command for `rquickjs-sys v0.14.0`
exit 101; deferred to Windows CI (protocol step 7).
```

```text
CC_x86_64_pc_windows_msvc='clang --driver-mode=cl' cargo check -p bingo --all-targets --target x86_64-pc-windows-msvc
warning: aws-lc-sys@0.44.0: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/tests/compiler_features_tests/stdalign_check.c(10,10): fatal error: 'stdlib.h' file not found
warning: aws-lc-sys@0.44.0: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h(49,10): fatal error: 'windows.h' file not found
error: failed to run custom build command for `aws-lc-sys v0.44.0`
exit 101; deferred to Windows CI (protocol step 7).
```

```text
node --test --test-reporter=tap sdk/typescript/src/*.test.ts
# tests 13
# pass 13
# fail 0
# skipped 0
```
