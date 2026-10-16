# M182 — What the drive found

ADR-0095, ADR-0084, M180. Bugs found driving the prototype's 32 scenarios
against the real TUI (2026-10-16), and the `/` lag the person reported. Each is
a bug fix; they share a plan because they share the drive that checks them.

## Goal

Typing after `/` costs what typing after `@` costs and never flickers; the ten
drive bugs are gone, each with a test that failed before the fix.

## Bricks

1. **`/` types locally.** A key that only edits the query (characters,
   backspace, cursor moves) edits the composer at once; the palette's pull for
   the new query is sent, not awaited. Only keys that act on the list (Enter,
   Tab, ↑↓, PgUp/PgDn) wait in `run/typeahead.rs` for the current query's list.
   `palette::waiting` stops rebuilding the full props on every check: it compares
   the query the last answer was drawn for.
2. **No fallback flash.** While a pull is pending the palette keeps the last
   answer's drawing (old rows retained until the new answer), never the
   names-only `palette/fallback.rs`; fallback is only for "no answer has ever
   arrived". The palette's height grows only when the row count exceeds the
   last height (and shrinks to fit a smaller terminal), so the header row does not jump on every key.
3. **Pager Esc** closes the pager and does nothing else; `q` and `ctrl+o` close
   it too (B1).
4. **Esc on a question or form** answers "skipped, it decides" and the turn
   goes on (ADR-0095 §4; `input/interaction.rs::cancel`) (B7).
5. **One receipt per answer**: the `⏺ Form` echo after `The user answered` goes
   (B8); `• 1 steps` reads `• 1 step` (`ask/receipts.rs:118`).
6. **Toasts never replay late**: toast clocks run on the wall clock while a pane
   holds; an expired toast is dropped (`toast.rs`).
7. **Catalogue cwd**: `bingo-core/src/host/catalog.rs` lists skills from the
   session's cwd, not `std::env::current_dir()`.
8. **Model card** shows "compacts first" only when the window is known and the
   kernel reports that this chat exceeds its effective switch limit
   (`bingo-mod-models/card.rs`).
9. **Picker cursor**: `*` unpin keeps the cursor on the row and toasts
   `unpinned <model>`; the hint reads `* unpin` on a pinned row; a picker opened
   with a filter shows its cursor row (B12, B19).
10. **`/login`** opens one pane deep; one Esc closes it (B13).

## Files

- `crates/bingo-surface-tui/src/{palette.rs,layout.rs,run/typeahead.rs,pulled.rs,toast.rs,input/interaction.rs,ask/receipts.rs}` and pager key routing.
- `crates/bingo-mod-models/src/{card.rs,label.rs,draw.rs,keys.rs,pins.rs,rows.rs,commands.rs}`.
- `crates/bingo-core/src/host/catalog.rs`, interaction receipts, `crates/bingo-tool-fs/src/ask.rs`, `crates/bingo-surface-tui/src/guide.md`, and isolated question/login smoke coverage, and `crates/bingo/tests/cli` headless skip assertions.
- `crates/bingo-sdk/src/tool.rs`, core executor call binding and `host/tool_host.rs`.
- Session catalogue API in sdk/core (`host/api.rs` keeps the host below its module budget), RPC params/client/server and generated schema; TUI catalogue refresh on view/attachment changes.

## Exit criteria

- [x] A `TestBackend`/typeahead test: ten query keys after `/` reach the composer without a reply from the mod; Enter waits for the list of the final query.
- [x] A tmux drive (fake provider, temp HOME, release build) under `stress`-style CPU load: per-key echo after `/` within 2× the `@` echo; painted bytes per key recorded before/after.
- [x] Captured frames between keys never show the fallback drawing once a list has arrived.
- [x] Each of bricks 3–10 has a test that fails on dev and passes here.
- [x] Native gates: fmt, check, clippy, test, discipline, budget, deny, tui-smoke.
- [ ] Windows all-target checks: four pass; tool-fs, TUI and binary remain deferred by the unavailable MSVC C toolchain (details below).

## Non-goals

The palette's new groups, wording and skills group (M187); the `KeyBar` (M184).

## Risks

- Local filtering must use the same matcher the mod uses, or rows shift when the real answer lands: reuse the mod's matcher from the sdk-level pure function, or keep the old rows unfiltered until the answer arrives.
- M180's guarantee (keys never lost, never into an ask) must hold: its tests stay green unchanged.
- Follow-up: bug 8's warning is hidden only in the picker (`bingo-mod-models/src/label.rs`); the kernel still publishes `switchAt` from the fallback window (`host/catalog/limits.rs:25`) and `model_switch.rs::needs_compaction` still compacts on it — move the known-window rule into the kernel later.

## Implementation decisions

- Keep the last palette rows unfiltered while awaiting the mod. Its matcher and selection stay with its owner; local filtering would duplicate both. List actions still wait for the current query and cannot activate those old rows.
- Keep the largest accepted palette height for each opening, bounded by the viewport. Padding shorter answers keeps the header steady while narrowing.
- Root Backspace and Escape retain the palette owner's close/back semantics; ordinary edits and horizontal cursor movement stay with the composer.
- Question/form Escape sends the existing `Cancel` answer without an interrupt. The receipt says `skipped · it decides`; the question tool returns a successful skip result so the model continues. Permission and sign-in refusal stay intact. Escape from a draft set aside returns to its ask, as ADR-0095 §4 requires.
- An interaction that does not offer `Cancel` cannot accept a skip. Preserve its interrupt-only escape route; do not invent an unsupported answer or trap focus.
- The extra Form block comes from a missing source item on tool questions. Bind the executor's call prompter to its existing item and carry it through `ToolHost::ask_for` (defaulting to `ask` for existing hosts). This repairs the receipt link for every tool without guessing from text or tool names. The TUI's existing linked-receipt rendering then owns the one receipt.
- Catalogue requests carry an optional session through the existing RPC method. Session requests use its cwd (including hook context); unscoped callers use configured host home. Changing the viewed session clears/refetches commands; existing request generations discard stale answers. No kernel feature noun.
- Toast deadlines ignore `holdToasts`; the existing wire field stays compatible. Only the alert-once set remains, with no paused copy of a deadline.
- Require a known positive context window for compaction copy. Keep the kernel's `compacts` decision (including output reserve and estimated input growth), rather than reimplementing its threshold using the card's displayed token count.
- Unpin selects the same model under its provider (revealing hidden rows when needed) and sends a three-second toast; hints derive pin/unpin from the current catalogue. ModelPicker cache identity includes viewport height so a smaller drawing keeps its selected row visible. The supplied settled filtered captures already show a cursor; the failing regression covers height reuse, rather than claiming those frames lack one.
- Bare `/login` starts at the provider list, preferring an unsigned provider, then the current provider, then the first. Enter opens its management sheet; Escape at the list closes the picker. Argument-based sign-in remains explicit.

## Verified — 2026-10-16

Latency drives used fake responses, temp HOME/data/cwd, and unique `tmux -L m182-* -f /dev/null` sockets. No credentials/live providers. Logs stayed in `/tmp/m182-verification`, outside git. No dependency or ember allowlist changes. The full suite initially hit an OAuth test assuming port 1 was unreachable (got HTTP success); it passed 1/1 with `NO_PROXY=localhost,127.0.0.1,::1`. Final suite/subsequent gates use that loopback bypass and remove credential/proxy environment variables; no global proxy settings changed.

Red-first output summaries, followed by each brick's sequential `cargo check -p … --all-targets --locked`, `cargo clippy -p … --all-targets --locked -- -D warnings`, and `cargo test -p … --locked` (all exit 0). Crate names below have the `bingo-` prefix except the binary `bingo`; totals include doc tests.

| Brick | Crates | Red failures | Green passed / ignored |
|---|---|---|---|
| 1 query echo / Enter waits | surface-tui | 1 | 1310 / 2 |
| 2 retained drawing / fixed header | surface-tui | 1 + 1 | 1311 / 2 |
| 3 pager closes without answering | surface-tui | 1 | 1312 / 2 |
| 4 skip / receipt / successful tool result | core, surface-tui, tool-fs | 1 + 1 + 1 | 2026 / 2 |
| 4 CLI follow-up | bingo | old assertion: 303 passed / 1 failed | 531 / 1 |
| 5 source-linked receipt / singular step | sdk, core, surface-tui | 1 + 1 | 2173 / 2 |
| 6 no expired-toast replay | surface-tui | 1 | 1314 / 2 |
| 7 session cwd / wire / view refresh | sdk, core, surface-rpc, surface-tui | 1 + 1 + 1 | 2237 / 3 |
| 7 API extraction | core | discipline: host.rs 1014 > 1000 lines | 631 / 0 |
| 7 catalogue script grouping | sdk | discipline: Scripts 17 > 16 fields | 228 / 0 |
| 8 unknown-window warning | mod-models | 1 | 13 / 0 |
| 9 unpin / hint / filtered height; hidden alias follow-up | mod-models, surface-tui | 2 + 1; 1 | 1331 / 2; 1333 / 2 |
| 10 one-Escape login | mod-models, surface-tui | 1 | 1332 / 2 |

`BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk -p bingo-plugin-rpc -p bingo-surface-rpc committed --locked`: exit 0, 7 passed. `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-mods schema_and_types_are_generated_from_the_same_rows --locked`: exit 0, 1 passed. Regenerated schema/TypeScript; only `schema/rpc.json` changes (optional session). Existing M180 typeahead tests pass unchanged. The first full smoke failed on its old plural `steps had run` assertion; the displayed `0 of 1 step had run` was correct, and the assertion now checks that exact singular receipt.

Release drives: `cargo build -p bingo --release --locked` exited 0 before/after. Same 120×40 fixture, `permission` after each opener, ten keys, 5 ms capture polling and 500 ms paint collection per key; echo includes tmux round-trip overhead. Load was ten owned Python busy loops, all stopped and waited for. No concurrent cargo commands during measurement. Idle means no added busy loops; ambient timing still varies.

| Drive | Key | Echo ms per key | Painted bytes per key |
|---|---|---|---|
| before-idle | @ | 15.7, 16.2, 14.8, 15.8, 16.7, 14.6, 15.8, 16.4, 16.0, 17.5 | 40, 80, 40, 195, 42, 1618, 42, 42, 42, 83 |
| before-idle | / | 58.8, 57.2, 60.8, 60.8, 62.5, 64.8, 60.3, 61.1, 59.8, 63.2 | 2256, 2553, 2812, 1579, 114, 115, 156, 117, 106, 147 |
| before-load | @ | 39.1, 27.5, 31.8, 24.4, 28.6, 46.6, 31.1, 38.4, 28.9, 30.6 | 40, 40, 40, 1771, 42, 42, 83, 42, 42, 42 |
| before-load | / | 87.7, 75.8, 125.1, 71.8, 87.6, 96.7, 83.6, 112.7, 75.6, 85.8 | 2296, 2553, 2772, 1620, 114, 115, 115, 125, 106, 106 |
| after-idle | @ | 17.3, 17.1, 17.9, 37.0, 40.0, 22.6, 16.9, 17.3, 41.9, 46.9 | 40, 80, 40, 195, 42, 1618, 42, 42, 42, 83 |
| after-idle | / | 56.9, 32.8, 19.6, 17.3, 18.1, 32.6, 28.5, 17.8, 17.6, 18.8 | 2224, 2521, 2537, 1595, 81, 82, 123, 84, 73, 73 |
| after-load | @ | 70.5, 31.3, 37.1, 25.9, 34.0, 86.3, 27.2, 31.7, 79.3, 40.4 | 40, 40, 1520, 236, 42, 42, 83, 42, 42, 42 |
| after-load | / | 32.3, 34.8, 26.6, 57.3, 81.2, 56.3, 25.8, 34.5, 32.1, 39.2 | 2264, 2521, 2497, 1636, 81, 82, 82, 125, 73, 73 |
| final-idle | @ | 31.7, 59.0, 43.3, 59.3, 45.9, 68.5, 57.4, 43.4, 51.6, 50.2 | 40, 80, 40, 195, 1618, 42, 42, 42, 83, 42 |
| final-idle | / | 66.1, 39.9, 45.1, 66.9, 39.3, 69.4, 53.5, 39.9, 55.5, 50.7 | 2224, 2561, 2497, 1595, 81, 123, 82, 84, 73, 114 |
| final-load | @ | 23.8, 43.5, 56.3, 34.4, 46.7, 53.9, 50.7, 46.3, 56.5, 51.2 | 40, 40, 1560, 195, 42, 42, 83, 42, 42, 42 |
| final-load | / | 50.8, 56.9, 58.1, 43.1, 32.7, 61.3, 36.7, 25.2, 42.4, 48.8 | 2224, 2521, 2497, 1636, 81, 82, 123, 84, 73, 73 |

Final median echo `/` vs `@`: idle **52.1/50.9 ms (1.02×)**; loaded **46.0/48.7 ms (0.94×)**. Final maxima: idle **69.4/68.5**, loaded **61.3/56.5 ms**; every final `/` sample is below 2× its same-run `@` median. Before medians were **60.8/15.9** idle and **86.7/30.9** loaded. The earlier after-runs are retained above: absolute echo varies, so compare the same-run reference. Mean painted bytes `/` vs `@`: before idle **995.5/222.4**, loaded **992.2/218.4**; final idle **943.4/222.4**, loaded **939.4/212.8**. This isolated release baseline did not reproduce the audit's ~13 KB/~900 ms; these measurements do not replace that evidence.

All 40 captured after/final frames retain rich palette text; its header stays at zero-based row 16 (before: rows 16, 20, 30). The delayed-response TestBackend regression also checks retained rich rows, fixed height and cursor movement while the current query has no answer.

Final native command output summaries (run sequentially; shell-only smoke assertion updated after its red run):

```text
cargo fmt --all -- --check: exit 0
cargo check --workspace --all-targets --locked: Finished dev profile; exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings: Finished dev profile; exit 0
cargo test --workspace --locked: 6247 passed; 0 failed; 6 ignored (197 groups); exit 0
bash scripts/check_discipline.sh: discipline ok; exit 0 (advisory warnings remain)
bash scripts/budget.sh: 422/422 deps; warm core check 0s/20s; 0 TUI-to-core recompiles; budget ok; exit 0
cargo deny check: advisories ok, bans ok, licenses ok, sources ok; exit 0
bash scripts/tui-smoke.sh: tui-smoke ok; exit 0 (233.33s on rerun)
```

Budget reported target/debug 15 GB above its 5 GB soft limit; the final action after committing this record is `cargo clean`, with its output reported at delivery. The HostApi advisory points at the production facade moved unchanged into `host/api.rs`, not a new test double.

Windows commands ran separately and sequentially: `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc --locked`.

- `bingo-sdk`, `bingo-core`, `bingo-surface-rpc`, `bingo-mod-models`: `Finished dev profile`, exit 0 for each.
- `bingo-tool-fs`, `bingo-surface-tui`: exit 101 in `aws-lc-sys 0.44.0`, C compiler reports `stdlib.h` / `windows.h` not found.
- `bingo`: exit 101 in `rquickjs-sys 0.14.0` (MSVC C flags passed to the host compiler) and `aws-lc-sys` (same missing Windows headers). These are the allowed C-toolchain deferrals; those crates are not claimed Windows-verified.

## Verified — bug 6 fix round, 2026-10-16

The stale replay was status-line notices (`ui.rs`), not toasts: `Ui::say` queued every notice and showed each for ~4 s. Now one `Option<Notice>` slot, latest wins, life measured from `say` on the wall clock; the queue is deleted. Tests: `rapid_notices_leave_only_the_last_one_said`, `a_notice_whose_moment_passed_behind_another_is_never_said`, `a_notice_expires_by_the_wall_clock_from_when_it_was_said`. The `toast.rs` change (f7e7238f) stays: pausing toast clocks behind a `holdToasts` pane replayed them after it closed. Drive: release build, `tmux -L m182fix -f /dev/null`, temp HOME, fake provider, five BTab 150 ms apart, status line sampled each second:

```text
 0.0s permission mode: default    1.1s same    2.1s same    3.1s same
 4.1s ? for shortcuts             5.2s–8.3s ? for shortcuts (nothing replayed)
```

