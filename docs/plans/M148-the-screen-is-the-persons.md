# M148 — The screen is the person's

## Goal

[tui-identity-2026-10.md](../design/tui-identity-2026-10.md) §2 and C5,
[ADR-0081](../adr/0081-the-screen-is-the-persons.md). The rail and the
`ctrl+p` sheet — what the person called "the board" — go, by the
person's word of 2026-10-13. The transcript takes the width at every
size. A plugin is seen in the TUI only where it placed itself; the
in-tree producers that relied on the legacy lanes place themselves or
become data. No new dependency.

## Bricks, in build order

1. **Failing tests first**:
   - sdk `state.rs`: `extended(.., Value::Null)` removes the kind, like
     `signalled`; a fixture journal with an extension then a `Null` folds
     to no entry (ADR-0081 §5).
   - TUI `frame.rs`: at 200 columns a legacy panel payload, a live
     signal, a `Slot::Rail` and a `Slot::Panel` each leave the transcript
     its full width and draw nothing beside it.
   - TUI `keys.rs`: `ctrl+p` is unbound; the `?` sheet has no `ctrl+p`.
   - demo-ui: `/board` places a `Row` whose body is the table and whose
     actions are Tick and Reset; the progress tool places an `Activity`.
   - rooms: the `owed` signal and the `members` extension carry
     `slots: []`; `seats.rs`'s readers still read both by name.
   - TUI: a session with only `owed`, `members` and a schedule `wake`
     draws nothing for them at any width.
2. **The reducer** (sdk `state.rs` `extended`): `Null` removes. Check
   every `extensions` reader for a `Null` it relied on (the schedule's
   wake, `bingo-schedule/src/wake.rs`) and list them.
3. **Subtract** (`frame.rs`, `rail.rs`, `panel.rs`, M124's layout,
   `pointer.rs`, `keys.rs`, `view.rs`, `status.rs`): the rail, its
   threshold and width, pins, the panel sheet and its key, the legacy
   lanes' drawing and the under-the-running-rows lane. Every caller of
   what goes is listed; nothing is kept "for later".
4. **The producers move.** demo-ui (`board.rs`, `command.rs`,
   `journal.rs`, `progress.rs`, `lib.rs` module header): the board as a `Row`, the
   progress as an `Activity`, placed through ADR-0070's `slot::with`.
   rooms (`owed.rs`, delegating to `bingo-collab/src/owed.rs`; the
   `members` payload writer in `bingo-collab/src/room.rs`)
   and schedule (`wake.rs`): `slots: []`.
5. **Records.** ADR-0081 accepted; dated notes on ADR-0013 §2 §4,
   ADR-0070 §1 and ADR-0079 §3; tui.md §3 (*Rail* removed), §8's lane
   table, §10 dated entry; the guide pages that name `ctrl+p` or the
   rail.

## Files

- `crates/bingo-sdk/src/state.rs` and its fixtures
- `crates/bingo-surface-tui/src/{frame,rail,panel,layout,pointer,status,keys,view}.rs`
  (`rail.rs` and `panel.rs` deleted; `layout.rs` is M124's)
- TUI removal callers: `src/{lib,ui,focus,input,layers,test_lanes,window}.rs`,
  `src/layout/tests.rs`, `src/screens{,/teams,/windows}.rs`, and their snapshots
- `crates/bingo-demo-ui/src/{board,command,journal,progress,lib,tests}.rs`
- `crates/bingo-rooms/src/{owed,roster}.rs`, `crates/bingo-collab/src/{room,owed}.rs`
- `crates/bingo-schedule/src/wake.rs`
- `crates/bingo-context/src/baseline/lifecycle.rs` (approved Null-absence assertion)
- `scripts/tui-smoke.sh`, the guide pages naming `ctrl+p` or the rail
- `docs/adr/{0013,0070,0079,0081}-*.md`, `docs/design/tui.md`,
  `docs/design/tui-identity-2026-10.md`

## Exit criteria

- [ ] Each brick-1 test failed first (output pasted) and passes now.
- [ ] `rg 'RAIL_AT|RAIL_WIDTH|ctrl\+p' crates/bingo-surface-tui/src` and
      `rg '^mod (rail|panel);' crates/bingo-surface-tui/src/lib.rs` find nothing.
      `views/panel.rs` remains: it draws `View::Panel`, not the removed sheet.
- [ ] A tmux drive at 80×24, 120×40 and 200×50 with a room open and
      `--demo-ui`: no column beside the transcript at any width; `/board`
      shows the board row and its Tick button changes the table; the
      progress bar runs in the activity band; `ctrl+p` does nothing.
- [ ] The removed snapshots (rail, sheet, pins, `owed_*`, the rail cases
      of `roster_*`) are listed with the reason; the rest unchanged.
- [ ] `scripts/tui-smoke.sh` passes; its `❯ Board` and pinned-board steps
      now go through `/board` (edit the steps, say so).
- [ ] All gates of the upgrade protocol, plus the Windows cross-check
      for every touched crate.

## Non-goals

- The crew strip (M150), which is where `owed` is said on the status
  line; until then the session list's row says it, as it does today.
- Panes (M140), which follow ADR-0081 §7 when they land.
- Removing `Slot::Rail` or `Slot::Panel` from the wire (ADR-0081 §4).
- External plugins: no protocol change; one on the legacy lanes is not
  drawn until it places itself.

## Risks

- A protocol-1 plugin that drew only through the panel lane disappears
  from the TUI. That is the decision; its guide page says how to place
  itself, and its data stays in the fold.
- `Null` removing an extension changes a journaled fold. Old journals
  fold to fewer entries, never more; the fixture pins it.
- Scope approval (2026-10-14): the baseline lifecycle assertion may change
  from present-as-null to absent. The same sole assertion change elsewhere
  is approved; list every such test here. No other outside-scope change is
  authorized. Currently identified: context
  `session_start_invalidates_both_keys_without_opening_the_session`.
- **Stopped during brick 1 (2026-10-14), protocol step 6 and the user's
  scope restriction.** The re-verification omitted `bingo-surface-tui/src/blocks.rs`:
  `Blocks::live`, the `sync(..., live)` argument and `segments()` implement
  the under-the-running-rows lane that brick 3 explicitly removes. Passing
  an empty vector forever would retain the obsolete lane, contrary to the plan.
  Recommended: approve this file for removing that field, argument and segment,
  plus mechanical `sync` caller edits in `src/motion.rs`, `src/transcript.rs`
  and `src/transcript/pictured.rs` (tests; unchanged assertions). `view.rs`
  and the callers inside `blocks.rs` complete the call-site list.
  Alternative: explicitly revise brick 3 to retain the empty lane API.
  None of these outside-scope files was changed. The ten new contract tests
  compile and fail for the expected missing behavior; implementation has not begun.

## Re-verified (2026-10-14, `23810b23`)

- Branch `m148-the-screen-is-the-persons`; clean tree at the supplied base.
  M108–M136 and M147 are the supplied landed set; M137 files are untouched.
- M124/M125 geometry is in `frame.rs` and `layout.rs`; `view::compose`
  renders rail cards and the inline fallback. `Ui::pinned`, `Focus::Card`,
  `Open::Panel`, key/pointer dispatch and layout targets are removal callers.
- `slot::with` adds `slots` to a View object without losing its other fields;
  `Slot::Row` holds a body and actions, but requires an existing `ItemId`.
  `/board` currently returns `Applied` and journals only an extension.
  Use `CommandOutcome::Record` for a board notice anchor and a demo-ui
  event observer to place the row on its recorded id; no new kernel door.
- The shared owed/member writers are in `bingo-collab`, not `roster.rs`.
  TUI `seats.rs` uses the collab readers. Schedule `wake::payload` currently
  places a `Status` countdown; M148 intentionally changes it to empty slots.
- The extension-reader search found the context assertion above. Other
  inspected readers use optional lookup/deserialization: tasks, demo-ui,
  collab room/ear/cursor/readers, context memory rows and agents report-back.
- `scripts/tui-smoke.sh` currently uses fixed tmux socket `bingo` and kills
  that server in cleanup. Before running it, use a unique owned socket;
  migrate the progress and pinned-board steps to Activity and `/board`.

## Verified (2026-10-14)

The initial re-verification stopped before code; the user approved the
Null-absence assertion updates on 2026-10-14. Implementation resumed from
`dd226ee6`. Gate evidence follows as checks run.

```text
$ git branch --show-current
m148-the-screen-is-the-persons
$ git rev-parse --short HEAD
23810b23
$ git status --porcelain=v1
(no output before the plan edit)
```

Brick 1: an initial test-fixture compile failed with `E0308` (`round: None`,
expected `u32`) and `E0560` (`ended_at`; the field is `completed_at`). The
fixture was corrected before the red run below; these were not behavioral failures.
Screen operands are omitted below; the equality failures show the legacy cards
still drawing. The journal fixture was generated with `BINGO_UPDATE_SCHEMA=1`.

```text
$ BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk -p bingo-surface-tui -p bingo-collab -p bingo-demo-ui -p bingo-schedule m148 --locked --no-fail-fast
test room::m148::m148_members_are_unplaced_and_readable ... FAILED
test owed::m148::m148_owed_is_unplaced_data ... FAILED
assertion `left == right` failed
  left: Null
 right: Array []
assertion `left == right` failed
  left: Null
 right: Array []
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s
test progress::m148::m148_progress_places_an_activity ... FAILED
test tests::m148_a_fresh_board_records_a_transcript_anchor ... FAILED
test tests::m148_board_places_its_table_and_buttons_on_its_recorded_row ... FAILED
assertion `left == right` failed
  left: Null
 right: "activity"
assertion failed: matches!(outcome, CommandOutcome::Record
    { body: ItemBody::Notice { ref code, .. } } if code == "DEMO_BOARD")
board row
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 16 filtered out; finished in 0.01s
test wake::m148::m148_wake_is_unplaced_data ... FAILED
assertion `left == right` failed
  left: Array [Object {"kind": String("status"), "text": String("wake"), "tone": String("neutral"), "until": String("1970-01-01T00:00:00Z")}]
 right: Array []
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 115 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 0.00s
test state::tests::m148_null_extension_journal_removes_only_its_kind ... FAILED
assertion failed: !state.extensions["fixture"].contains_key("board")
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 135 filtered out; finished in 0.02s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 3 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.00s
test keys::m148::m148_control_p_is_unbound_and_absent_from_help ... FAILED
test frame::m148::m148_room_and_wake_data_draw_nothing_at_every_width ... FAILED
test frame::m148::m148_unplaced_payloads_leave_the_screen_unchanged_at_every_width ... FAILED
assertion failed: !ui.focus.showing()
assertion `left == right` failed
assertion `left == right` failed
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 1137 filtered out; finished in 0.03s
error: 5 targets failed:
    `-p bingo-collab --lib`
    `-p bingo-demo-ui --lib`
    `-p bingo-schedule --lib`
    `-p bingo-sdk --lib`
    `-p bingo-surface-tui --lib`
```

```text
$ cargo fmt --all -- --check
(exit 0; no output)
$ git diff --check
(exit 0; no output)
```

Stopped before implementation: workspace check, clippy, workspace tests,
discipline, budget, deny, MSVC and PTY gates were not run. No live or credential
tests ran; no tmux server was touched. All completion criteria remain unticked.
