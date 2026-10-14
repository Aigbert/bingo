# M146 — Parity: Claude Code's examples run in the TUI

## Goal

The Phase 7 acceptance test: Claude Code 2.1.296's tool-call, band and
pane mods run in process in the TUI with only import-specifier and tool
name/input changes. Committed examples remain bingo's independent rewrites.
Every parity matrix row has evidence or an owned gap. ADRs: 0078, 0079, rejected 0080, and 0081 §7. No desktop work (M143–M145).

## Bricks, in build order

1. Check the committed examples are rewrites, not proprietary copies.
   Retain their existing tests; never commit original source without terms.
2. Add missing `bingo/testing` behavior cases, seen failing before code:
   - tool-call: deny Edit of `.env` and `config/.env.local`, naming the
     plugin; allow `.envrc`; trim Bash; failure status clears on success;
     a throwing hook's catch denies only before next was called.
   - band: count per turn; mock.clock past 120 s toasts once; mount
     AbovePrompt; button hides the summary, next prompt resets the count.
   - pane: session.start registers/opens tool-calls; list every call,
     dim completed calls, and bound the list by viewport.rows.
3. `scripts/port_examples.sh <originals> <temporary-output>` creates
   plugin.json, hooks/<original>, and types/index.d.ts from *-state.d.ts.
   Only claude-code → bingo is replaced; Edit/file_path and Bash/command
   already agree. Run validate and committed parity behavior tests for all three ports.
   Report failures to their owners (0078 §1/8, 0079 §1/4/5); never patch
   the originals or a Decision. Paste source-free diff evidence.
4. Drive a detached tmux session on an owned `-L` socket with isolated
   home/cwd and BINGO_FAKE_SCRIPT: protected edit, padded Bash, one failure.
   Open the waiting pane by the person's command, then capture denial,
   status, band, tool row and the pane inline at 160 and 100 columns (0081 §7).
5. Run tool-call under Node through the SDK with equivalent results.
   plugins test requires a module manifest; use binary black-box coverage
   for the process transport. Node type stripping does not run JSX ports.
6. Add Status to parity §2/3 (`done — evidence`, `gap — non-goal in M<n>`,
   or `n/a`), evidence lines to §4–6, and missing §8 owner Non-goals in a
   docs(plans) commit. Run the sequential protocol gates and record results.

## Files

- `examples/plugins/{tool-call,band,pane}/`: rewrites and behavior tests.
- `scripts/port_examples.sh`: temporary layout and validation/test runner.
- `crates/bingo/tests/mods_parity.rs`: CLI and Node parity coverage.
- `docs/design/mods-parity-2026-10.md`: matrix evidence.
- Owner plans, only to record missing §8 Non-goals (brick 6).

2026-10-14: delegate widened scope to every port compatibility gap; no new kernel door/noun or ADR Decision change:
- `crates/bingo-mods/src/validate/{dollar,source,tests}.rs`,
  `src/{lib,imports,strip,load,mount,module,chain}.rs`, `src/dollar/{table,tests,callback}.rs`,
  `tests/{strip,engine,testing,plugin}.rs`: schema-derived data reads and erased imports.
- `schema/dollar.json`, `sdk/typescript/src/{dollar,dollar-types,plugin}.ts`,
  `crates/bingo-plugin-rpc/src/{wire,bridge,validate}.rs`, generated plugin schemas/wire,
  `docs/adr/0075-plugin-protocol-2.md`: plugin metadata in both transports.
- `crates/bingo/src/plugins/test/{driver,driver/mock}.rs`, `crates/bingo-mods/js/testing.ts`:
  test metadata, asynchronous startup and clock test controls; `tests/plugin_kit.rs`, `tests/support/nouns.rs`.
- `crates/bingo-mods/src/{thread.rs,thread.js,load.rs}` and SDK `{ui,plugin}.ts`:
  settle synchronous element references through the intercepted table.
- SDK `src/{ui,state,nouns,dollar}.test.ts`, `schema/events.json`, generated SDK events;
  `crates/bingo-plugin-rpc/src/{schema,wire}.rs`: port-required event contracts.
- `crates/bingo-sdk/src/{catalogue,intercept}*`, `crates/bingo-core/src/chain/`:
  deny-only results and core regressions; generated catalogue/protocol types.
- `crates/bingo-mods/src/validate/{hooks,keys}.rs`, SDK state/UI/command/clock
  helpers and their existing tests; shared dollar/runtime/testing paths:
  static matchers, object atom keys, status clear, and port-required contracts.

## Exit criteria

- [ ] Each committed example validates/tests; new band/pane contracts fail.
- [ ] Original ports validate/test; diff evidence proves import/tool-map only.
- [ ] mods_parity passes including Node, required on Linux CI; incomplete.
- [ ] tmux captures: denial, status, band, tool row, inline pane at both widths.
- [ ] Every §2/3 row has Status; awk prints 0 rows without it.
- [ ] rg locates each §8 gap in its owner's Non-goals; all gates pass.

## Non-goals

- Client modules and ui.blit (M140/M141): no surface runtime/state copy.
- prompt.edit per keystroke (M136): surface composer/IME ownership.
- prompt.attachment/mention (M136): surfaces resolve @ before prompt.submit.
- ui.scroll/focus/selection/copy and open.isPlaced (M140/M141): surface
  ownership, no host verb toward a surface (ADR-0013 §4).
- tool.check without running, prompt.compose as a read, session.repo
  (M138): each needs a new kernel door.
- audio (M139): no player or audio stack; classic.* (M136): hooks-shell.
- Desktop/vscode/mobile surfaces: outside this tree (ADR-0080 rejected).
- Unasked/width-rule pane docking: person's opening and declared placement
  govern it (ADR-0081 §7, M151). telemetry/attribution.text: not applicable.
- Committing originals, JSX under Node, or type-checking Claude-only props
  such as AbovePrompt.hasSurvey (undefined means no survey here).

## Re-verified / Risks (2026-10-14)

- Base `3b10b522`; M136–M142 and M147–M151 landed; M143–M145 dropped.
  All originals read; tool inputs agree; Linux CI requires Node 24.17.0. Examples remain rewrites.
- Both authorized blockers fixed: schema-derived data reads preserve alias refusals; erased imports share the runtime boundary and never load d.ts.
  mount.rs folds Views, not sources; Engine/Runner share load.rs/strip.rs.
- Delegate authorized all exposed compatibility gaps, each in its own fix commit
  after a failing test. New kernel doors/nouns or ADR Decision changes still stop.
- Compatibility fixes preserve existing doors and Decisions; remaining work is
  startup test synchronization, command event vocabulary and parity acceptance.

## Verified (2026-10-14, in progress)

- `cargo test -p bingo-mods --lib validate::tests --locked`: data reads red, then 6 pass; erased imports red (directory load), then 8 pass.
- `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-mods --lib schema_and_types_are_generated_from_the_same_rows --locked`: 1 pass.
- `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc --lib --locked`: 130 pass.
- `cargo test -p bingo-mods --locked`: prior run 75 pass, 1 manual RSS ignored; mechanical identity fixture adds version.
- `cargo test -p bingo --test mods_parity port_layout_changes_only_the_package_specifier --locked`: red, then 1 pass; only parity tests copied to ports.
- Disposable-copy `bingo plugins test`: own tool-call 4 pass; band/pane new contracts red, original contracts pass.
- `scripts/port_examples.sh <supplied-originals> /tmp/m146-ports-current`: exit 1, temporary ports only; deny Edit incorrectly completed, status failed to clear, band/pane blocked.
- Source-free diff evidence: tool-call.ts -1/+1; band.tsx -2/+2; band-state.d.ts -1/+1; pane.tsx -2/+2; pane-state.d.ts -1/+1; package specifiers only, source unchanged, no tool mapping needed.
- `cargo test -p bingo-core --lib a_json_deny_only_answer_never_runs_the_tool --locked`: red, execution count 1; `cargo test -p bingo-core --lib chain::tests --locked`: 8 pass, zero executions for deny at all tiers.
- `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk catalogue:: --locked`: pass; `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc --lib events_ts:: --locked`: 1 pass.
- `HOME=<isolated-home> target/debug/bingo plugins test /tmp/m146-ports-current/tool-call`: denial and envrc pass; status still red before its fix.
- `node --test --test-name-pattern='undefined status' sdk/typescript/src/ui.test.ts`: red; `node --test sdk/typescript/src/ui.test.ts`: 5 pass after fix.
- `cargo test -p bingo-mods --lib module_constant_matchers --locked`: red, then 1 pass.
- `cargo fmt --all -- --check`: pending final gate.
- `cargo check --workspace --all-targets --locked`: pending.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: pending.
- `cargo test --workspace --locked`: pending.
- `scripts/check_discipline.sh`: pending.
- `scripts/budget.sh`: pending; no new dependencies.
- `cargo deny check`: pending.
- `scripts/tui-smoke.sh`: pending.
- `cargo check -p bingo -p bingo-mods -p bingo-plugin-rpc --all-targets --target x86_64-pc-windows-msvc`: pending.
- `cargo test -p bingo-mods --lib object_atom_keys --locked`: assertion-wrapped key red, then 1 pass.
- Matrix awk/rg, Node parity and TUI captures: pending; no logs/captures committed.
