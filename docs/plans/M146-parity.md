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
  `src/{lib,imports,strip,load,mount,module,chain}.rs`, `src/dollar/{table,tests,callback,runtime}.rs`, `src/testing/environment.rs`,
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

- [x] Each committed example validates/tests; new band/pane contracts seen red first.
- [x] Original ports validate/test; diff evidence proves package-specifier-only changes.
- [x] mods_parity passes including Node, required on Linux CI.
- [x] tmux captures: denial, status, band, tool row, inline pane at both widths.
- [x] Every §2/3 row has Status; awk prints 0 rows without it.
- [x] rg locates each §8 gap in its owner's Non-goals; native gates pass; Windows C-toolchain checks deferred to CI under protocol §7.

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
- Compatibility fixes preserve existing doors and Decisions. Windows CI owns
  the known third-party C-toolchain deferral; no Rust error was reported.

## Verified (2026-10-14)

- `cargo test -p bingo-mods --lib validate::tests --locked`: data reads red→6 pass; erased imports red→8 pass; as-const object keys red→1 pass; static matchers red→1 pass.
- `cargo test -p bingo-core --lib a_json_deny_only_answer_never_runs_the_tool --locked`: red (1 execution); `cargo test -p bingo-core --lib chain::tests --locked`: 8 pass, deny executes zero at every tier.
- `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk catalogue:: --locked`: pass; dollar/schema and plugin/event generators run with the same flag.
- `cargo test -p bingo-plugin-rpc --lib --locked`: 131 pass; deny-only wire contract red first; authorized baseline regenerated, freeze passes.
- `node --test sdk/typescript/src/*.test.ts`: 30 pass; status(undefined), atoms, callback-free commands and synchronous resolve seen red first.
- `cargo test -p bingo --test plugin_kit clock_and_environment_mocks_keep_grants_and_hook_budgets --locked`: numeric clock red→1 pass.
- `cargo test -p bingo-mods --test engine --locked`: 5 pass, 1 manual RSS ignored; unawaited dollar call regression red first.
- Original shell parity test: wall-deadline failure repeats alone; shared runtime/job waker fixes nested continuation stall, all 4 tool-call cases pass.
- `cargo test -p bingo --test mods_parity --locked -- --nocapture`: 3 pass; own tool-call 5, band 3, pane 2 cases pass; both transports preserve .env, trim shell, clear failure status.
- `scripts/port_examples.sh <supplied-originals> /tmp/m146-ports-verified`: all validate/test exit 0; tool-call 4, band 2, pane 1 behavior cases pass.
- Source-free `diff -U0` counts: tool-call.ts -1/+1; band.tsx -2/+2; band-state.d.ts -1/+1; pane.tsx -2/+2; pane-state.d.ts -1/+1; package specifiers only, originals unchanged.
- Original Node port `bingo --print --output-format json --plugin-dir <temporary/tool-call>`: exit 0, NDJSON valid; .env unchanged, Bash trimmed, failure status then null (isolated rewrite grant).
- `tmux -L bingo-m146-acceptance-1048 capture-pane -p -t m146`: 160×48 and 100×40: `tool-call: .env is protected here.` / `Bash(  exit 7  )` / `failed:   exit 7` / `Last turn: 0s, 1 tool calls` / `[ 1 Hide ]` / `Tool calls` / `done Bash`.
- Same owned tmux drive: before `/tool-calls`, `1 pane waiting`; after it, pane inline at both widths; ctrl-c exit and owned socket cleaned. No captures/logs committed.
- Mechanical fixture outside original Files: `crates/bingo/tests/support/nouns.rs` expects schema-declared plugin.version; `tests/plugin_kit.rs` helper case count 3→4; no theme allowlist or discipline 4c changes.
- `cargo fmt --all -- --check`: exit 0.
- `cargo check --workspace --all-targets --locked`: exit 0 (final rerun 1.22s).
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: large-enum lint fixed by boxing CommandReply outcome; final rerun exit 0 (1.58s), wire unchanged.
- `cargo test --workspace --locked`: stale helper count red→1 focused pass; full rerun exit 0, 160 groups / 5383 passed / 0 failed / 5 ignored.
- `scripts/check_discipline.sh`: exit 0, `discipline ok`; advisory function/history lengths only.
- `scripts/budget.sh`: exit 0, `budget ok`; dependencies 411/411, core check 0s/20s, relink 0; debug 10 GB (5 GB soft warning), 140 test binaries; no new dependencies.
- `cargo deny check`: exit 0, `advisories ok, bans ok, licenses ok, sources ok`.
- `scripts/tui-smoke.sh`: exit 0, `tui-smoke ok`; band hidden, Edit denied, Bash rewritten, two pane rows, Escape closed.
- `cargo check -p bingo-sdk -p bingo-core -p bingo-mods -p bingo-plugin-rpc -p bingo --all-targets --target x86_64-pc-windows-msvc --locked`: exit 101; known C deferral: rquickjs-sys 0.14.0 MSVC flags, aws-lc-sys 0.44.0 missing stdlib.h/windows.h; Rust E-code errors: 0; Windows CI owns completion.
- `awk` over §2/3 table rows: `matrix rows=48; missing Status=0`; `rg` in owner Non-goals: M136 prompt.edit/attachment/classic, M138 tool.check/prompt.compose/session.repo, M139 audio, M140 Client/blit/vscode/mobile/surface APIs, M141 surface APIs, M151 unasked/isPlaced (6 owner plans).
