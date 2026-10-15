# M178 — Memory in one panel

ADR-0092, ADR-0049, ADR-0014, ADR-0082, ADR-0086. Needs M167.
M170 and M177 remain independent; amendments below explain their existing substitutes.

## Goal

`/memory` opens one panel for memories, loaded instruction files and this project's
experience. Enter reads a row, `d` asks to forget a memory or entry. The model stays
its only author. `/memory forget [user|project] <name>` and `/experience forget <id>`
use the same operations. With `bingo.mod.memory` off, both original tables remain.
The sketch is the design, at 108×30 and 80×24; there is no prototype.

```
 memory · about 6.1k tokens carried                       ⏎ read  d forget
 memories (4)
❯ project  feedback  no-sonnet-workers   Workers are Opus only      320
  user     user      role                Rust reviewer, prefers…    110
 instructions (2)
  ~/.bingo/AGENTS.md                                             about 900
  ./AGENTS.md                                                   about 4.2k
 experience (3)
  01J9…  active  fixture before impl for wire formats  3 ok      about 80
 ──────────────────────────────────────────────────────────────────────
 forget "no-sonnet-workers"? the file and its index line go.  ⏎ yes  esc
```

## Bricks

1. **Unlink, pure.** `memory/index.rs::unlink(index, file)` drops each line whose
   inline Markdown link targets the file; other bytes, headings, blanks and CRLF
   survive. Tables include two links per line and a link with a title.
2. **Forget.** `memory/store.rs::forget` removes the file, then writes the index to
   a same-directory temporary file and renames over the old index. The helper lives
   in `forget.rs`. Missing files are `NOT_FOUND`; ambiguous names are `INVALID_INPUT`
   naming both scopes. Validate names; temp-HOME tests cover replacement and cleanup.
3. **Instructions.** `InstructionsContributor::loaded(cwd)` reuses the contributor's
   walk and capped blocks, in order, measured with `bingo_sdk::tokens::text`.
4. **Services, contracts first.** `bingo_services::{memory,experience}` define native-or-wire
   `find(host)`, `list`, `read`, `forget`; every request carries cwd. Memory rows carry
   scope/name/kind/description/path/tokens/error, plus loaded instructions. Experience
   rows carry id/status/summary/outcomes/tokens. Fixtures precede implementations.
5. **Commands.** Instant forget commands call the same owner functions; experience
   uses `Library::delete`, also used by `ExperienceForget`. Replies match services.
   Memory replies explain prompt retention until next composition and possible relearning.
6. **Rows, pure.** `bingo-mod-memory/rows.rs` groups rows and totals estimated tokens.
   Tests include empty scopes, malformed memories with errors and no experience owner.
7. **Keys.** Root `onKey` owns arrows, Enter to the text Pager and `d` to confirmation.
   Confirmation calls the typed forget operation shared with the command; stale targets
   delete nothing. Instructions refuse with `edit the file itself`; Escape cancels/closes.
8. **Drawing.** Shared `Pane` tree, counts, right-aligned estimates, selection and a last-line
   confirmation. Full/empty/confirming/malformed TestBackend snapshots at both sizes.
9. **Guide.** `src/guide.md`, registered as a `Page` in `guide.rs`, under 200 lines.

## Files

- Context: instructions, memory index/store/forget/command/service, registration, tests, guide.
- Experience: command/service, registration, parity tests, guide.
- Services: memory/experience contracts and fixtures; new `bingo-mod-memory` crate.
- TUI: generic `run/surface.rs`, pager sources, pane Escape, rendering, tests and snapshots.
- Core: `host/registry.rs` classifies static guide Pages; binary registration and CLI tests.
- Workspace/binary/test crate edges and lockfile; `scripts/budget.toml` +1 member, no external dependency.
- ADR-0049 amendment and ADR-0092 clarification; guides stay in owning crates, never `docs/guide/`.

## Exit criteria

- [x] Unlink/rows/key tables, temp-directory forget tests, both parity tests, service fixtures.
- [x] TestBackend: open, page memory, confirm forget, remove file/index/row, refuse instruction deletion.
- [x] Ambiguous unscoped forget names both scopes without deletion; CLI exit/stdout/NDJSON coverage.
- [x] `enabledPlugins: {"bingo.mod.memory": false}` preserves both original tables.
- [x] Isolated tmux socket, temporary HOME/data, fake provider, two memories, instructions,
      one experience entry; read both kinds, forget one of each, capture both sizes.
- [x] Sequential fmt/check/clippy/workspace tests/discipline/budget/deny/tui-smoke gates.
- [ ] Windows all-target checks for every touched crate (toolchain deferrals recorded below).

## Decisions and amendments

- The existing SDK estimator already serves context measurement; no M170 `ContextPart` is needed.
  M177 is absent, so text Pager support extends the current item pager. Typed `read` was missing
  from the original service brick but is required by the keys brick; owners authorize listed files.
- Confirmation calls the typed service directly: nested command submission adds scheduling inside
  an action handler without changing its operation. Commands and services have reply-parity tests.
- Static, native, wire-free `Pages` are catalogue data, not runtime services. The previous classifier
  prevented documented mods from switching now; actual memory/experience services still switch
  next start. This is a generic correction, with no feature noun added to core or SDK.
- Panes opting out of automatic close pass Escape to their root handler, enabling cancellation.
  An overlay fits the sketch at 80×24; above-prompt placement reserves eight transcript rows.
- Exact interceptor prefilters limit interception to bare memory/open and its disable command;
  a broad matcher had turned unrelated command errors into skipped-interceptor notices.
- Keep bare commands' `ArgSpec::None`: a free-argument spec made Enter open an argument page.
  Typed forget arguments still work. This was caught by the isolated terminal drive.
- Keep actual paths for reads and shorten instruction labels relative to cwd/home for drawing.
  A flexible label with zero minimum width reserves space for estimates even for malformed rows.
- Services take explicit cwd, avoiding a hidden active project. The mod clears its cursor on
  session end and closes its panes on live disable; absent experience is an empty group.
- New workspace edges are services for owners/mod and owners/mod/services for TUI tests.
  No external dependencies or allowlist changes; the member budget grows 414 → 415.

## Non-goals and risks

No authoring/editing, `$EDITOR`, instruction editing or other projects' memories.
A concurrent writer may replace the index after forget (last whole-file writer wins).
Token counts estimate listed material, not server usage; recall and prompt caps may carry less.
A model may relearn a forgotten fact from the conversation; the reply tells the person to say so.

## Verified — 2026-10-15

All commands used temporary HOME/data, real tool binaries outside mise shims, no credentials or live providers. Terminal drives used their own `tmux -L … -f /dev/null` and fake scripts.
Final sequential gate output summaries (exit 0):
```text
cargo fmt --all -- --check: clean
cargo check --workspace --all-targets --locked: Finished dev profile in 3.96s
cargo clippy --workspace --all-targets --locked -- -D warnings: Finished in 6.33s
cargo test --workspace --locked: 5754 passed; 0 failed; 5 ignored (summed)
bash scripts/check_discipline.sh: discipline ok (existing advisories)
bash scripts/budget.sh: 415/415 dependencies; 0s core check; 0 relinked; budget ok
cargo deny check: advisories ok, bans ok, licenses ok, sources ok
bash scripts/tui-smoke.sh: tui-smoke ok
```
Budget warned at 20 GB target/debug; final `cargo clean` removes build artifacts.
Generators: `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk -p bingo-plugin-rpc -p bingo-surface-rpc -p bingo-store-jsonl -p bingo-mods --locked --lib`:
464 passed, 1 ignored; schema/ and sdk/typescript regeneration produced no diff.
Windows, separately: `cargo check -p CRATE --all-targets --target x86_64-pc-windows-msvc --locked`:
`bingo-services`, `bingo-context`, `bingo-experience`, `bingo-core`, `bingo-mod-memory`: exit 0.
`bingo-surface-tui`, `bingo`: exit 101 in aws-lc-sys; missing Windows stdlib.h/windows.h.
The Windows criterion remains open for the release toolchain; no Rust diagnostic preceded it.
Earlier command-error regressions were fixed by exact prefilters; discipline caught missing
service declarations and test-file classification, now corrected. Temporary-HOME mise prompts
were removed with real executable paths and `SHELL=/bin/sh`. One smoke exit-hint timeout at
120×40 passed on the unchanged isolated rerun. A wrapper-cleanup error prompted a clean rerun.

Final tmux drive: read both kinds, forget both, refuse instructions, live disable and original table fallback passed. Capture excerpts (full/empty/confirming/malformed goldens cover both sizes):
```text
108×30
 memory · about 131 tokens carried                                                          ⏎ read  d forget
❯ project  feedback   no-sonnet-workers   Workers are Opus only                                     about 31
  01J9fixture  active  fixture before impl for wire formats  0 ok · 0 harmful                       about 47
80×24
 memory · about 131 tokens carried                              ⏎ read  d forget
 memories (2)
❯ project  feedback   no-sonnet-workers   Workers are Opus only         about 31
  user     user       role   Rust reviewer, prefers concise answers     about 27
 instructions (1)
  ~/.bingo/AGENTS.md                                                    about 26
 experience (1)
  01J9fixture  active  fixture before impl for wire formats  0 ok · 0 … about 47
 forget "no-sonnet-workers"? the file and its index line go.  ⏎ yes  esc
After both forgets: about 53 tokens carried; memories (1); experience (0).
```
