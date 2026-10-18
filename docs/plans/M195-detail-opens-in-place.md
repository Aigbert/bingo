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
   - `View::Code` gains `start: Option<u32>`. It is additive and skipped when absent; the
     schemas regenerate.
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

## Exit criteria

- [ ] `done-detail`, `done-well`, `done-folded`, `ledger`, `ledger-up`, `find`, `find-up` and
      `find-back` pass full-frame through the harness, keys driven as `index.json` lists them.
- [ ] Unit tests:
  - well cuts: head 12, tail 12, the notes;
  - diff line numbers across two hunks;
  - word spans;
  - a find hit on a wide character;
  - the esc of find going back to the old scroll.
- [ ] `Read` returns the same `parts` as before (its fixture test is unchanged), and its
      `display` carries `start`.
- [ ] Gates and checks as in M192. tmux drive: `ctrl+o` on a done turn, ↑↓, ⏎ on a diff, a
      command and a read, esc; `ctrl+f`, typing, ↑, esc.

## Non-goals

Browse; selecting text to copy (M92 keeps it); wells for the agent or room rows (M200); a well's
own scroll (it shows 12 lines; the rest is in Browse).

## Risks

- **`ctrl+o` is a well-known key for the pager.** Its new meaning (detail) must keep a path to
  the whole output: Browse, named in the keys sheet (M198).
- **Find over drawn cells only finds what is drawn.** A folded thought is found only in detail.
  This is accepted, and the prototype does the same.
