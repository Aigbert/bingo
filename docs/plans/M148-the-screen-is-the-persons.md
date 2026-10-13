# M148 — The screen is the person's

## Goal

[tui-identity-2026-10.md](../design/tui-identity-2026-10.md) §2 and C5,
[ADR-0081](../adr/0081-the-screen-is-the-persons.md). The rail and the
`ctrl+p` sheet — what the person called "the board" — stop showing
plugin data nobody asked for. The rail holds what a person pinned, and
what a plugin placed when the terminal has room for it (144 columns);
the sheet names things in the plugin's words and hides what is empty;
the rooms plugin's `owed` and `members` become data, said by the session
list. No new dependency.

## Bricks, in build order

1. **Failing tests first**:
   - sdk `state.rs`: `extended(.., Value::Null)` removes the kind, like
     `signalled`; a fixture journal with an extension then a `Null` folds
     to no entry (ADR-0081 §4).
   - TUI `frame.rs`: a legacy payload draws no rail at 200 columns; a
     `Slot::Rail` card seats at 144 and not at 143; a pinned panel seats
     at `80 + RAIL_WIDTH` and not one column under.
   - TUI `panel.rs`: a payload of `Null` or an empty view is not a row;
     `bingo.schedule`/`wake` is named `wake` with `schedule` beside it; a
     placed `Panel` keeps its title; the cursor row says `pin`/`unpin`.
   - rooms: the `owed` signal and the `members` extension carry
     `slots: []`; `seats.rs`'s readers still read both by name.
   - TUI: a session with only `owed` and `members` draws no rail and no
     sheet rows at any width.
2. **The reducer** (sdk `state.rs` `extended`): `Null` removes. Check
   every `extensions` reader for a `Null` it relied on (the schedule's
   wake, `bingo-schedule/src/wake.rs`) and list them.
3. **The rail's gate** (`frame.rs`, `rail.rs`, and M124's layout): two
   thresholds, `RAIL_UNASKED = 144` for placed cards and
   `80 + RAIL_WIDTH` for pinned ones, replacing `RAIL_AT`. Legacy
   payloads leave `rail::cards` and the under-the-running-rows lane;
   `rail.rs`'s module header and tui.md §3 say so.
4. **The waiting notice**: the first frame a placed card cannot seat, one
   `Info` notice `a card is waiting · ctrl+p`, once per session; derived
   from the layout, nothing kept but the once.
5. **The sheet** (`panel.rs`): the naming rule, the empty rule, the `⏎`
   word; `NOTHING` stays for a session with no row.
6. **Rooms' data** (`bingo-rooms` `owed.rs`, the `members` writer in
   `roster.rs`/`bingo-collab`): `slots: []` on both. The seats readers
   (`seats.rs`, through M123's `bingo-collab` types) keep reading.
7. **Records.** ADR-0081 accepted; dated notes on ADR-0013 §2 §4 and
   ADR-0070 §1; tui.md §3 *Rail*, §8's lane table and §10.

## Files

- `crates/bingo-sdk/src/state.rs` and its fixtures
- `crates/bingo-surface-tui/src/{frame,rail,panel,layout,status,keys}.rs`
  (`layout.rs` is M124's)
- `crates/bingo-rooms/src/{owed,roster}.rs`, `crates/bingo-collab/src/room.rs`
- `crates/bingo-schedule/src/wake.rs` (only if brick 2 finds it relied on
  `Null` staying)
- `docs/adr/{0013,0070,0081}-*.md`, `docs/design/tui.md`,
  `docs/design/tui-identity-2026-10.md`

## Exit criteria

- [ ] Each brick-1 test failed first (output pasted) and passes now.
- [ ] `rg 'RAIL_AT' crates/` finds nothing.
- [ ] A tmux drive at 120×40 and 150×40 with a room open and the
      `--demo-ui` board: no rail at 120; at 150 only placed cards; `ctrl+p`
      lists `Board` and `progress` by their words, no `–` row, no
      `members`; `⏎` pins the board and it seats at 120.
- [ ] The removed snapshots (`owed_*`, the rail cases of `roster_*`,
      `the_rail_holds_*`) are listed with the reason; the rest unchanged.
- [ ] `scripts/tui-smoke.sh` passes; its `❯ Board` step now reaches the
      board through `ctrl+p` (edit the step, say so).
- [ ] All gates of the upgrade protocol, plus the Windows cross-check
      for every touched crate.

## Non-goals

- The crew strip (M150), which is where `owed` is said on the status
  line; until then the session list's row says it, as it does today.
- Pins kept across runs; a rail width other than 24; the panel sheet's
  look beyond naming (M149 restyles everything at once).
- External plugins: no protocol change; they are drawn by §1.

## Risks

- A plugin that relied on the rail to be noticed is now one key away.
  The notice in brick 4 covers placed cards only, by design: a legacy
  payload never asked for a place.
- `Null` removing an extension changes a journaled fold. Old journals
  fold to fewer entries, never more; the fixture pins it.
- Re-verify at start: M124's layout targets for the rail and the sheet,
  M123's readers for `owed`/`members`, and how M121 carries `slots` on a
  payload that is itself a `View`.
