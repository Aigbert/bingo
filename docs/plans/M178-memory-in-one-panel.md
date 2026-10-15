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

1. Pure index unlink preserves other bytes, CRLF and headings; targets accept `./`,
   fragments and percent encoding without case folding.
2. Forget validates the name, writes and syncs a same-directory exclusive temp file,
   checks the index again, renames and syncs the directory on Unix, then removes the file.
   Retry one concurrent edit; refuse a second. Windows directory sync is a documented no-op.
3. Instructions reuse the contributor's walk, caps and SDK token estimator.
4. Native-or-wire memory/experience services expose list/read/forget with explicit cwd.
   Fixtures precede implementations; kind/status/outcomes are enums, including unreadable rows.
5. Commands share owner operations and replies. Memory commands carry the real config path.
   Blocking filesystem transactions run in spawn_blocking, including experience service I/O.
6. Pure grouped rows sum capped memory index lines and active experience index lines, not bodies.
7. Root keys require an open cursor and the drawn target matching its selected cached row.
   Enter reads, d confirms, Escape cancels/closes; instruction deletion is refused.
8. Pane listings load on open and after forget. Name columns align, estimates stay right,
   separators follow width, instruction titles shorten paths, forget toasts fit 80 columns.
9. Text pagers fence frontmatter verbatim and render the body as Markdown; returning to a
   removed pane closes normally. JS closeOnEscape=false remains respected.
10. Per-crate guide.md pages are registered in guide.rs and remain under 200 lines.

## Files

- Context: instructions, memory index/store/forget/command/service, registration, tests, guide.
- Experience: command/service, shared index cap, registration, tests, guide.
- Services: memory/experience contracts and fixtures; new bingo-mod-memory crate.
- TUI: generic surface/pager/pane handling, integration tests and eight snapshots.
- Core: static Pages classification; binary: registration and CLI tests (original M178).
- TypeScript UI regression; schema/TypeScript generator tests; ADR-0049 and ADR-0092.
- Workspace edges and budget +1 member; no new external crate or allowlist changes.

## Exit criteria

- [x] Pure tables, temp-HOME transactions, atomicity/error/concurrent-edit tests, owner parity, fixtures.
- [x] Closed/mismatched actions cannot forget; cached lists refresh after forget; unreadable rows remain.
- [x] TestBackend at 108×30/80×24: panel, frontmatter pagers, confirmation, refusal and snapshots.
- [x] Ambiguous unscoped forget names both scopes; CLI exit/stdout/NDJSON; disabled original tables.
- [x] Isolated tmux/fake-provider drive: two memories, instructions, experience, read/forget both kinds.
- [x] Sequential fmt/check/clippy/tests/discipline/budget/deny/tui-smoke gates after review fixes.
- [ ] Windows all-target checks for every touched crate (toolchain deferrals below).

## Decisions and amendments

- The SDK estimator already exists; no M170 ContextPart is needed. M177 is absent, so the
  existing item pager gained text support. Its action shape is unchanged in this fix round.
- The original whole-body estimates overstated prompt use. Memory rows now measure their
  capped index lines, counting a multi-link line once. Experience uses the capped active index's render::line, also used by recall;
  retired/omitted/unreadable rows count zero. A cwd-only listing cannot infer extra recalled copies.
- The original file-first order could lose a memory on index failure. Index-first replacement is
  the safe side: unlink failure is reported after the pointer is gone. A hard-link test detects
  in-place index writes; temp collisions and deterministic concurrent edits exercise failure paths.
- One optimistic recheck/retry limits concurrent overwrite; a writer after the check can still win.
  No locking protocol with model tools is introduced. Unix syncs the renamed directory.
- Services trust wire cwd like CLI (ADR-0092). Existing Tokio moves from dev-only to normal use
  in experience for spawn_blocking; it adds no crate to the dependency tree.
- Confirmation uses typed owner services directly; commands have reply-parity tests. Cached open
  rows eliminate per-key/draw reads; current selection, not action arguments, authorizes the target.
- Native wire-free Pages are catalogue data, allowing immediate mod switches; actual owner
  services switch next start. No feature noun enters core or SDK.
- Overlay placement fits 80×24. Root Escape cancels confirmation without auto-closing a JS pane.
  Missing return panes never receive focus. Frontmatter is data, so fence it before Markdown.
- Exact interceptor prefilters avoid swallowing unrelated command errors. Bare ArgSpec::None
  preserves palette Enter behavior. Read titles and drawing use project/home-relative paths.
- Cursors clear on session end/disable. No editor, authoring, instruction deletion or other-project UI.
  Prompt snapshots may retain old material; the forget reply explains refresh and relearning.

## Verified — 2026-10-15

All checks used temporary HOME/data with credentials removed; terminal drives used only their
own `tmux -L … -f /dev/null` sockets and fake providers. Final sequential output summaries:
```text
cargo fmt --all -- --check: clean (exit 0)
cargo check --workspace --all-targets --locked: Finished dev profile in 44.30s
cargo clippy --workspace --all-targets --locked -- -D warnings: Finished in 37.75s
cargo test --workspace --locked: 5771 passed; 0 failed; 5 ignored (summed)
bash scripts/check_discipline.sh: discipline ok; no new function-length warnings
bash scripts/budget.sh: 415/415 dependencies; 0s core check; 0 relinked; budget ok
cargo deny check: advisories ok, bans ok, licenses ok, sources ok
bash scripts/tui-smoke.sh: tui-smoke ok (first run this round)
```
Budget's soft warning was 16 GB target/debug; final cargo clean removes build artifacts.
`BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk -p bingo-plugin-rpc -p bingo-surface-rpc
-p bingo-store-jsonl -p bingo-mods --locked --lib`: 464 passed, 1 ignored; generated files unchanged.
`node --test sdk/typescript/src/ui.test.ts`: 10 passed, 0 failed, including nonclosable root Escape.
Windows ran separately with `cargo check -p CRATE --all-targets --target x86_64-pc-windows-msvc --locked`:
services/context/experience/core/mod-memory (all bingo-prefixed): exit 0.
bingo-surface-tui and bingo: exit 101, aws-lc-sys missing stdlib.h/windows.h; release-toolchain
verification remains unticked. No Rust diagnostic preceded these C toolchain failures.
The original run's one smoke exit-hint timeout passed on rerun; no gate flake recurred this round.

Final isolated drive passed reading/forgetting both kinds, instruction refusal, live disable and
original-table fallback. Captures show whole-file bodies no longer inflate the 60-token estimate:
```text
80×24
 memory · about 60 tokens carried                               ⏎ read  d forget
 memories (2)
❯ project  feedback   no-sonnet-workers   Workers are Opus only         about 11
  user     user       role                Rust reviewer, prefers concis… about 4
 instructions (1)
  ~/.bingo/AGENTS.md                                                    about 26
 experience (1)
  01J9fixture  active  fixture before impl for wire formats  0 ok · 0 … about 19
108×30 experience pager (separate verbatim lines)
    status: active
    summary: fixture before impl for wire formats
    trigger: [wire]
    steps: [write fixture]
After both forgets: about 30 tokens carried; memories (1); experience (0).
```
Eight full/empty/confirming/malformed goldens and both frontmatter TestBackend sizes passed.
