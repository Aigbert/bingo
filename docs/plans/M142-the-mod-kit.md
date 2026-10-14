# M142 — The mod kit
## Goal
ADR-0078 §8: AST validation, isolated in-process tests against the real host,
and live author types on module load. Replace M133 scenarios with tests.
Re-verified at `ba70fdf6` (2026-10-14). Landed dependencies: M137
(`c1d95e8c`), M138 (`5581b978`), M139 (`686ad289`), M140 (`ba70fdf6`),
M147 (`6e3b1b03`), M148 (`fddb62f3`), M149 (`b2bee6f4`), M150
(`a2f77d96`); M133/M136 are present. Resumed after the authorized merge
of dev `a88a5363` (M141 and M151) in `2b422229` on 2026-10-14.
## Bricks, in build order
1. **Contracts first.** `Finding { file, line, column, rule, severity, text }`;
   rules: `on-literal`, `dollar-static`, `state-key`, `env-name`, `uses`,
   `state-contract`, `prop-subset`, `import`. Each has an `expected.json`
   fixture; counter and the examples are clean. `Report { file, test,
   outcome: Pass | Fail { message, at } | Skip }` emits text or NDJSON.
2. **AST reader.** Reuse M137's pinned oxc across the resolved module graph.
   `hooks.rs` resolves literal/const patterns and literal matchers;
   `dollar.rs` refuses computed access, aliases and stored members, permits
   passing `$`; `keys.rs` requires literal state/store/env/atom keys;
   `uses.rs` checks unsafe nouns against declared uses and warns on unused
   uses; `contract.rs` checks state keys against `types/index.d.ts`'s
   `PluginState` (missing file warns); `props.rs` warns outside the SDK's
   drawn JSX subset. List hooked events, calls, keys and uses.
3. **Validate CLI.** Module manifests use the AST reader without spawning;
   process manifests keep M133's handshake. Emit `file:line:col rule text`
   and the listing, exit 0 or 1.
4. **Test runtime.** Embed `bingo/testing` only for `plugins test`, with
   `test`, `describe`, `expect` (`toBe`, `toEqual`, `toMatchObject`,
   `toContain`, `toThrow`, `rejects`, `not`), `mock.clock.set/advance`,
   `mock.store.seed/read`, `mock.env`, `mock.session({ model, cwd })`,
   and immediate `reload()`. Test sessions can set ordinary permission rules.
   Use real grants and isolated state; tokio
   paused time drives timers, and the mock clock supplies wall timestamps.
5. **Mount.** `$.ui.mount(component | { pane: id }, props?, size?)` pulls
   `HostApi::render` as terminal (default 80x24). Handle: `tree`, `drawn`,
   `find`, `press`, `input`, `select`, `advance`, `resize`; dispatch minted
   actions via `Input::Action` and rerender on `Invalidated`. The bin
   injects TUI `drawn(&View, width) -> Vec<String>`; no plugin imports a
   surface. Engine nodes draw as `⟨Component⟩`; no style snapshot.
6. **Test CLI.** The bin builds the real host with FakeProvider, discovers
   `**/*.test.{ts,tsx}` outside generated/vendor trees, and runs each file
   in a fresh runtime/session. Hook budgets name hung tests. Exit 1 names
   the failing test and `file:line`; `--json` emits one report per line.
7. **Replace scenarios.** ts-status gains `hooks/register.test.ts` while
   retaining its node transport. Delete scenario runner/schema/assertion
   helpers and `schema/plugin-scenario.json`; migrate plugin-kit cases.
8. **Types on load.** Use `bingo-schema-ts` and write only changed bytes:
   `.bingo/types/bingo/` exports Register, live kernel/op/noun event types,
   Dollar, the drawn elements and M141's atom/derive/read/update/memberOf;
   also emit testing types, `bingo-tools` input schemas, `.bingo/tsconfig.json`
   (h/Fragment JSX factories and paths) and `.bingo/.gitignore` (`*`).
   Create a root extending config only if absent. Write failure is a notice.
9. **M141 last.** Add helper tests in `sdk/typescript/src/state.test.ts`
   (`ifVersion` retry, derive memo, memberOf) and tool-call/band/pane tests.
## Files
- `crates/bingo-mods/src/validate/`, `{testing,mount,types}/` or `.rs`,
  `js/testing.ts`, `tests/fixtures/validate/` and kit contract tests (new)
- `crates/bingo-mods/src/{lib,load,strip,engine,thread,module,manager,plugin}.rs`,
  `thread.js`, `manager/reload.rs`, `dollar/runtime.rs`: load/test/type integration
- `crates/bingo-mods/Cargo.toml`: promote existing `bingo-schema-ts` to
  a normal library dependency for types on load; enable tokio test-util
  for the isolated test clock (no new external crate)
- `crates/bingo/src/plugins/{mod,switch,validate,test}.rs`, `test/`, `main.rs`;
  `{scenario,schema,expect}.rs` and `schema/plugin-scenario.json` deleted
- `crates/bingo-surface-tui/src/lib.rs` (`pub fn drawn`)
- `crates/bingo-sdk/src/host.rs`, `crates/bingo-core/src/host/catalog.rs`:
  `CatalogKind::Events` reads kernel entries and registered faces through
  the existing `HostApi::catalog` door; no new host verb or plugin import.
- `crates/bingo-plugin-rpc/src/schema.rs`: regenerate the scoped baseline.
- ADR-0078 acceptance and dated verification notes in its amended ADRs.
- Generated `schema/{rpc,plugin,plugin-2,events,dollar}.json` and
  `sdk/typescript/src/{wire,events,dollar-types}.ts`; ADR-0075 dated note.
- `sdk/typescript/src/state.test.ts`, `package.json`, `test/testing.ts`: run
  the same helper cases with bingo/testing in QuickJS and a Node assertion adapter.
- `examples/plugins/ts-status/` (`hooks/register.test.ts` new, `scenarios/` deleted);
  `examples/plugins/{tool-call,band,pane}/` (one `*.test.ts` each; named state contracts)
- `crates/bingo/tests/plugin_kit.rs`; `tests/fixtures/mods/counter/types/index.d.ts`
  adds the state contract required for the existing counter to validate cleanly.
## Exit criteria
- [x] Validator fixtures, counter and ts-status pass; invalid on-literal exits 1.
- [x] ts-status acceptance, wrong expect/source position, NDJSON and mount actions pass.
- [x] Second load preserves generated-type mtimes; pane passes local tsc.
- [x] Old scenario references are absent; native gates and isolated TUI smoke pass.
- [x] M141 band/pane/tool-call validate and pass their new tests.
- [x] Pane test runs two fake tool calls, presses Escape and asserts drawn().
- [x] `bingo plugins test sdk/typescript` covers helper retry, memoization and membership.
- [x] `tsc --noEmit -p examples/plugins/pane` passes.
- [x] Windows gate: SDK/core pass; known C-header deferrals recorded for Windows CI.
## Non-goals
- Hot reload itself (ADR-0078 §7, M137); `bingo plugins new`; publishing.
- Snapshot files (`toMatchSnapshot`), coverage, watch mode, npm imports.
- `mount` drawing the desktop table; `tsc` in CI (it is a dependency).
- Tests for a process plugin in another language: it tests itself with
  its own tools, and the scenarios are not kept for it.
## Risks
- Validation is a lint, not a sandbox; runtime grants remain authoritative.
  Passing `$` is allowed; storing it or a noun/method alias is rejected.
- `drawn()` drops style; inspect tree() for style. Mock clock advances tokio
  time and wall timestamps; hook/host-operation budgets use real elapsed time.
- Types write into `.bingo/`, ignored locally; existing author tsconfig is preserved.
- Scoped protocol-2 amendments use a dated ADR-0075 note and BINGO_UPDATE_SCHEMA=1.
  No SDK/kernel tool names, discipline 4c changes, live or credential tests.
- Live type schemas need enumeration: additive `CatalogKind::Events` on
  HostApi::catalog reads kernel entries and registered faces, without a new verb.
- Resumed 2026-10-14 after M141/M151 merge. SDK is a library, not a mod:
  a manifestless package runs files importing bingo/testing against a disposable
  empty mod; Node-only tests are reported as skipped and retain their npm runner.
- Mount uses component request identities, follows resize invalidation, and sends
  the same pane Escape close action as the TUI; examples need no behavior changes.

## Verified (2026-10-14)
- `cargo test -p bingo-mods fixtures_pin_each_rule_and_its_source_location --locked` → red before implementation: empty findings, expected on-literal finding.
- `cargo test -p bingo-mods --test testing testing_module_runs_matchers_and_reports_each_outcome --locked` → red: test runtime is not installed; implemented suite now passes.
- `cargo test -p bingo-mods interaction_keeps_the_minted_action_and_sets_only_its_value --locked` → red: no matching handler; implemented test passes.
- `cargo test -p bingo --test plugin_kit module_tests_report_ndjson_and_use_the_real_host --locked` → red: unknown --json, exit 2; implemented test passes.
- `cargo test -p bingo-surface-tui drawn_uses_terminal_layout_and_marks_nested_engine_nodes --locked` → red: empty drawing; implemented test passes.
- `cargo test -p bingo-core event_catalogue_includes_kernel_and_late_service_schemas --locked` → red: unknown events catalogue kind; implemented test passes.
- `cargo test -p bingo-mods writes_event_and_tool_types_once_and_keeps_the_authors_tsconfig --locked` → red: missing generated file; implemented test passes.
- `cargo test -p bingo-mods passing_a_noun_or_method --locked` → red: 0 findings, expected 2; fixed and passed.
- `cargo test -p bingo-mods validate:: --locked` → 5 passed, including all eight rule fixtures, counter and all four examples.
- `cargo test -p bingo --test plugin_kit --locked` → 20 passed; all examples, SDK helpers, NDJSON, wrong expect, actions, mocks, grants and equal second-load mtimes.
- `target/debug/bingo plugins validate examples/plugins/pane` → exit 0; session.start/tool.call/ui.render; command.register/state.get/state.set/ui.open/ui.resolve; state:calls; uses command/state/ui.
- `target/debug/bingo plugins validate crates/bingo-mods/tests/fixtures/validate/on-literal` → exit 1; register.tsx:1:31 on-literal.
- `node /Users/yexrob/.npm/_npx/397d2194ba975aaa/node_modules/typescript/bin/tsc --noEmit -p examples/plugins/pane` → exit 0.
- `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk the_committed_catalogue_is_generated --locked` → passed; regenerated catalogue.
- `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc the_committed_schema_is_this_document --locked` → passed; regenerated protocol schemas.
- `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc '_ts::' --locked` → 2 passed; regenerated TypeScript.
- `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-mods schema_and_types_are_generated_from_the_same_rows --locked` → passed; regenerated dollar files.
- `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-surface-rpc schema:: --locked` → 4 passed; regenerated RPC schema.
- `cargo fmt --all -- --check` → exit 0.
- `cargo check --workspace --all-targets --locked` → exit 0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings` → exit 0 on the merged result.
- `cargo test --workspace --locked` → exit 0; 5371 passed, 0 failed, 5 existing manual/stress/storyboard tests ignored on the merged result.
- `scripts/check_discipline.sh` → exit 0; discipline ok; kernel names no tool; no new test-driver length warnings.
- `scripts/budget.sh` → exit 0; 411/411 dependencies, core 1s, relink 0; target/debug 12 GB soft-limit warning.
- `cargo deny check` → exit 0; advisories, bans, licenses and sources ok.
- `CC_x86_64_pc_windows_msvc='clang --driver-mode=cl' cargo check -p bingo-sdk -p bingo-core -p bingo-mods -p bingo-plugin-rpc -p bingo-surface-tui -p bingo --all-targets --locked --target x86_64-pc-windows-msvc` → exit 101; rquickjs-sys stdlib.h and aws-lc-sys windows.h absent; mods/plugin-rpc/TUI/bin deferred to Windows CI, no Rust error reported.
- `cargo check -p bingo-sdk -p bingo-core --all-targets --locked --target x86_64-pc-windows-msvc` → exit 0 on the merged result.
- `scripts/tui-smoke.sh` → exit 0; tui-smoke ok, including all M141 examples; private tmux socket/temp home cleaned.
- `rg -n 'plugin-scenario|scenario::' crates schema` → exit 1, no matches.
- `git merge --no-ff dev -m 'merge: dev into M142'` → 2b422229 merges a88a5363; runtime conflicts combined and generated files regenerated.
- `cargo test -p bingo --test plugin_kit sdk_helpers_run_without_a_mod_manifest --locked` → red before integration: no module manifest; now passes.
- `cargo test -p bingo --test plugin_kit component_examples_run_with_the_author_kit --locked` → red: unanswered permission interaction; explicit test-session rules now pass without widening mod grants.
- `target/debug/bingo plugins test examples/plugins/tool-call` → exit 0; denial, rewrite, count and ToolUse wrapper pass.
- `target/debug/bingo plugins test examples/plugins/band` → exit 0; survey fallback, press and reload pass.
- `target/debug/bingo plugins test examples/plugins/pane` → exit 0; two tool calls, exact drawn text, reload, resize and Escape pass.
- `target/debug/bingo plugins test sdk/typescript` → exit 0; 3 helper tests pass; 5 Node-only suites explicitly skipped.
- `node --test src/*.test.ts` (sdk/typescript) → exit 0; all 24 tests pass, no skips.
