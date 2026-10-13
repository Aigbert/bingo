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

- `crates/bingo-process/**` (new), workspace `Cargo.toml`
- `crates/bingo-tool-bash/{Cargo.toml,src/run.rs,src/lib.rs}` and tests
- `crates/bingo-hooks-shell/{Cargo.toml,src/run.rs}` and tests
- `.github/workflows/ci.yml`
- `#[cfg_attr(windows, ignore)]` lines in whichever tests need them
- `docs/adr/0063-*.md`, `docs/adr/0001-crate-map.md`

## Exit criteria

- [ ] `rg -n 'fn shell\(' crates` finds one definition, in
      `bingo-process`.
- [ ] Resolution unit tests cover every case in brick 1 and pass on
      the host platform (output pasted).
- [ ] With `BINGO_SHELL=/nonexistent`, a session offers no Bash tool and
      shows the notice (test output pasted).
- [ ] `cargo check -p bingo-process -p bingo-tool-bash -p
      bingo-hooks-shell --all-targets --target x86_64-pc-windows-msvc`
      exit 0.
- [ ] `ci.yml` has no `continue-on-error`; `dev` is a push trigger.
- [ ] The windows job's result is recorded: green, or the run stopped
      with each failure listed.
- [ ] All gates.

## Non-goals

- PowerShell or cmd support; WSL. Sharing the spawner (M128).
- Changing which tests run on unix.

## Risks

- The hosted runner's `bash.exe` on `PATH` may be Git's or WSL's
  depending on image order; the skip rule covers WSL, and a test asserts
  the resolved path ends in `Git\bin\bash.exe` or `usr\bin\bash.exe`
  when running on Windows CI.
- Process-heavy suites may be slow on Windows; `--test-threads=2` and the
  timeout bound it. A suite that needs more is a finding, not a reason
  to keep the step soft.
- Pushing to run CI is the person's decision (protocol step 9).
- Re-verify at start: both `shell()` copies and their callers by name.
