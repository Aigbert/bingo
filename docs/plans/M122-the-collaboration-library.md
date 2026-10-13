# M122 — The collaboration library

## Goal

ADR-0071 §1–3. Agents and rooms share one name rule, one set of room
record types and one writer for a room. A chase, a patience deadline and
an agent's report-back survive a restart, and a resumed session shows no
countdown for a wake that died. Two library crates join the tier:
`bingo-collab` (names, room records, pure folds) and `bingo-alarm`
(owned, keyed timers). No kernel door, no feature noun in sdk or core,
no new external dependency. The TUI's copies are M123's.

## Bricks, in build order

1. **Contract tests first**, failing:
   - a table of name pairs (`Scout`/`scout`, `Parent`/`PARENT`, `#room`)
     through `bingo_collab::Name`;
   - a golden fixture per room record kind, taken from today's journals;
   - `bingo-alarm` on the paused clock (M111): arm and fire; cancel
     before it fires; re-arming a key replaces it; `reconcile` keeps,
     cancels and arms; dropping `Alarms` cancels all.
2. **`bingo-collab`** (`tier = "library"`; `bingo-sdk`,
   `bingo-services`, `serde`, `serde_json`, `jiff`):
   - `name`: `Name::parse` holds the one check; equality ignores case
     and display keeps it; `PARENT` is reserved in any case.
   - `room`: the record id and kinds as constants beside their types,
     `Opened`, `Closed`, members, `Listener`, `Ear`, cursor.
   - `mentions`, `owed` and `seen`: the pure folds from rooms' own
     modules and agents' `serial.rs` (`read`, `verdict`).
3. **Rooms reads through it.** `name.rs` goes. `room.rs`, `cursor.rs`,
   `ear.rs`, `mentions.rs` and `owed.rs` keep their I/O and import the
   types. Existing rooms tests pass unchanged.
4. **Agents reads through it.** `names.rs` keeps `free`, `candidates`
   and `named`, comparing by `Name`. M109's local `same` and its
   two-crate test in `crates/bingo` go. `rooms.rs` is deleted: the
   cursor and `closed` come from `bingo_collab::room`.
5. **`RoomPost`** in `bingo-services` (key `rooms.post`, trait, wire
   adapters, golden JSON): `post(room, caller, author, said, cut)`
   returns a landed post or a worded bounce; `said` is text or `Again`.
   Rooms provides it: the closed check, the serial rule, the draft and
   the bounce wording move from agents to a rooms `serial.rs`, and
   `post.rs` writes what lands. Agents' `SendMessage` room arm calls
   `room_post::find`, loses `post` and `journals`, and declares
   `uses: &[room_post::KEY.name()]`. Rooms already uses `team::KEY`.
6. **`bingo-alarm`** (`tier = "library"`; `bingo-sdk`, `tokio`, `jiff`):
   `Alarms<K: Ord + Clone>` with `arm(key, at, deliver)`, `cancel(key)`,
   `reconcile(wanted)`. Each alarm is a task whose token `Alarms` holds.
7. **Rooms re-arm.** `chase.rs` and `deadline.rs` keep `Alarms` keyed by
   debt and by `(seat, room)`, armed from the owed fold and the cursors.
   After a nudge, the next one is armed only if the fold still owes.
   `RoomsPlugin::start` lists the sessions under `room::KEY` that are not
   closed and reconciles both. Overdue is chased once, as today
   (ADR-0022 §3). The lazy `overdue` call in `hook.rs` goes.
8. **Schedule clears a dead wake.** `wakes.rs`'s loop becomes
   `Alarms<SessionId>`. A new schedule hook clears the journaled `wake`
   at `on_session(Start)` when this process holds no wake for that
   session. The manifest gains `hook:schedule`.
9. **The report-back is journaled.** `spawn.rs` writes an obligation in
   the parent: extension `bingo.agents`/`report:<child>`, payload
   `{ child, turn }`, with a golden fixture. It is cleared (`Null`) on
   delivery. The report task is held per child and cancelled at the
   parent's `on_session(End)`. At the parent's `on_session(Start)`,
   agents re-arms each open obligation:
   - the awaited turn already ended → the reply is delivered now;
   - the child is gone → the obligation is cleared with a note;
   - otherwise the child is followed again.
10. **Records.** ADR-0021, 0022 §3, 0025 §2, 0029 §3 and 0019 §8 get
    dated notes; ADR-0071 is accepted for §1–3; ARCHITECTURE.md lists both.

## Files

- `crates/bingo-collab/{Cargo.toml,src/{lib,name,room,mentions,owed,seen}.rs,fixtures/}` (new)
- `crates/bingo-alarm/{Cargo.toml,src/lib.rs}` (new)
- `crates/bingo-services/src/{lib.rs,room_post.rs}`, `fixtures/room_post/`
- `crates/bingo-rooms/src/{lib,room,cursor,ear,mentions,owed,chase,deadline,post,hook}.rs`,
  a new `serial.rs`; `name.rs` deleted
- `crates/bingo-agents/src/{lib,names,message,spawn,watch,hook}.rs`;
  `{rooms,serial}.rs` deleted
- `crates/bingo-schedule/src/{lib,wake,wakes}.rs`, a new `hook.rs`
- `crates/bingo/tests/` (M109's two-crate name test removed)
- `Cargo.toml` (members), `ARCHITECTURE.md`,
  `docs/adr/{0019,0021,0022,0025,0029,0071}-*.md`

## Exit criteria

- [ ] `cargo test -p bingo-collab -p bingo-alarm` passes; the failing
      first run is pasted.
- [ ] `rg 'fn same|fn is_holder|"bingo\.rooms"' crates/bingo-agents/src`
      finds nothing; rooms' `name.rs` and agents' `{rooms,serial}.rs` are gone.
- [ ] An agents test refuses `scout` beside `Scout`, and `Parent`.
- [ ] `rg 'tokio::spawn' crates/bingo-rooms/src/{chase,deadline}.rs crates/bingo-agents/src/spawn.rs crates/bingo-schedule/src/wakes.rs`
      finds nothing.
- [ ] Restart tests, a second plugin starting over a store the first
      wrote: a debt is chased once; a patient seat behind is nudged once;
      a stale wake is cleared; a report owed by a finished child is
      delivered, and one owed by a running child when it finishes.
- [ ] The `SendMessage` room tests pass unchanged against rooms' writer,
      and check 4e passes with `rooms.post` declared.
- [ ] All gates of the upgrade protocol, plus the Windows cross-check
      for the five crates touched.

## Non-goals

- `Voice::Aside` on nudges (ADR-0071 §4), which needs
  `Origin.voice` (M123).
- The TUI's `seats.rs`, `roster.rs`, `mentions.rs` and `HOLDER` (M123).
- Wakes that outlive their process (ADR-0019 §8 keeps them in memory).

## Risks

- The milestone is large. If it outgrows a review, split it after
  brick 5 (names, records, the post) and land brick 6 onward as its own
  milestone, with no record change between the two.
- Case-folded equality makes two agents that differ only in case one
  name. A journal that already holds both resolves to the first; the
  commit body says so.
- `start` reads every open room's journal. That is bounded by the
  number of open rooms, and is the read the lazy path did on first use.
- Re-verify at start: M109's name fix, M116's owned tasks, M119's
  `bingo-services` layout, and the call sites above by symbol.
