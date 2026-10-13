# M132 — Plugin protocol 2

## Goal

Implement ADR-0075: freeze protocol 2, negotiate host verbs and two-phase
interceptors, bill side completions through the session, retain process-tree
ownership, and reload explicit plugin directories. M133 owns the author kit.

## Bricks, in build order

1. **Contracts first.** `PROTOCOL = 2`; generated `schema/plugin.json` and
   frozen `plugin-2.json`. Reject removed/retyped methods, fields and capabilities;
   permit additions. Unknown requests answer `METHOD_NOT_FOUND`; refuse protocol 1.
2. **Capabilities.** Initialize with `pluginName`, `pluginRoot`, host capabilities
   and the plugin's answer. Pure set intersection; refuse unnegotiated verbs.
3. **Reverse table.** `host.rs` translates one verb per handler; `service/call`
   keeps ADR-0031 behavior and its existing tests.
4. **Host verbs.** Place through `signal`, persist through `extend`, deliver with
   origin `plugin:<name>`. Delivery queues input and never waits for its turn.
5. **State.** Pure version CAS, then the ADR-0072 shelf under
   `<data_dir>/plugins/<name>/state/{global,<session>}`. One write per set;
   conflicts return the held version; `SessionRemoved` removes session values.
6. **Completion.** Default SDK refusal is `Unsupported`. Core resolves the session
   model and provider; additive `UsageCharged` bills the live turn or the session.
   The actor's single tally supplies `TurnCompleted.usage`; rounds/context stay put.
7. **Contributions.** Replace whole tool/command lists on change notifications;
   preserve notifications arriving at the handshake boundary.
8. **Links.** `intercept/<point>` before/after, cancel on drop, `HOOK` per phase
   excluding the continuation. Dynamic `InterceptorSource`, process identity and
   host trust tiers. Retire `hook/decide`; retain observations. ADR-0078 names
   the stop point `turn.complete`.
9. **Process tree.** Retain M128's `bingo-process::Tree` and `bingo-jsonrpc::Peer`;
   rerun grandchild cleanup. Fixtures use the supported shell and executable suffix.
10. **Reload.** Repeatable `--plugin-dir` contributes `pluginDirs`, the last
    discovery layer. Pure path/length/mtime comparison; poll once per second on
    tokio time. Stop, spawn, handshake, refresh contributions and notify once;
    preserve state and report changed providers/services as requiring restart.

## Files

- `schema/{plugin,plugin-2,rpc}.json` (`plugin-2.json` new).
- `crates/bingo-plugin-rpc/src/{wire,connection,bridge,hook,source,manager,discovery,config,manifest,schema,testing,lib}.rs`
- `crates/bingo-plugin-rpc/src/{host,state,intercept,reload}.rs` (new).
- `crates/bingo-plugin-rpc/examples/stub_plugin/{main,hooks}.rs`, `tests/plugin/*.rs`.
- `crates/bingo-sdk/src/{host,error,intercept,plugin,event,state,lib}.rs`,
  `src/event/raw.rs`, `src/testing/{api,calls}.rs` as needed,
  `tests/{record,intercept}.rs`, `fixtures/record/event/usageCharged.json`.
- `crates/bingo-core/src/{host,plugins,session,turn}.rs`, `host/{registry,complete}.rs`,
  `session/{mailbox,running,complete,gate,inputs,invoke}.rs`, `turn/{late,stream}.rs`,
  `chain/*.rs` and their tests; turn tests read emitted usage, not a duplicate tally.
- `crates/bingo/src/{main,plugins}.rs`, `crates/bingo/tests/plugin_rpc.rs`;
  `examples/plugins/wordcount/main.py`.
- `crates/bingo-surface-print/src/{render,stream_json}.rs` (authorized option A).
- `docs/adr/{0015,0030,0031,0075}-*.md`.
- `crates/bingo-plugin-rpc/Cargo.toml`, `Cargo.lock`: existing workspace shelf;
  no external dependency. Mechanical fixtures follow step 6; SDK intercept's
  catalogue-name fixture changes to `turn.complete`. No M123 files are changed.

## Exit criteria

- [x] Schema compatibility passes; deleting a frozen field fails (output below).
- [x] Stub suite covers ADR-0075 §2, unknown methods, protocol 1 and state conflicts.
- [x] Remote `tool.call/after` sees the result; esc sends cancel and ends Interrupted.
- [x] Core side completion bills the turn or session, with the resolved model.
- [ ] Grandchild cleanup passes on Unix; Windows runtime execution awaits CI.
- [x] Manifest change re-handshakes, keeps global state and posts one notice;
      test advances tokio time without sleeping. Held links fail in words.
- [x] Binary acceptance passes with wordcount on 2 and `--plugin-dir`.
- [x] Local gates pass; MSVC checks attempted for all touched crates, with the
      documented C-header exception deferred to Windows CI (details below).

## Non-goals

M133's TypeScript kit and validate/test commands; a watcher dependency;
watching implicit directories; live provider/service registration changes;
rewinding state or putting it in the journal; speaking protocol 1 alongside 2.

## Risks and decisions

- Re-verified at `32dcbfb1`: `slot::with`, shelf `save`, dynamic sources in
  registry/late, actor mailbox billing and the existing usage tally.
- Billing decision 2026-10-13: fixture-first additive `UsageCharged`; never use
  a model round's `TurnUsage` for a side call. Deliver never awaits queued work.
- Decision 2026-10-13 (delegate, option A): add both print matches. Text draws
  nothing for `UsageCharged`; stream JSON emits the event; neither counts a round.
  This replaces the step-6 stop. Reload skips a failed held link per ADR-0074.
- Gate fixes stayed in Files: shared test home, test-module placement, mechanical
  SDK name fixture, completion module helpers, portable process test fixtures.

## Verified (2026-10-13)

All commands ran one at a time. Selected output is pasted below. Earlier
red-to-green logs and full output stay in untracked `.m132-logs/`, never committed.

```text
cargo test -p bingo-plugin-rpc --lib reload:: --locked (before implementation)
error[E0425]: cannot find function `changed` in this scope
cargo test -p bingo-plugin-rpc --lib schema::tests::protocol_two_keeps_every_frozen_shape --locked
# Temporary candidate copied from plugin-2.json, with pluginName deleted:
protocol 2 removed or retyped a frozen shape: pluginName
test result: FAILED. 0 passed; 1 failed
# Mutation restored. Schema tests:
cargo test -p bingo-plugin-rpc --lib schema:: --locked
test result: ok. 9 passed; 0 failed
cargo test -p bingo-plugin-rpc --test plugin --locked
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
cargo test -p bingo-core complete:: --locked
test result: ok. 2 passed; 0 failed
cargo test -p bingo --test plugin_rpc --locked
test result: ok. 7 passed; 0 failed
cargo fmt --all -- --check
(no output; exit 0)
cargo check --workspace --all-targets --locked
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.81s
cargo clippy --workspace --all-targets --locked -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.48s
cargo test --workspace --locked
(exit 0; 122 result lines total 5142 passed, 0 failed, 4 ignored)
test result: ok. 467 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.85s
test result: ok. 139 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.32s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
scripts/check_discipline.sh
discipline ok
scripts/budget.sh
dependencies (unique, normal): 350 (max  350)
warm cargo check -p bingo-core: 1s (max  20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
target/debug: 6 GB (soft max  5)
warn: target/debug exceeds the soft limit
test binaries: 94
budget ok
cargo deny check
advisories ok, bans ok, licenses ok, sources ok
```

The workspace run includes the print charge/round test, both reload tests,
remote cancellation, grandchild cleanup and PTY tests; no PTY rerun was needed.
Four pre-existing ignored tests remain: append measurement, 100 MiB event stress,
and the two interactive opening previews. No ignored test was added.

MSVC command for each crate: `cargo check -p <crate> --all-targets --target
x86_64-pc-windows-msvc`. SDK, core and surface-rpc each report `Finished dev`.
Plugin-rpc and surface-print: `aws-lc-sys` reports `fatal error: 'windows.h'
file not found` (also `stdlib.h`). Binary: tree-sitter/tree-sitter-bash report
`fatal error: 'stdio.h' file not found`. No Rust diagnostic occurred. Per
execution protocol step 7, these builds and Windows runtime cleanup remain
for the blocking Windows CI job; no toolchain was installed and no CI was invoked.
