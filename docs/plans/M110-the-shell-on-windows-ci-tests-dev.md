# M110 — The shell on Windows; CI tests `dev`

## Goal

ADR-0063: one shell resolution in a new `bingo-process` library, Git
Bash on Windows, the Bash tool withheld with a remedy when no shell
resolves. Then CI that would have caught it: Windows `cargo test`
blocking, and every push to `dev` tested. After this milestone the
Windows binary the release ships can run the model's commands.

## Bricks, in build order

1. **Contract tests for the resolution, before it exists.** In
   `bingo-process`, `shell::resolve_with(env: &dyn ShellEnv)` is pure
   over an injected view of the environment (variables, `PATH`
   entries, "does this file exist"), so every case is a unit test on
   every platform:
   - `BINGO_SHELL` set and present → it; set and absent →
     `ShellMissing` naming it;
   - unix: `/bin/bash`, else `/bin/sh`;
   - windows: `PATH` `bash.exe` taken; one under `%SystemRoot%` or a
     `WindowsApps` directory skipped; then the three Git install
     locations in ADR-0063 §4 order; then `git.exe` on `PATH` →
     `..\bin\bash.exe`; none → `ShellMissing` listing what was tried.
   The platform is a parameter of `resolve_with`, not a `cfg`, so the
   Windows rules are tested on macOS and Linux too. `resolve()` is the
   memoised call with the real environment and `cfg!(windows)`.
2. **The crate.** `crates/bingo-process`, `tier = "library"`, depends on
   `thiserror` only. Modules: `shell` (above). Nothing else yet; M128
   adds the spawner.
3. **Both callers move.** Delete `shell()` from
   `bingo-tool-bash/src/run.rs` and `bingo-hooks-shell/src/run.rs`; call
   `bingo_process::shell::resolve()`. The bash tool's `spawn` and the
   hook runner keep their process-tree wrapping unchanged.
4. **No shell, no tool.** Where `bingo-tool-bash` contributes its tools
   and the `!` command, skip the contribution on `ShellMissing` and
   push one notice (`SHELL_MISSING`, warn) with ADR-0063 §5's remedy.
   The `!` command's existing "no shell here" answer (ADR-0008 §5)
   covers the person's side. Hooks: a shell hook fails as a hook error
   with the same words. Tests for both with an injected missing shell.
5. **The description names the shell.** `run::shell()` in the tool
   description becomes the resolved path; on Windows the description adds
   one sentence: Git Bash, `/c/...` paths, Windows programs on `PATH`.
6. **CI.** `.github/workflows/ci.yml`:
   - `on.push.branches: [main, dev]`; `concurrency: { group: ci-${{
     github.ref }}, cancel-in-progress: true }`; `timeout-minutes` on
     every job (45 for test and windows, 15 for the rest);
   - the windows job: drop `continue-on-error` and its comment; run
     `cargo test --workspace --locked -- --test-threads=2`. Git Bash is
     on the hosted runner; set nothing.
7. **Make Windows green honestly.** Run the windows job (push the
   branch only if the person allows it — otherwise use
   `cargo test --no-run --target x86_64-pc-windows-msvc` locally for
   what compiles, and record that CI is the first real run). Each test
   that cannot run on Windows gets
   `#[cfg_attr(windows, ignore = "<why>")]`; each real failure is fixed
   or, if outside this plan's Files, written in Risks and the run stops
   (protocol step 6).
8. ADR-0063 accepted; dated note on ADR-0001 naming the new library.

## Files

- `crates/bingo-process/**` (new), workspace `Cargo.toml`, `Cargo.lock`
- `scripts/budget.toml` (+1 member: 342 → 343; no new external dependency)
- `crates/bingo-tool-bash/{Cargo.toml,src/run.rs,src/lib.rs}` and tests
- `crates/bingo-hooks-shell/{Cargo.toml,src/run.rs}` and tests
- `.github/workflows/ci.yml`
- `#[cfg_attr(windows, ignore)]` lines in whichever tests need them
- `docs/adr/0063-*.md`, `docs/adr/0001-crate-map.md`

## Exit criteria

- [x] `rg -n 'fn shell\(' crates/bingo-{tool-bash,hooks-shell}` finds
      no definitions; both callers use `bingo_process::shell::resolve`.
- [x] Resolution unit tests cover every case in brick 1 and pass on
      the host platform (output pasted).
- [x] With `BINGO_SHELL=/nonexistent`, a session offers no Bash tool and
      shows the notice (test output pasted).
- [x] `cargo check -p bingo-process -p bingo-tool-bash -p
      bingo-hooks-shell --all-targets --target x86_64-pc-windows-msvc`
      exit 0.
- [x] `ci.yml` has no `continue-on-error`; `dev` is a push trigger.
- [ ] The windows job's result is recorded: green, or the run stopped
      with each failure listed.
- [ ] All gates: stopped before the workspace host gates (see below).

## Non-goals

- PowerShell or cmd support; WSL. Sharing the spawner (M128).
- Changing which tests run on unix.

## Risks

- **Stopped 2026-10-13, protocol step 6:** brick 7's workspace Windows
  test build exits 101 in `aws-lc-sys`: the cross C toolchain lacks
  `stdlib.h` and `windows.h`. Supplying a Windows SDK/toolchain changes
  the host outside Files; no toolchain or external crate was changed.
  Options: resume on a prepared Windows host/cross toolchain, or explicitly
  defer this build and finish host gates with Windows execution pending.
  No push is authorized; ADR acceptance and the ADR-0001 note are pending.

- A Windows CI test checks the resolved Git Bash path; WSL is skipped.
- Process-heavy suites may be slow on Windows; `--test-threads=2` and the
  timeout bound it. A suite that needs more is a finding, not a reason
  to keep the step soft.
- Pushing to run CI is the person's decision (protocol step 9).
- Re-verified at `82e78d22`: both resolvers and description call exist;
  registration uses `Registrar` contributions. A session-start hook can
  publish the missing-shell notice through `HostApi::notice`.
- Local Windows execution is unavailable; cross-compilation checks types,
  and CI will be the first real Windows run (no push authorized).

## Verified (2026-10-13)

Partial evidence only; M110 is **not verified**. No Windows CI run occurred.
Contract tests were committed before implementation; expected red output:
```text
$ cargo test -p bingo-process
shell resolved: ShellMissing { tried: [] }
test result: FAILED. 2 passed; 6 failed; 0 ignored
$ cargo test -p bingo-hooks-shell a_missing_shell --locked
run::tests::a_missing_shell_is_a_hook_error_with_the_remedy ... FAILED
$ cargo test -p bingo-tool-bash a_missing_shell --locked
tests::a_missing_shell_withholds_tools_and_warns_the_session ... FAILED
$ cargo test -p bingo-process -p bingo-tool-bash -p bingo-hooks-shell --locked
run::tests::a_missing_shell_is_a_hook_error_with_the_remedy ... ok
test result: ok. 104 passed; 0 failed; 0 ignored
test result: ok. 9 passed; 0 failed; 0 ignored
tests::a_missing_shell_withholds_tools_and_warns_the_session ... ok
test result: ok. 145 passed; 0 failed; 0 ignored
$ cargo check -p bingo-process -p bingo-tool-bash -p bingo-hooks-shell --all-targets --target x86_64-pc-windows-msvc
Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.86s
$ scripts/budget.sh  # after adding the library member
dependencies (unique, normal): 343 (max  343)
warm cargo check -p bingo-core: 1s (max  20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
budget ok
$ cargo test --workspace --locked --no-run --target x86_64-pc-windows-msvc
fatal error: 'stdlib.h' file not found
fatal error: 'windows.h' file not found
error: failed to run custom build command for `aws-lc-sys v0.44.0`
# exit 101; protocol step 6 stop
```

`rg -n 'fn shell\(' crates/bingo-tool-bash crates/bingo-hooks-shell`
returned no matches. CI diff has `[main, dev]`, concurrency, all job timeouts,
and blocking Windows tests with two threads. `cargo fmt --all` ran; the
format check, workspace check/clippy/test (including the isolated ACP bridge
token test), discipline, final budget and deny gates have not run.
