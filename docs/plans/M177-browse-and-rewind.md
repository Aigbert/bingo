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

1. **One turn walk.** `crates/bingo-sdk/src/transcript/turns.rs:: turns(&Transcript) -> Vec<Turn { id, asked, first, last }>` on
   `bingo_sdk::asked` (M175). `crates/bingo-checkpoints/src/turns.rs` and `crates/bingo-surface-tui/src/rewind.rs::turns` are deleted and call
   it; their tests move with it (ADR-0045's recorded debt paid).
2. **Find in items, pure.** `crates/bingo-sdk/src/transcript/find.rs:: find(&Transcript, query) -> Vec<Match { item, turn, line }>` over each
   item's full text, case-folded (as `search.rs`). Table test: a match in a folded tool result, in reasoning, none in a dropped (rewound) item.
3. **What a rewind takes, contract first.** `crates/bingo-services/src/ checkpoints.rs`: `touched(session, from: TurnId) -> Touched { files: [{
   path, fate: restored | removed | skipped }], shell_calls }`, `find(host)` native-or-wire, fixtures `fixtures/checkpoints/` first.
   `bingo-checkpoints` registers it over `restore::plan` (read only; the plan is never applied by the service) and counts `Bash` calls in the
   undone turns.
4. **The mode.** `/rewind <turn> [both|conversation|files]` in `crates/bingo-checkpoints/src/command.rs`: `both` today's order (files, then
   `host.rewind`); `conversation` only `host.rewind`, reply says the files were kept; `files` only `restore::apply`, journal untouched. Unknown
   mode is `INVALID_INPUT`. Test per mode over a two-turn session with one `Write` and one `Bash`.
5. **The sheet, pure.** `crates/bingo-mod-browse/src/sheet.rs::sheet( turns, touched, mode) -> Sheet` rows and the line its `⏎` submits. Table
   test incl. no files, all skipped, shell calls only.
6. **Props.** `Browse { session, selected, query, matches }` in `crates/bingo-sdk/src/component/{browse.rs,props.json}` and `components!`; golden
   contract; schema and TypeScript regenerated. The surface fills `matches` from brick 2 and peeks a folded block that holds the current match
   (`browse/paint.rs`), deriving a temporary fold map while open; the saved fold never changes.
7. **Keys.** `crates/bingo-mod-browse/src/keys.rs` through `onKey`: `[` `]` → select previous/next turn (scroll to its first item), `n` `N` step,
   `⏎` → `Pager { item }` surface action (`pager.rs::open`), `y` → copy the block's text (the surface's clipboard path, `select.rs`), `r` → load
   `touched`, open the sheet; in the sheet `←→` mode, `⏎` submits `/rewind <turn> <mode>`, `esc` back. Table test (state, key) → outcome.
8. **The card asks too.** `crates/bingo-surface-tui/src/input.rs:464-488`: the `esc esc` card's `⏎` opens the same sheet when the mod is on
   (`Browse` placed); with it off the card submits as today.
9. **The prompt comes back.** After `Rewound` for this session's own `/rewind`, the surface puts the undone turn's `asked` into the composer
   through the `Composer` action (M170), unless a draft is there (toast).
10. **Opener.** `/browse` in the mod, `keys: ["ctrl+f"]`; it appears in `?` from the catalogue. With the mod off `input.rs::layered` keeps
    `ctrl+f` → `Search::open()`.
11. **Drawing.** `draw.rs`: the top bar and the sheet; the transcript stays the surface's drawing with the turn rail and marks. Snapshots at
    108×30 and 80×24: walking, searching, sheet in each mode.
12. **Guide.** `guide.md` (<200 lines) by `guide.rs`, as in `bingo-mod-palette`.

## Files

- sdk: `asked.rs`, `transcript.rs`, new `transcript/{turns.rs,find.rs}`, `component/{browse.rs,props.json,props.rs}`, `component.rs`,
  `render.rs`; `schema/`, `sdk/typescript/`.
- `crates/bingo-services/src/{lib.rs,checkpoints.rs}` and fixtures.
- `crates/bingo-checkpoints/src/{lib.rs,command.rs,turns.rs (deleted)}`.
- TUI: `rewind.rs`, `input.rs`, `browse/{paint,actions}.rs`, `pager.rs`, `run/surface.rs` (`Pager` action), site placement for `Browse`.
- New `crates/bingo-mod-browse/src/{lib.rs,sheet.rs,keys.rs,draw.rs, guide.rs,guide.md}`; listed in `crates/bingo/src/main.rs`; +1 member in
  `scripts/budget.toml`, no dependency. ADR-0045, ADR-0084 and ADR-0086 gain their amendment lines.

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
- [ ] For each touched crate (`bingo-sdk`, `bingo-services`, `bingo-checkpoints`, `bingo-mod-browse`, `bingo-surface-tui`, `bingo`):
      `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc --locked`: four pass; TUI/bin deferred by the C toolchain below.

## Decisions

- M175 is not merged: add the pure `asked` brick here, returning the full typed text; labels take its first line. This preserves multiline
  prompts on rewind.
- M170 is not merged: add the generic Composer action in the TUI here; do not edit the status mod. Surface actions remain existing named `Action`
  receipts.
- Props also carry the projected turn rows, current match, confirmation opener and a minted client action, as Palette does. They contain no
  second transcript. Selection/query live only in the mod; the surface reads its accepted drawing to derive search matches, scroll position and
  temporary folds. A temporary map avoids storing a second copy of the saved folds just to restore it on close.
- `asked` preserves multiline input, takes only the typed line of an expanded command, and excludes injected context and non-spoken producer
  voices.
- PTY captures showed absolute paths hiding filenames at 80 columns; the mod displays paths relative to the session cwd. The service retains full
  paths.
- Search commits on Enter; `/` edits a new query. This disambiguates `n`, `r` and `y` from search text and avoids scanning every item per
  keystroke.
- File lists scroll with ↑↓/PgUp/PgDn; warnings and mode choices stay visible.
- A queued rewind keeps its own intent until completion, so its prompt returns even when it waited behind a running turn. Withdrawing it retires
  that intent only after a successful withdrawal; a failed withdrawal leaves the running rewind identifiable.
- Guides use `crates/bingo-mod-browse/src/guide.md` and its registered Page. Only existing workspace edges are added; the member budget grows 414
  → 415. The checkpoints service makes its switch take effect at next start (M173); no M166 or M173 crate changes are needed.
- A synchronous answer after drawing schedules another frame: the smoke exposed a committed query whose second projection otherwise stayed invisible.
- Key waiting uses the same render predicate as placement; switching off or narrowing a replacement cannot strand queued keys.

## Non-goals

Rewinding a child agent or a room; exporting a transcript; tracking what a shell command wrote; a rewind of only some files.

## Risks

- `conversation` mode leaves files the transcript no longer explains; the sheet names it and the `Rewind` item's reply says so.
- Searching full item text of a long session on each keystroke can lag: Browse commits search on `⏎`.
- Peeking a folded block shifts the scroll; the selection is kept by item id, never by row.

## Verified — 2026-10-15

All runtime tests used local/fake fixtures with provider credentials removed. Gates ran sequentially; no wall-clock flake occurred. Initial failures were fixed: legacy one-Enter PTY expectation, discipline layout, and the synchronous search redraw (regression red → green).

```text
cargo fmt --all -- --check → exit 0
cargo check --workspace --all-targets --locked → Finished dev in 4.76s
cargo clippy --workspace --all-targets --locked -- -D warnings → Finished dev in 5.13s
RUST_TEST_THREADS=4 cargo test --workspace --locked → 5746 passed; 0 failed; 5 ignored (173 suites)
bash scripts/check_discipline.sh → discipline ok (existing advisory warnings)
bash scripts/budget.sh → dependencies 415/415; warm core check 1s; TUI→core recompiles 0; budget ok
cargo deny check → advisories ok, bans ok, licenses ok, sources ok
bash scripts/tui-smoke.sh → tui-smoke ok
(cd sdk/typescript && npm test) → tests 31; pass 31; fail 0
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk -p bingo-plugin-rpc -p bingo-surface-rpc -p bingo-store-jsonl -p bingo-mods --locked --lib → 467 passed; 0 failed; 1 ignored
cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc --locked:
  bingo-sdk / bingo-services / bingo-checkpoints / bingo-mod-browse → Finished dev (each exit 0)
  bingo-surface-tui → aws-lc-sys: stdlib.h / windows.h not found (exit 101)
  bingo → rquickjs-sys: host clang rejects MSVC flags; aws-lc-sys: Windows headers missing (exit 101)
```

Isolated tmux: seven scenarios passed, 20 captures; three Write+Bash turns, all modes at 108×30 and 80×24, plus disabled-mod line search/direct rewind.
Both/files left `note.txt` at `edit 1`; conversation kept `edit 3`; both/conversation restored `prompt 2`. Cropped capture excerpts (same rows at both sizes):
```text
browse · turn 2 of 3 · "" 0/0 (n/N)  [ ] turn  ⏎ page  y copy  r rewind
│ undoes turns 2–3 · 8 items
│ restored note.txt
│ ! files a shell command wrote are not tracked (2 Bash calls)
│ ❯ both    conversation    files    ⏎ rewind  esc
│ both    ❯ conversation    files    ⏎ rewind  esc
│ both    conversation    ❯ files    ⏎ rewind  esc
```
The budget's 10 GB debug tree exceeded its 5 GB soft limit; the final `cargo clean` removes this worktree's build artifacts.
