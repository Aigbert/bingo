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
   `progress.rs`, `lib.rs` module header): the board as a `Row`, the
   progress as an `Activity`, placed through ADR-0070's `slot::with`.
   rooms (`owed.rs`, the `members` writer in `roster.rs`/`bingo-collab`)
   and schedule (`wake.rs`): `slots: []`.
5. **Records.** ADR-0081 accepted; dated notes on ADR-0013 §2 §4,
   ADR-0070 §1 and ADR-0079 §3; tui.md §3 (*Rail* removed), §8's lane
   table, §10 dated entry; the guide pages that name `ctrl+p` or the
   rail.

## Files

- `crates/bingo-sdk/src/state.rs` and its fixtures
- `crates/bingo-surface-tui/src/{frame,rail,panel,layout,pointer,status,keys,view}.rs`
  (`rail.rs` and `panel.rs` deleted; `layout.rs` is M124's)
- `crates/bingo-demo-ui/src/{board,command,progress,lib,tests}.rs`
- `crates/bingo-rooms/src/{owed,roster}.rs`, `crates/bingo-collab/src/room.rs`
- `crates/bingo-schedule/src/wake.rs`
- `scripts/tui-smoke.sh`, the guide pages naming `ctrl+p` or the rail
- `docs/adr/{0013,0070,0079,0081}-*.md`, `docs/design/tui.md`,
  `docs/design/tui-identity-2026-10.md`

## Exit criteria

- [ ] Each brick-1 test failed first (output pasted) and passes now.
- [ ] `rg 'RAIL_AT|RAIL_WIDTH|mod rail|mod panel|ctrl\+p' crates/bingo-surface-tui/src`
      finds nothing.
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
- Re-verify at start: M124's layout targets for the rail and the sheet,
  M123's readers for `owed`/`members`, how M121 carries `slots` on a
  payload that is itself a `View`, and whether a `Row` holds a table body
  and two actions as the board needs.
