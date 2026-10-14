# M157 — The person's choice is kept

## Goal

Keep explicit pane choices across starts and discover mod changes without restarting.

## Bricks

- Fixture-tested JSON pane choices, keyed by plugin name and pane id, under the
  user's data directory (`pane-choices.json`). Missing or corrupt data means no choices, never a failed start.
- Scope choices per user: screen preferences follow the person between projects;
  neither a project, a session journal nor a mod's store owns them.
- The host combines the unspoofable input task-local with remembered open choices
  when publishing panes. Only actual person input changes the stored choice.
- Waiting-row clicks submit a host action; surfaces consume the resulting pane slot.
- Reconcile discovered modules by name, origin, root and file stamps. Reuse admission,
  callback replacement and session.start replay; retain host state and store.
- Watch user and explicit roots; freeze project-name precedence until explicit
  `/plugins reload`, including project process plugins that this runtime does not load.
- One-line reload summary and fail-open per-module diagnostics. No new dependencies.

## Files

- `crates/bingo-mods/src/{ui,handlers,manager}.rs` and new small helper modules.
- `crates/bingo-surface-tui/src/{pane,commands}` for actions and completion.
- SDK plugin-source reload contract and core `/plugins` dispatch, if needed.
- `crates/bingo/src/plugins/switch.rs` help and mod guide.
- ADR-0081 and ADR-0057 dated amendments; fixture and host integration tests.

## Exit criteria

- [x] Person open survives restart; person close makes the next open wait.
- [x] Autonomous opens/closes never write choices; corrupt/missing files wait.
- [x] Reload and watch add/remove/change mods, preserving state and trust decisions.
- [x] A broken mod reports its error without disabling other mods.
- [x] Completion, help and guide describe reload and user-directory watching.
- [x] Isolated tmux drive verifies click/restart, Esc/restart, user addition and reload.
- [x] Sequential gates: fmt, check, clippy, test, discipline, budget, deny, TUI smoke,
  Windows workspace check (known header deferrals recorded).
- [x] Two feature commits and one docs commit, no push or PR.

## Non-goals

Persist pane position/focus, watch project roots, change trust tiers, live-switch built-in
plugins, change process-plugin reload, add dependencies, run live/credential tests.

## Risks

- Watch reconciliation must not elevate project precedence or bypass admission.
- Generated type files must not cause endless reloads; take stamps after generation.
- Concurrent reloads serialize; automatic changes wait for idle or a turn boundary.
- Failed writes must leave startup usable; persistence errors should be diagnosed.
- Tests await host events/conditions with bounded timeouts, never scheduling sleeps.


## Verified — 2026-10-14

All native gates passed, run sequentially. Final workspace test result summary:
162 test groups, 5,412 passed, zero failed, five ignored. Tests used isolated
fixtures; no live service or real-credential tests were invoked.

```text
$ cargo fmt --all -- --check
exit 0
$ cargo check --workspace --all-targets --locked
Finished `dev` profile [unoptimized + debuginfo] target(s)
$ cargo clippy --workspace --all-targets --locked -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s)
$ cargo test --workspace --locked
pane_choices: 1 passed; 0 failed
rescan: 3 passed; 0 failed
reload: 2 passed; 0 failed
workspace: exit 0 (162 successful result groups)
$ scripts/check_discipline.sh
discipline ok
$ scripts/budget.sh
dependencies (unique, normal): 411 (max 411)
warm cargo check -p bingo-core: 1s (max 20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
target/debug: 7 GB (soft max 5)
warn: target/debug exceeds the soft limit
budget ok
$ cargo deny check
advisories ok, bans ok, licenses ok, sources ok
$ scripts/tui-smoke.sh
tui-smoke ok
$ cargo check --workspace --all-targets --target x86_64-pc-windows-msvc
exit 101: native Windows toolchain deferral
rquickjs-sys: clang rejects MSVC /std:c11 and /DWIN32_LEAN_AND_MEAN flags
aws-lc-sys: fatal error: 'stdlib.h' / 'windows.h' file not found
```

Windows compilation remains unverified here; the installed Rust target cannot
supply the Windows C compiler/SDK. This is the permitted native-header/toolchain
deferral, not a successful Windows build. No new dependency was added.
An earlier full run hit the existing gateway claim-lock test's held-after-drop
failure; its isolated retry and the final full suite passed without gateway edits.

The tmux drive used `tmux -L bingo-m157-drive-hylm794m -f /dev/null`, a clean
process environment, a temporary HOME and a fake provider. The process cwd was
the temporary project, matching discovery's startup project root. Captured output:

```text
PASS initial pane waits
PASS click seats pane
PASS restart seats pane
PASS Escape closes pane
PASS restart waits after close
PASS new user folder loads while running
reloaded: 1 added (manual-fixture), 0 removed, 0 reloaded
PASS /plugins reload answers with changes
PASS all M157 tmux scenarios
Removed isolated socket and temporary home
```

The drive left the real `~/.bingo` untouched. Its temporary directory was deleted,
not moved to Trash. Raw verification logs are `/tmp/m157-gate-*.log` and
`/tmp/m157-drive.log`. The watcher harness also proves that a new same-name
project mod cannot displace a watched user mod before explicit reload, and that
state/store, refusal and independent healthy modules survive replacement.

## Review follow-up verified — 2026-10-14

`ArgSpec::Words.alone` owns words that end their argument; TUI completion reads
that contract without command-specific grammar. Empty lists stay off the wire;
old frames still decode. Generated schemas/types and ADR-0008 §6a agree.
The guide's wording is unchanged, its opening paragraph is within 80 columns,
and the page remains 199 lines. Added one follow-up commit without rebasing.

Sequential verification (logs: `/tmp/m157-review-*.log`):

```text
cargo fmt --all -- --check: exit 0
cargo check --workspace --all-targets --locked: exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings: exit 0
cargo test -p bingo-sdk -p bingo-core -p bingo-mcp -p bingo-mods \
  -p bingo-permissions -p bingo-schedule -p bingo-surface-tui \
  -p bingo-plugin-rpc --locked: 2,459 passed, 0 failed, 3 ignored
cargo test --workspace --locked: 5,414 passed, 0 failed, 5 ignored
scripts/check_discipline.sh: discipline ok (existing advisory warnings)
scripts/budget.sh: budget ok
  dependencies: 411 (max 411); warm core check: 0s (max 20s)
  relink isolation: 0; target/debug: 10 GB (soft max 5, warning)
```
