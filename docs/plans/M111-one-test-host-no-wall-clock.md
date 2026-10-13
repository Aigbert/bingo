# M111 — One test host, no wall clock

## Goal

Share one scriptable, recording `bingo_sdk::testing` host across plugin tests;
remove machine-speed assertions. Preserve `HostApi` defaults as M119's foundation.

## Bricks, in build order

1. Add `TestHost` behind the existing `testing` feature. Record every verb
   and argument in `HostCall`; provide per-verb scripts, otherwise `NoHost`
   defaults. Support a seeded in-memory session table (sessions/open/close/
   delete), services map and driven gateway stream. Contract tests come first;
   document recorded verbs and state. No new dependency.
2. Migrate plugin fakes, one crate per commit: agents, rooms, tasks, schedule,
   skills, checkpoints, permissions, tool-bash, context, demo-ui, channels,
   TUI, print, MCP and plugin-RPC. Agents and channels go last, separately,
   touching only the fake and its uses. Keep a fake only if expressing its
   behaviour needs a feature noun; add a reason line.
3. Keep production implementations in core host, unavailable, session commands
   and RPC client. Retain core test support and RPC wire host with reason
   comments: they drive kernel/port behaviour. Leave core session/context
   test bodies alone beyond replacing a fake.
4. Replace TUI warm/delta/per-draw speed bounds with cache behaviour counters.
   Replace `elapsed() < N` liveness assertions with `tokio::time::timeout`
   and the same message; list each in its commit body. Pause Tokio time for
   owned timers (chase, wakes, retry), preserving already-paused tests.
5. Add warning-only discipline scans for new qualified/unqualified `HostApi`
   implementations outside allowed files and elapsed-time test assertions.

## Files

- `crates/bingo-sdk/src/testing.rs` and `testing/`
- Test modules/fakes in brick 2 and their `Cargo.toml` dev-dependencies,
  including checkpoints `src/lib.rs`, MCP `tests/server.rs` and
  plugin-RPC `tests/plugin/harness.rs`
- TUI `src/{blocks.rs,highlight.rs,view.rs}` cache tests
- Core `src/test_support{.rs,/peers.rs}` and RPC wire host: reason comments only
- Workspace tests with elapsed bounds, including shell hooks and CLI/RPC
- `crates/bingo-schedule/src/wakes.rs` timer tests
- `scripts/check_discipline.sh`

## Exit criteria

- [x] Host implementations are production, SDK utilities or reasoned exceptions.
- [x] No remaining test assertion bounds machine speed.
- [x] Workspace test count does not drop: 4,968 → 4,974; three ignored both times.
- [x] All gates: local checks passed; six Windows checks left to CI by decision below.

## Non-goals

Typed services (M119), HostApi changes, kernel test support or PTY smoke rewrites.
No TUI-visible behaviour changes; smoke is not required.

## Risks

- Preserve each fake's assertions; do not loosen behaviour while migrating.
- Wide migration: separate crate commits, targeted tests, then full gates.
- **Decision, 2026-10-13:** Windows checks failing only because third-party
  C build scripts cannot find Windows SDK headers are left to Windows CI;
  no toolchain installed. Any Rust compile error still blocks.
  `bingo-permissions`: `tree-sitter` and `tree-sitter-bash`;
  `bingo-surface-print`, `bingo-surface-tui`, `bingo-mcp`, `bingo-channels`,
  `bingo`: each has `aws-lc-sys`. Windows CI results remain pending.

## Re-verification (2026-10-13, `3d53863d`)

All named paths/symbols exist; unqualified host scan found 23 files. Add MCP
and plugin-RPC fakes to Files before code. The qualified scan also found
checkpoints' `OneSession`; include it in the checkpoints migration. Three TUI
speed bounds and Bash/hooks/CLI/RPC liveness bounds remain. Chase/deadline and
core retry already pause time; schedule wakes did not. No ADR is named.

## Verified (2026-10-13)

Sequential command logs: ignored `target/m111/`. Contracts: `726c6739`; implementation: `f9fc41a5`.
```text
$ cargo test -p bingo-sdk --features testing --locked
error[E0432]: unresolved imports `super::HostCall`, `super::TestHost`
error: could not compile `bingo-sdk` (lib test) due to 1 previous error
$ rg -l 'impl (bingo_sdk::)?HostApi for' crates
crates/bingo-surface-rpc/src/client.rs
crates/bingo-surface-rpc/tests/wire/host.rs
crates/bingo-sdk/src/testing.rs
crates/bingo-sdk/src/testing/api.rs
crates/bingo-core/src/session/commands.rs
crates/bingo-core/src/test_support/peers.rs
crates/bingo-core/src/host.rs
crates/bingo-core/src/test_support.rs
crates/bingo-core/src/host/unavailable.rs
$ rg -n 'elapsed\(\) ?<' crates --glob '**/test*'
(no matches; exit 1)
```
The advisory also scans `#[cfg(test)]` modules: no speed warnings. A synthetic
fixture proved both advisory warnings. Retained core/RPC fixtures have reasons.

Baseline/final commands (ACP token test explicitly isolated in both):
```text
$ cargo test --workspace --locked -- --skip a_token_may_be_dialled_again_once_its_stream_has_closed
$ cargo test -p bingo-provider-acp --test bridge --locked a_token_may_be_dialled_again_once_its_stream_has_closed -- --exact --test-threads=1
baseline: 4968 passed; 3 ignored; 1 initial failures
final: 4974 passed; 3 ignored; 0 initial failures
```
Totals sum Cargo result lines. Baseline's theme PTY and an earlier final run's
rewind PTY each passed alone (`--exact --test-threads=1`). Baseline resumed with
`--workspace --exclude bingo` (same ACP skip), then `-p bingo --test rooms --test rpc --test thinking --test views` (all locked).
The last full run (`test-final.log` plus `test-token-final.log`) had no failures.
The qualified checkpoints correction also passed all 42 crate tests.
```text
$ cargo fmt --all -- --check
(exit 0; no output)
$ cargo check --workspace --all-targets --locked
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.25s
$ cargo clippy --workspace --all-targets --locked -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.85s
$ scripts/check_discipline.sh
test host and wall-clock advisory checks complete
discipline ok
$ scripts/budget.sh
dependencies (unique, normal): 343 (max  343)
warm cargo check -p bingo-core: 0s (max  20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
target/debug: 7 GB (soft max  5)
warn: target/debug exceeds the soft limit
test binaries: 91
budget ok
$ cargo deny check
advisories ok, bans ok, licenses ok, sources ok
$ cargo check -p bingo-sdk -p bingo-core -p bingo-agents -p bingo-rooms -p bingo-tasks -p bingo-schedule -p bingo-skills -p bingo-checkpoints -p bingo-context -p bingo-demo-ui -p bingo-tool-bash -p bingo-hooks-shell -p bingo-plugin-rpc -p bingo-surface-rpc --all-targets --locked --target x86_64-pc-windows-msvc
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 12.99s
$ cargo check -p bingo-permissions -p bingo-surface-print -p bingo-surface-tui -p bingo-mcp -p bingo-channels -p bingo --all-targets --locked --target x86_64-pc-windows-msvc
fatal error: 'stdlib.h' file not found
fatal error: 'windows.h' file not found
error: failed to run custom build command for `aws-lc-sys v0.44.0`
(exit 101; Windows SDK C headers missing; left to Windows CI)
$ cargo check -p bingo-permissions --all-targets --target x86_64-pc-windows-msvc --locked
error: failed to run custom build command for `tree-sitter v0.26.13`
fatal error: 'stdio.h' file not found
error: failed to run custom build command for `tree-sitter-bash v0.25.1`
fatal error: 'stdlib.h' file not found
(exit 101; Windows SDK C headers missing; left to Windows CI)
$ cargo tree -p <crate> -i aws-lc-sys --target x86_64-pc-windows-msvc --locked
(first output line for each of six sequential invocations)
bingo-permissions: error: package ID specification `aws-lc-sys` did not match any packages
bingo-surface-print: aws-lc-sys v0.44.0
bingo-surface-tui: aws-lc-sys v0.44.0
bingo-mcp: aws-lc-sys v0.44.0
bingo-channels: aws-lc-sys v0.44.0
bingo: aws-lc-sys v0.44.0
```
