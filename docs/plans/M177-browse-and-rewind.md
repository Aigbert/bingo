# M177 — Browse and rewind

ADR-0091, ADR-0045, ADR-0084, ADR-0086. Needs M167, M175 (`asked`); M170 for the `Composer` surface action.

## Goal

Read the conversation by turns and go back to one knowing what goes back. `ctrl+f` opens the `Browse` site over the transcript: `[` `]` walk
turns, typing searches every block including what is folded, `n` `N` step matches, `⏎` opens a block in the pager, `y` copies it, `r` rewinds to
the selected turn. Rewind, from Browse or the `esc esc` card, first shows a sheet: the turns undone, each file and its fate, that what a shell
command wrote is not tracked, and `both | conversation | files`. After a rewind the prompt that opened the turn is back in the composer. The
feature is the built-in mod `bingo-mod-browse`; with it off, `ctrl+f` is the terminal's line search as today.

```
 browse · turn 4 of 9 · "fork"  2/5 (n/N)        [ ] turn  ⏎ page  y copy
 ...transcript, the selected turn's rail lit, matches marked, a folded
    block holding a match peeked open in place...
 ┌ rewind to turn 4 ─────────────────────────────────────────────────┐
 │ undoes turns 4–9 · 31 items                                       │
 │   restored  crates/bingo-core/src/fork.rs                         │
 │   removed   crates/bingo-core/src/fork/tests.rs                   │
 │   skipped   assets/big.bin (too large)                            │
 │ ! files a shell command wrote are not tracked (2 Bash calls)      │
 │ ❯ both    conversation    files                  ⏎ rewind  esc    │
 └───────────────────────────────────────────────────────────────────┘
```

## Bricks

1. **One turn walk.** `crates/bingo-sdk/src/transcript/turns.rs:: turns(&Transcript) -> Vec<Turn { id, headline, first, last }>` on
   `bingo_sdk::asked` (M175). `crates/bingo-checkpoints/src/turns.rs` and `crates/bingo-surface-tui/src/rewind.rs::turns` are deleted and call
   it; their tests move with it (ADR-0045's recorded debt paid).
2. **Find in items, pure.** `crates/bingo-sdk/src/transcript/find.rs:: find(&Transcript, query) -> Vec<Match { item, turn, line }>` over each
   item's full text, case-folded (as `search.rs`). Table test: a match in a folded tool result, in reasoning, none in a dropped (rewound) item.
3. **What a rewind takes, contract first.** `crates/bingo-services/src/ checkpoints.rs`: `touched(session, from: TurnId) -> Touched { files: [{
   path, fate: restored | removed | skipped, changed }], shell_calls }`, `find(host)` native-or-wire, fixtures `fixtures/checkpoints/` first.
   `bingo-checkpoints` registers it over `restore::paths` (read only; the plan is never applied by the service) and counts `Bash` calls and `Shell` items in the
   undone turn ranges, including items without a turn id.
4. **The mode.** `/rewind <turn> [both|conversation|files]` in `crates/bingo-checkpoints/src/command.rs`: `both` today's order (files, then
   `host.rewind`); `conversation` only `host.rewind`, reply says the files were kept; `files` only `restore::apply`, journal untouched. Unknown
   mode is `INVALID_INPUT`. Test per mode over a two-turn session with one `Write` and one `Bash`.
5. **The sheet, pure.** `crates/bingo-mod-browse/src/sheet.rs::sheet( turns, touched, mode) -> Sheet` rows and the line its `⏎` submits. Table
   test incl. no files, all skipped, shell calls only.
6. **Props.** `Browse { session, selected, query, hit, count }` in `crates/bingo-sdk/src/component/{browse.rs,props.json}` and `components!`; golden
   contract; schema and TypeScript regenerated. The surface caches brick 2 by transcript length/query (plus session/generation) and peeks a folded block that holds the current match
   (`browse/paint.rs`), deriving a temporary fold map while open; the saved fold never changes.
7. **Keys.** `crates/bingo-mod-browse/src/keys.rs` through `onKey`: `[` `]` → select previous/next turn (scroll to its first item), `n` `N` step,
   `⏎` → `Pager { item }` surface action (`pager.rs::open`), `y` → copy the block's text (the surface's clipboard path, `select.rs`), `r` → load
   `touched`, open the sheet; in the sheet `←→` mode, `⏎` submits `/rewind <turn> <mode>`, `esc` back. Table test (state, key) → outcome.
8. **The card asks too.** `crates/bingo-surface-tui/src/input.rs:464-488`: the `esc esc` card's `⏎` opens the same sheet when the mod is on
   (`Browse` placed); with it off the card submits as today.
9. **The prompt comes back.** After `Rewound` for this session's own `/rewind`, the surface puts the undone turn's `asked` (read from the transcript) into the composer
   through the `Composer` action (M170), unless a draft is there (toast).
10. **Opener.** `/browse` in the mod, `keys: ["ctrl+f"]`; it appears in `?` from the binding table. With the mod off `input.rs::layered` keeps
    `ctrl+f` → `Search::open()`.
11. **Drawing.** `draw.rs`: the top bar and the sheet; the transcript stays the surface's drawing with the turn rail and marks. Snapshots at
    108×30 and 80×24: walking, searching, sheet in each mode.
12. **Guide.** `guide.md` (<200 lines) by `guide.rs`, as in `bingo-mod-palette`.

## Files

- sdk: `asked.rs`, `transcript.rs`, new `transcript/{turns.rs,find.rs}`, `component/{browse.rs,props.json,props.rs}`, `component.rs`,
  `render.rs`; `schema/`, `sdk/typescript/`.
- core: `session.rs`, `session/title.rs`, `turn.rs` and their regression tests.
- `crates/bingo-services/src/{lib.rs,checkpoints.rs}` and fixtures.
- `crates/bingo-checkpoints/src/{lib.rs,command.rs,after.rs,preview.rs,restore.rs,hook.rs,store.rs,turns.rs (deleted)}`.
- TUI: `rewind.rs`, `input.rs`, `browse/{cache,paint,actions}.rs`, `pager.rs`, `run/surface.rs` (`Pager` action), site placement for `Browse`.
- New `crates/bingo-mod-browse/src/{lib.rs,sheet.rs,keys.rs,draw.rs, guide.rs,guide.md}`; listed in `crates/bingo/src/main.rs`; +1 member in
  `scripts/budget.toml`, no external dependency. ADR-0045, ADR-0084 and ADR-0086 gain their amendment lines.

## Exit criteria

- [x] `turns`, `find`, `sheet` and keys tables; checkpoints fixtures.
- [x] Mode test: `conversation` keeps the file, `files` keeps the journal, `both` restores then rewinds; a failing write stops before the journal
      (ADR-0045 §3, existing test still green).
- [x] `TestBackend`: `ctrl+f` opens Browse; `]` lands on the next turn; a match inside a folded block is peeked and folded again on `esc`; `r`
      shows the Bash line; `⏎` in the sheet rewinds and the composer holds the prompt; `esc esc` card shows the sheet.
- [x] `enabledPlugins: {"bingo.mod.browse": false}`: `ctrl+f` is line search, the card submits directly.
- [x] tmux drive on an isolated socket, temp HOME, fake provider scripted to `Write` a file and run `Bash` over three turns; rewind each mode;
      captures pasted.
- [x] `bash scripts/tui-smoke.sh`; `cargo fmt --all -- --check`; `cargo check --workspace --all-targets --locked`; `cargo clippy --workspace
      --all-targets --locked -- -D warnings`; `cargo test --workspace --locked`; `scripts/check_discipline.sh`; `scripts/budget.sh`; `cargo deny
      check`.
- [ ] For each touched crate (`bingo-sdk`, `bingo-core`, `bingo-services`, `bingo-checkpoints`, `bingo-mod-browse`, `bingo-surface-tui`, `bingo`):
      `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc --locked`: five pass; TUI/bin deferred by the C toolchain below.

## Decisions (amended after independent review)

- `asked(&Item) -> Option<&str>` borrows the first Text part when the effective voice is Said: an explicit voice wins; unstamped is Said only
  from `PERSON_SURFACES` (tui, print, rpc, acp, channels) and Quiet from anything else, so an unknown producer fails closed (no legacy table).
  Core titles, TUI voice and turn labels share the SDK bricks. Core stamps what it adds Quiet (context, length recovery, Stop-hook reasons).
- M170 is not merged: the TUI carries the generic Composer action; no status/model/plugins-panel crate is edited.
- Props carry one current hit and count, never all matches, and each turn's headline (first line, ≤200 chars), never the whole prompt.
  Turns/search cache by session, length, history generation and the transcript's revision (any folded change, an in-place edit included).
  Accepted props deserialize once per drawing version. Find stops at 1000 and shows `1000+`; Write input preserves logical lines.
- Enter commits a query and starts at its first hit; empty-query `n` types. Escape clears query/editing first, then closes.
- A preview pins its turn and transcript length. New items force re-preview; busy sessions refuse. Confirmation is one-shot once the sheet exists.
  Draw never previews or reads files; a frame never says busy, only `r`/`⏎` do, once. Run/close/pager do not invalidate; `touched` answers a
  missing turn with a typed `None` (no sheet), an error stays an error. Rewinding the last turn clears the fold's `last_turn` (its Worked line).
- `restore::paths` reads metadata only. Post-turn hashes use the existing `bingo-digest` workspace library edge (no external dependency);
  preview marks later edits, missing/unreadable/over-limit hashes warn conservatively, and confirmation still applies. Hash reads retain the
  8 MiB snapshot bound because the existing digest takes a complete byte slice; unknown hashes never count as equal.
- The checkpoint wire exposes only safe `touched`; no writing service method exists to mark unsafe. `apply` is rejected by a fixture test;
  ADR-0091 requires `unsafe_call: true` for any future wire write. Actual writes remain `/rewind` commands.
- Mode rows say files kept / no turns undone truthfully; the summary row is the drawer's alone. Reports return lines; the TUI status joins them with ` · `.
- Prompt text is captured at submit, keyed by session/target turn, independently of Applied acknowledgement; rejection/withdrawal retires it.
  This replaces keeping queued intents alive solely for refill. Existing drafts remain intact.
- Pager `return_to` preserves Browse query/position; one opener handles both entry points. Only Browse uses `whole`; copy uses readable rendering.
- The mod displays relative paths, scrolls files with ↑↓/PgUp/PgDn, and leaves warnings/mode choices visible at 80 columns.
- Guides live in `crates/bingo-mod-browse/src/guide.md`, registered by `guide.rs` as a Page (<200 lines), never `docs/guide/`.
- Empty-query bars show only position; single-turn sheets use the singular. Fixtures retain their existing Unicode.
- AGENTS adds the `checkpoints` commit scope. Existing workspace edges only; member budget 414 → 415. Checkpoints switch at next start. No ember allowlist changes.
- Disabled fixtures set the real flag and its published empty render table; smoke tests the actual switch in migrated TOML settings. Browse caches live with placement caches.
- Synchronous render answers schedule another frame; render predicates govern key waiting so disabled/narrow replacements cannot strand keys.

## Non-goals

Rewinding a child agent or a room; exporting a transcript; tracking what a shell command wrote; a rewind of only some files.

## Risks

- `conversation` mode leaves files the transcript no longer explains; the sheet names it and the `Rewind` item's reply says so.
- Search is bounded and cached; the cap deliberately reports `1000+` even when exactly 1000 lines match.
- Peeking a folded block shifts the scroll; the selection is kept by item id, never by row.

## Verified — 2026-10-15

First round (gates run individually; provider env cleared): fmt/check/clippy exit 0; workspace tests 5759 passed, 0 failed, 5 ignored;
discipline ok; budget ok (415/415); `cargo deny check` ok; tui-smoke ok; schema regen PASS; TypeScript 31/31. Windows check:
sdk/core/services/checkpoints/mod-browse exit 0; surface-tui/bingo exit 101 on `aws-lc-sys` `stdlib.h` (accepted C toolchain deferral).
tmux, temp HOME, fake Write+Bash: six mode/size drives and the disabled drive passed (`undoes turns 2–3 · 8 items`, `restored note.txt ·
changed since the turn`, `! files a shell command wrote are not tracked (2 Bash calls)`).

Fix round (`m177-fix`): busy said only from keys; unstamped voice fails closed; data-file noun scan; typed `None` for a missing turn;
rewound last turn clears `last_turn`; one summary row; headline-only turns; transcript revision in the cache key; Quiet stamps pinned.
```text
cargo fmt --all -- --check: fmt exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings: clippy exit 0
RUST_TEST_THREADS=4 cargo test --workspace --locked: test exit 0; 173 suites, 5773 passed, 0 failed, 5 ignored
scripts/check_discipline.sh: discipline ok · scripts/budget.sh: budget ok (target/debug 12 GB soft warning, cleaned)
cargo deny check: advisories ok, bans ok, licenses ok, sources ok · node --test: tests 31, pass 31, fail 0
scripts/tui-smoke.sh ×3: tui-smoke ok, tui-smoke ok, tui-smoke ok
tmux -L m177r2, temp HOME, fake provider, 108x30 and 80x24, browse open over a 6 s turn:
  busy notices before a key: 0 · after one r: 1 ("Wait for the running turn before rewinding")
  "▍ Worked for 6s · done" → rewind turn 2 (sheet: one "undoes turn 2 · 2 items") → worked lines after rewind: 0; composer "> two"
```
Older journals: unstamped room/wake/`plugin:*` user lines now draw as quiet notices; plugin-host deliveries without a voice are Quiet.
Rebased onto M175 (dev 4f13c12e), one effective voice shared by `asked`/`first_ask` and the sessions auto-title: fmt, clippy, test (177 suites, 5817 passed, 0 failed, 5 ignored), schema regen clean, node 31/31, discipline, budget (416/416), deny, tui-smoke ×3 all ok.
