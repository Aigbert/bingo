# M148 — The screen is the person's

## Goal

[tui-identity-2026-10.md](../design/tui-identity-2026-10.md) §2 and C5,
[ADR-0081](../adr/0081-the-screen-is-the-persons.md). The rail and the
`ctrl+p` sheet — what the person called "the board" — go, by the
person's word of 2026-10-13. The transcript takes the width at every
size. Plugins place themselves or remain data. No new dependency.

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
- `crates/bingo-rooms/src/{owed,roster,room}.rs`, `crates/bingo-collab/src/{room,owed}.rs`
- `crates/bingo-schedule/src/wake.rs`
- `crates/bingo/tests/rooms.rs` (approved default-slot expectation)
- `crates/bingo-context/src/baseline/lifecycle.rs` (approved Null-absence assertion)
- `scripts/tui-smoke.sh`, the guide pages naming `ctrl+p` or the rail
- `docs/adr/{0013,0070,0079,0081}-*.md`, `docs/design/tui.md`,
  `docs/design/tui-identity-2026-10.md`

## Exit criteria

- [x] Each brick-1 test failed first (output pasted) and passes now.
- [x] `rg 'RAIL_AT|RAIL_WIDTH|ctrl\+p' crates/bingo-surface-tui/src` and
      `rg '^mod (rail|panel);' crates/bingo-surface-tui/src/lib.rs` find nothing.
      `views/panel.rs` remains: it draws `View::Panel`, not the removed sheet.
- [x] A tmux drive at 80×24, 120×40 and 200×50 with a room open and
      `--demo-ui`: no column beside the transcript at any width; `/board`
      shows the board row and its Tick button changes the table; the
      progress bar runs in the activity band; `ctrl+p` does nothing.
- [x] The removed snapshots (rail, sheet, pins, `owed_*`, the rail cases
      of `roster_*`) are listed with the reason; retained help/roster snapshots change only
      the removed shortcut/cards and reflow, keeping their assertions.
- [x] `scripts/tui-smoke.sh` passes; its `❯ Board` and former pin steps
      now go through `/board`, Tab and the row’s Tick/Reset actions.
- [x] All gates of the upgrade protocol, plus the Windows cross-check
      for every touched crate. TUI (`aws-lc-sys`) and bin (`aws-lc-sys`,
      `tree-sitter`, `tree-sitter-bash`) await Windows CI for SDK C headers, per step 7.

## Non-goals

- The crew strip (M150), which is where `owed` is said on the status
  line; until then the session list's row says it, as it does today.
- Panes (M140), which follow ADR-0081 §7 when they land.
- Removing `Slot::Rail` or `Slot::Panel` from the wire (ADR-0081 §4).
- External plugins: no protocol change; one on the legacy lanes is not
  drawn until it places itself.

## Risks

- Legacy protocol-1 views disappear until placed; their data stays in the fold.
- Null removes old journal entries from the fold; the fixture pins it.
- `bingo-context/src/baseline/lifecycle.rs`: approved Null-absence expectation.
- Approved TUI removal scope (relative to `src/`; each is removal/caller-only):
  - `blocks.rs`.
  - `focus.rs`.
  - `frame.rs`.
  - `input.rs`.
  - `keys.rs`.
  - `layout.rs`.
  - `layout/tests.rs`.
  - `lib.rs`.
  - `motion.rs`.
  - `panel.rs`.
  - `pointer.rs`.
  - `rail.rs`.
  - `screens.rs`.
  - `screens/teams.rs`.
  - `screens/windows.rs`.
  - `select.rs`.
  - `test_lanes.rs`.
  - `theme.rs`.
  - `transcript.rs`.
  - `transcript/pictured.rs`.
  - `ui.rs`.
  - `ui/controls.rs`.
  - `view.rs`.
  - `views/mod.rs`.
  - `views/tests.rs`: delete the removed legacy action traversal test.
  - `window.rs`.
- Deleted snapshots: `panel_end`, `panel_middle`, `custom_panel_{80x24,120x40}`, `panel_sheet_{80x24,120x40}`, `owed_{80x24,120x40}`, `ctrl_p_shows_what_the_plugins_wrote_into_the_session`, `the_rail_holds_the_pinned_board_and_the_live_progress_card`, `without_a_rail_the_same_cards_draw_under_the_running_rows`: removed sheet/rail/lane only.
- `bingo/tests/rooms.rs`: approved default `slots: []` in the membership expectation.
- TUI `screens/snapshots/bingo_surface_tui__screens__teams__roster_in_the_rooms.snap`: removal-only reflow.
- TUI `snapshots/bingo_surface_tui__motion__sheet_close.snap`: removal-only reflow.
- TUI `snapshots/bingo_surface_tui__motion__sheet_slide.snap`: removal-only reflow.
- TUI `snapshots/bingo_surface_tui__screens__help_100x30.snap`: removal-only reflow.
- TUI `snapshots/bingo_surface_tui__screens__help_80x30.snap`: removal-only reflow.
- TUI `snapshots/bingo_surface_tui__screens__roster_120x40.snap`: removal-only reflow.
- TUI `snapshots/bingo_surface_tui__screens__roster_80x24.snap`: removal-only reflow.
- TUI `snapshots/bingo_surface_tui__view__tests__help_panel.snap`: removal-only reflow.
- `bingo-rooms/src/room.rs`: approved default `slots: []` in two complete-payload expectations.
- Standing approval (2026-10-14): any crate's expectation may add default `slots: []` or assert absence after Null removal; other outside-scope changes still stop.
- `bingo-surface-tui/src/screens/shells.rs`: remove stale rail comments.
- `bingo-surface-tui/src/layout/count.rs`: remove the deleted rail's test counter; four remaining bands still render exactly once.

## Re-verified (2026-10-14, `23810b23`)

- Original tree clean; branch `m148-the-screen-is-the-persons`. M137 untouched.
- Geometry: `frame`/`layout`; data writers: collab `room`/`owed`. Wake moves
  from Status to empty slots. The board uses Record plus an event observer.
- Extension readers inspected: tasks, demo-ui, collab room/ear/cursor/readers,
  context baseline/memory, agents report-back. Only the approved baseline
  assertion requires present-as-null to absent so far.
- Smoke uses a unique owned tmux socket; three widths replace the pinned-board steps.
- Known ACP token tests run alone, per protocol; no live/credential tests run.

## Verified (2026-10-14)

Full gate logs are in commit `c58fdcb3`; final gate result lines:

```text
$ cargo fmt --all -- --check                       (exit 0)
$ cargo check --workspace --all-targets --locked   Finished
$ cargo clippy --workspace --all-targets --locked -- -D warnings   Finished
$ cargo test --workspace --locked --no-fail-fast -- --skip <4 ACP token tests>
  summed: 5169 passed; 0 failed; 4 ignored
$ cargo test -p bingo --test acp_bridge ... --test-threads=1      1 passed
$ cargo test -p bingo-provider-acp --test bridge token -- --test-threads=1   3 passed
$ scripts/check_discipline.sh                      discipline ok
$ scripts/budget.sh                                355 deps (max 355); budget ok
$ cargo deny check                                 advisories ok, bans ok, licenses ok, sources ok
$ cargo check --target x86_64-pc-windows-msvc      sdk, context, demo-ui, collab, rooms, schedule ok;
  surface-tui and bingo deferred (aws-lc-sys / tree-sitter C headers absent on this host)
$ scripts/tui-smoke.sh                             80x24, 120x40, 200x50: board Tick/Reset
  and Activity ok; ctrl+p unbound; tui-smoke ok
$ rg 'RAIL_AT|RAIL_WIDTH|ctrl\+p' crates/bingo-surface-tui/src   no matches
$ git diff --check                                 (exit 0)
```
