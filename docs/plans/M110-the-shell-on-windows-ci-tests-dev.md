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
7. **Make Windows green honestly.** Cross-check each touched crate with
   `cargo check --all-targets --target x86_64-pc-windows-msvc`. Actual
   Windows tests await the first CI run after a person pushes `dev`.
   Each test that cannot run there gets a reasoned Windows ignore; each
   real failure is fixed or, if outside Files, reported under step 6.
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
- [ ] Windows job: pending the first CI run after a person pushes dev.
- [x] All gates: host gates and touched-crate Windows cross-check passed.

## Non-goals

- PowerShell or cmd support; WSL. Sharing the spawner (M128).
- Changing which tests run on unix.

## Risks

- **User decision, 2026-10-13:** defer the workspace Windows test build;
  it is not a local gate. Its `aws-lc-sys` failure needs Windows SDK
  headers (`stdlib.h`, `windows.h`) absent on this Mac. Do not install
  toolchains. Protocol step 7 requires only touched-crate Windows checks,
  which passed. Finish host gates and accept the ADR if they pass; keep
  Windows CI pending. Never push.
- A Windows CI test checks the resolved Git Bash path; WSL is skipped.
- Windows process suites use two threads and a timeout; failures stay blocking.
- Re-verified at `82e78d22`: both resolvers and description call exist;
  registration uses `Registrar` contributions. A session-start hook can
  publish the missing-shell notice through `HostApi::notice`.

## Verified (2026-10-13)

Local gates passed; Windows CI remains pending by the decision above.
Contract tests were committed before implementation; red then green:
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
$ cargo fmt --all -- --check
exit 0
$ cargo check --workspace --all-targets --locked
Finished `dev` profile [unoptimized + debuginfo] target(s) in 23.54s
$ cargo clippy --workspace --all-targets --locked -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 20.37s
$ cargo test --workspace --locked -- --test-threads=2 --skip a_proxy_with_a_token_this_run_never_minted_gets_nothing
# Exit 0; summed results: 4958 passed, 0 failed, 3 ignored, 1 filtered out.
$ cargo test -p bingo --test acp_bridge --locked a_proxy_with_a_token_this_run_never_minted_gets_nothing -- --exact --test-threads=1
test a_proxy_with_a_token_this_run_never_minted_gets_nothing ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out
$ scripts/check_discipline.sh
dependency direction ok
discipline ok
$ scripts/budget.sh
dependencies (unique, normal): 343 (max  343)
warm cargo check -p bingo-core: 1s (max  20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
target/debug: 3 GB (soft max  5)
budget ok
$ cargo deny check
advisories ok, bans ok, licenses ok, sources ok
```

Existing ignores: 100MiB pinned-event stress, opening storyboard play/preview.
No failure reruns needed. Discipline and deny emitted non-blocking warnings.
Both old `fn shell` definitions are gone; CI configuration criteria checked.
