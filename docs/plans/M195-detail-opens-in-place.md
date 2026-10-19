# M195 — Detail opens in place

ADR-0097 §5; ADR-0095 §4 (esc on an expanded view closes it, nothing else). Spec:
`docs/design/ember/`:
- `p4c-act.js`: `toggleLedger`, `foldLedger`, `ledgerKey`, `openFind`, `findKey`;
- `p5-stream.js`: `thoughtRows`, `wellRows` … `diffWell`;
- `p6-dock.js`: `findLines`, `findHits`, `markFinds`.

After: M193 and M194.

## Goal

`ctrl+o` brings a finished turn's thoughts, and what you added, back to where they happened.
↑↓ picks a row, ⏎ opens its well one shade deeper, and esc or `ctrl+o` folds it all again. A
well shows what the tool's owner drew. Find marks every match in the stream from the dock.

## Bricks

1. **Detail** (`toggleLedger`, `foldLedger`, `ledgerKey`, `rowItem`).
   - `ctrl+o` opens the newest finished turn that has steps or thoughts. If there is none, it
     toasts `no steps to show yet`.
   - The focus moves to the turn's rows: steps, plus thoughts while open. ↑↓ moves between
     them, and ⏎ or space opens or closes the picked row's well. esc or `ctrl+o` folds and gives
     the input back.
   - The picked row is kept in view (`keepSelVisible`). A click on any settled row opens its
     well, with or without detail.
   - The footer pairs are `↑↓ step`, `⏎ open`, `esc close`.
   - The selection lives in `ui` as an item id. Rows are derived each frame, never cached
     beside the transcript.
2. **Thoughts** (`thoughtRows`). In detail, each thought is a row in place: `≈` and `think` dim
   at 3 and 5, its first words dim italic at 13, and its length on the right from 1 s. Its
   well holds the whole thought, wrapped, muted italic. `Role::Thought` drawing outside detail
   goes.
3. **Wells** (`wellRows`, `wellFor`, `WELL_ROWS`).
   - A well is a block on the `well` ground from column 5 to the right margin, with one pad row
     above and below and text two cells in.
   - **Content:** the tool's `ToolOutput.display`, drawn by kind:
     - **Diff** (`diffWell`, `diffSegs`, `wordSpans`): no `---`/`+++`/`@@` header, line numbers
       read from the hunks, `+` good and `−` bad signs, rows on addTint/delTint, and a changed
       word on addWord/delWord.
     - **Code** (`codeWell`): numbered from where the read began.
     - **Text** (`outWell`): 12 lines. A command keeps its last 12, a lookup its first 12, with
       `… N earlier lines` or `… N more lines` dim. Lines matching error, failed or panicked
       read bad.
     - **Questions and answers:** each question muted, then its answer.
     - **Empty:** `no output`, or `did not run` for a skipped step.
   - The tier picks head or tail: a lookup keeps its head (`Tier::Lookup`).
4. **Read draws its code.**
   - `View::Code` gains `start: Option<u32>` and `of: Option<u32>`. Both are additive,
     skipped when absent; the schemas regenerate.
   - `Read` sets `display` to the lines it returned and the line they began at. The text the
     model reads is unchanged.
5. **The ledger scene.**
   - A turn that ended on a failed step opens with that step's well holding its last 4 lines
     (M194 draws it). In detail, ↑ and ⏎ move to an earlier step and open it.
   - What you added sits back where it landed, labelled `steered` or `said no` (`saidLines`
     in place).
6. **Find** (`openFind`, `findKey`, `findLines`, `findHits`, `markFinds`).
   - `ctrl+f` turns the dock into `find  ▍in this conversation`.
   - Typing searches the stream's drawn cells, case-insensitive, wide characters counted once.
   - Every match is marked; the current one is solid ember, and `N of M` or `no match` shows on
     the right.
   - ↑↓ moves to the previous or next match and centres it. ⏎ or `ctrl+f` closes the dock and
     stays there. esc closes it and goes back to where the stream was.
   - Today's `Focus::Search` (`search.rs`) keeps its matcher and moves into the dock.
7. **What goes:**
   - the pager (`pager.rs`) as the second `ctrl+o`;
   - the inline peek cuts (`transcript/output.rs` 12/5 rows with `… +N lines (ctrl+o to expand)`);
   - `Fold {Shut, Peek, Open}` where only a turn used it.

   Browse stays as it is. A plugin view the pager showed opens as a well instead.

## Files

- `crates/bingo-surface-tui/src/{input.rs,input/,keys.rs,fold.rs,pointer.rs,search.rs,pager.rs,
  transcript.rs,transcript/output.rs,transcript/thinking.rs,preview.rs,views/code.rs,ui.rs}`.
- New `src/well.rs` (wells only) and `src/detail.rs` (selection and keys).
- `crates/bingo-sdk/src/view/mod.rs` (`start`), `crates/bingo-tool-fs/src/read.rs`, regenerated
  schemas.
- `ui.rs` adds `Open::Reader` for memory pages, retaining their own layer.

## Exit criteria

- [x] `done-detail`, `done-well`, `done-folded`, `ledger`, `ledger-up`, `find`, `find-up` and
      `find-back` pass full-frame through the harness, keys driven as `index.json` lists them.
- [x] Unit tests:
  - well cuts: head 12, tail 12, the notes;
  - diff line numbers across two hunks;
  - word spans;
  - a find hit on a wide character;
  - the esc of find going back to the old scroll.
- [x] `Read` returns the same `parts` as before (its fixture test is unchanged), and its
      `display` carries `start`.
- [x] Gates and checks as in M192. tmux drive: `ctrl+o` on a done turn, ↑↓, ⏎ on a diff, a
      command and a read, esc; `ctrl+f`, typing, ↑, esc.

## Non-goals

Browse; selecting text to copy (M92 keeps it); wells for the agent or room rows (M200); a well's
own scroll (it shows 12 lines; the rest is in Browse).

## Risks

- **`ctrl+o` is a well-known key for the pager.** Its new meaning (detail) must keep a path to
  the whole output: Browse, named in the keys sheet (M198).
- **Find over drawn cells only finds what is drawn.** A folded thought is found only in detail.
  This is accepted, and the prototype does the same.

## Verified

Evidence in `docs/verification/m195/`: `gates.txt`, `contracts.txt`, `snapshots.txt`, `history.txt`,
`drive.log`, and the drive's text/ANSI captures.

- [x] Frames: `m195_full_frames_follow_the_prototype_keys ... ok` (done-detail/well/folded,
  ledger/up, find/up/back), keys as `index.json` lists them.
- [x] Units: well head/tail 12, two-hunk line numbers, word spans, a wide-character hit and find's
  esc going back all pass. The ledger test builds two failed steps and opens the first.
- [x] Read: the `parts` fixture is unchanged; `display` carries `start`/`of` and is sliced from the
  count the shared join kept. A new file's Write returns an all-added diff (fails before, passes after).
  Scene displays come from running the real Read, Write and Edit tools.
- [x] Scroll: one `Layout::below()`; the renderer writes no UI state. Detail pins once on open and
  closes into `Held`. `closing_detail_holds_the_drawn_place_when_more_rows_arrive` and
  `closing_detail_follows_when_the_folded_stream_fits` pass; the second fails without its fix.
- [x] Gates (`gates.txt`): fmt exit 0; check and clippy `-D warnings` finished; `cargo test
  --workspace --locked` 6571 passed, 0 failed, 0 ignored (TUI 1427, PTY 19); `discipline ok`;
  budget 423 of 423, `budget ok`; `cargo deny check` all ok.
- [x] Drive: `python3 docs/verification/m195/drive.py` gives `PASS`/`CLEAN` at 100x32 and 80x24
  (ctrl+o, up/down, enter on a diff, a command and a read, esc; ctrl+f, typing, up, esc).
  `scripts/tui-smoke.sh`: `tui-smoke ok`.
- [x] Windows: sdk and schedule `cargo check --target x86_64-pc-windows-msvc` exit 0.
- [ ] Windows tool-fs, TUI, Feishu, bin: owed to CI: aws-lc-sys headers (`stdlib.h`/`windows.h`
  are not on this machine).
- [x] History: a `rebase -x` fmt and check run over every commit (`history.txt`).
- Snapshots: 17 modified, 1 added, 5 deleted, each with a reason in `snapshots.txt`. Six are
  palette changes from the ctrl+o key row. The permission text snapshots are unchanged.

Departures:
- The diff renderer has one parser and one pairing pass. `DiffStyle` picks the look; `Well` adds
  the gutter and drops headers. The permission card shares the character-run emphasis, which
  removes one unchanged character from `inked_diff`'s emphasis.
- Find's esc gives back the whole `Scroll` it found, not only an offset.
- `message_tools` is drawn at 80x32 so the openroom row stays visible.
- Closing detail follows the tail when the folded stream reaches it, even if detail was scrolled
  up; otherwise it holds the drawn place.
- `dialog.expanded` stays, because ctrl+e and the permission preview use it. No dependency, merge or push.
