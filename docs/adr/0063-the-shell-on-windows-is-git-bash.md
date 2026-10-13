# ADR-0063 — The shell on Windows is Git Bash

Status: proposed · 2026-10-13 · Plan: M110

## Context

The Bash tool and the shell hooks each resolve a shell of their own
(`bingo-tool-bash/src/run.rs` and `bingo-hooks-shell/src/run.rs`, two
`OnceLock`s) and both answer `/bin/bash` or `/bin/sh`. On Windows neither
path exists, so every command and every shell hook fails at spawn, and
CI's Windows `cargo test` step is `continue-on-error` for that reason
alone — its comment still waits on "M6's question". The release ships a
Windows binary whose main tool cannot run.

The model writes POSIX shell: its training, every `AGENTS.md`, every
hook a person copies from a README. Teaching it PowerShell per platform
would split every prompt, every tool description and every hook in two.
Claude Code answered the same question by requiring Git for Windows and
running its `bash.exe`, with `CLAUDE_CODE_GIT_BASH_PATH` as the override.
Windows also ships `%SystemRoot%\System32\bash.exe`, which is the WSL
launcher: it runs the command in a Linux VM with another filesystem and
must never be taken for Git Bash. A `bash.exe` under `WindowsApps` is the
same launcher's store alias.

## Decision

1. **One resolution.** A new library crate `bingo-process`
   (`tier = "library"`, ADR-0012 §1) owns `shell::resolve() ->
   Result<Shell, ShellMissing>`, memoised once per process. The Bash tool
   and the shell hooks call it; neither keeps a copy. ADR-0072 later
   adds the process spawner to the same crate.
2. **`BINGO_SHELL` wins** on every platform: an absolute path to a
   POSIX shell. A path that does not exist is `ShellMissing` naming it,
   not a silent fall-through.
3. **Unix** keeps today's order: `/bin/bash`, then `/bin/sh`.
4. **Windows** takes the first that exists, in order:
   1. `bash.exe` on `PATH`, skipping any under `%SystemRoot%` and any
      under a `WindowsApps` directory (the WSL launcher and its alias);
   2. `%ProgramFiles%\Git\bin\bash.exe`, then
      `%ProgramFiles(x86)%\Git\bin\bash.exe`, then
      `%LocalAppData%\Programs\Git\bin\bash.exe`;
   3. `git.exe` on `PATH`, from which `..\bin\bash.exe` is derived
      (Git's `cmd\git.exe` sits beside its `bin\`).
5. **No shell, no tool.** When resolution fails the Bash tool is not
   offered to the model at all (it is a contribution, not a runtime
   error), and the session carries one ephemeral notice naming the
   remedy: install Git for Windows or set `BINGO_SHELL`. A shell hook
   fails as a hook error (ADR-0032 §5) with the same words.
6. **The description says which shell.** The Bash tool's description
   names the resolved path, and on Windows adds that it is Git Bash, so
   paths are `/c/Users/...` and Windows programs are on `PATH`.
7. **CI runs `cargo test` on Windows, blocking.** A test that cannot run
   there carries `#[cfg_attr(windows, ignore = "<reason>")]` with the
   reason; there is no job-level `continue-on-error`. CI also runs on
   pushes to `dev`, with a timeout per job and one run per ref.

## Consequences

- One function to change when a fourth location matters; the two
  copies cannot drift. `bingo-tool-bash` and `bingo-hooks-shell` gain
  one library edge each, which ADR-0012 §1 allows.
- A person on Windows without Git sees why the model has no shell,
  instead of a tool that fails every call.
- Process trees stay as they are (process group on unix, job object on
  Windows, `bingo-tool-bash/src/run.rs`); this ADR moves only the name
  of the program. Sharing the spawner is ADR-0072's.
- Cygwin's and MSYS2's own `bash.exe` are found through `PATH` and
  accepted: they are POSIX shells with the same path dialect.
- The Windows test job gets slower and will find real failures; each
  becomes a fix or a named `ignore`, never a step that may fail.

Refs: ADR-0001, ADR-0012, ADR-0032; Plan: M110; audit-2026-10 §Also
