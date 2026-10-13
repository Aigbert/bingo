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
- **Stopped at re-verification (2026-10-14), protocol step 6.** Outside
  Files, `crates/bingo-context/src/baseline/lifecycle.rs:121`, test
  `session_start_invalidates_both_keys_without_opening_the_session`, asserts
  `fixture.journal.state().extensions[PLUGIN][id].is_null()`.
  `BaselineHook::on_session` invalidates these keys with `extend(..., Null)`.
  ADR-0081 §5 removes the keys, so the indexed assertion would panic.
  This is an assertion change, not the mechanical constructor/fixture
  exception allowed by step 6; no implementation or test was changed.
- Recommended resolution: authorize adding that lifecycle test to Files and
  assert key absence with `extensions.get(PLUGIN).is_none_or(|kinds|
  !kinds.contains_key(id))`. Keep its refresh, stable-recapture and sequence
  assertions. The production `baseline::retained` reader already accepts
  a missing key, so no context production change is indicated.
  Alternative: defer §5 to a separately approved milestone, requiring an
  explicit revision of this plan/ADR's implementation scope.

## Re-verified (2026-10-14, `23810b23`)

- Branch `m148-the-screen-is-the-persons`; clean tree at the supplied base.
  M108–M136 and M147 are the supplied landed set; M137 files are untouched.
- M124/M125 geometry is in `frame.rs` and `layout.rs`; `view::compose`
  renders rail cards and the inline fallback. `Ui::pinned`, `Focus::Card`,
  `Open::Panel`, key/pointer dispatch and layout targets are removal callers.
- `slot::with` adds `slots` to a View object without losing its other fields;
  `Slot::Row` holds a body and actions, but requires an existing `ItemId`.
  `/board` currently returns `Applied` and journals only an extension.
  Its anchor mechanism remains to be resolved before implementation;
  `CommandOutcome::Record` plus an event observer is an existing-door option.
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

Re-verification only; stopped under protocol step 6 before code. Exit
criteria remain unticked, ADR-0081 remains proposed, and no gate, live test,
credential test or tmux drive was run. No schema or fixture was regenerated.

```text
$ git branch --show-current
m148-the-screen-is-the-persons
$ git rev-parse --short HEAD
23810b23
$ git status --porcelain=v1
(no output before the plan edit)
```
