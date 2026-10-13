# ADR-0071 — Collaboration is one library

Status: proposed · 2026-10-13 · Plan: M122 · Amends: ADR-0021, ADR-0022
§3, ADR-0025 §2, ADR-0029 §3, ADR-0034 §3, ADR-0019 §8

## Context

Agents and rooms are one feature split across two crates by strings:

- **A name is written three times and compared two ways.** Rooms
  (`bingo-rooms/src/name.rs`: `same`, `is_holder`, `check`, `PARENT`)
  lowercases both sides. Agents (`bingo-agents/src/names.rs`: `free`,
  `check`, `named`, `PARENT`, `ROOM`) compares exactly. So `Scout` and
  `scout` can both be spawned while a room treats them as one member
  with one cursor, and an agent may be named `Parent`. The TUI keeps a
  third copy (`seats.rs`, `HOLDER`). M109 fixes the symptom.
- **The room's record is read by three crates.** Agents re-spells
  `"bingo.rooms"`, `cursor:`, `post` and `closed` (`bingo-agents/src/rooms.rs`).
  The TUI re-spells those and about twenty more (`seats.rs`).
- **The `#room` write path lives in agents.** `message.rs` `post()` and
  `journals()` and `serial.rs` `draft`/`bounce` decide what a post is
  and whether it lands. That is room law, held by the plugin that does
  not own rooms.
- **Four timers implement "deliver to S at T unless cancelled"** and
  each dies with its process. Rooms' `chase.rs` arms a bare
  `tokio::spawn` per debt with a token map. Rooms' `deadline.rs` arms
  one with no token. Schedule's `wakes.rs` holds wakes in memory, as
  ADR-0019 §8 wants, but its journaled countdown outlives them. The
  agent report-back (`spawn.rs`, `tokio::spawn(watch::report(..))`) is a
  detached task with a token nobody cancels, lost on restart.

No kernel door is opened, and no feature noun reaches the sdk or core.
Everything here is a library a plugin chooses to depend on (ADR-0012 §1).

## Decision

1. **`bingo-collab` is the collaboration library** (`tier = "library"`;
   sdk and `bingo-services` only). It holds:
   - `Name`: one validation; equality ignores case, display keeps it;
     `PARENT` is reserved in any case. Rooms' `name.rs`, agents'
     `names.rs` checks and the TUI's `HOLDER` go.
   - The room's record types: members, listeners and ears, cursors,
     posts, the owed fold, closed. Each has a serde shape pinned by a
     golden fixture, and the record's id and kinds are constants beside
     them. Agents and the TUI read a room through these types and never
     spell a field.
   - The pure folds rooms already has (mentions, owed, the serial rule's
     "seen"). Agents and the TUI can then ask the same question without
     a copy.
2. **A post is written by rooms.** `bingo-services` gains `RoomPost`:
   `post(room, author, text, cut)` returns a landed post or a worded
   bounce. Agents' `SendMessage` room arm calls it. The serial
   checkpoint, the draft and the bounce move to rooms. ADR-0025 §2 is
   amended: the checkpoint is room law, run by rooms when any tool posts.
   Both sides declare the other in `uses` (ADR-0069 §3), not `requires`.
   A mutual `requires` stands under the fixpoint, but switching rooms off
   would take agents down with it.
3. **A pending delivery is derived, then armed.** A timer is never the
   record. Each owner keeps the fact in its own record, arms from it, and
   re-arms from it when a process starts:
   - `bingo-alarm` is a small library crate: `Alarms<K>` keyed by the
     owner's key, `arm(key, at, deliver)`, `cancel(key)` and
     `reconcile(wanted)`. Every alarm is a task with a token the
     owner holds (ADR-0068 §1). It is its own crate because schedule
     must not depend on a collaboration library. It adds no new external
     dependency.
   - Rooms derive chases and patience deadlines from the owed fold and
     the cursors (ADR-0022 §3, ADR-0029 §3). At `Plugin::start` they list
     open rooms and `reconcile`. Overdue is chased once, as today.
   - Schedule keeps wakes in memory (ADR-0019 §8). Its journaled `wake`
     record is cleared at `on_session(Start)` in a process that holds no
     wake for that session, so a resumed session shows no countdown for
     a wake that died.
   - An agent's report-back becomes an obligation the parent journals
     (an Extension naming the child and the turn awaited). It is re-armed
     at the parent's session start by following the child again, and
     cleared on delivery. The report task is owned and cancelled with the
     parent's close.
4. **A nudge is a delivery with a voice.** Room nudges are stamped
   `Voice::Aside` (ADR-0070 §4). The room reading folded into a turn is a
   contributor piece, already omitted as context. ADR-0034 §3's wake
   stays `Delivery::Wake`, and the voice says how it is drawn.

## Consequences

- One name rule. `Parent`, `PARENT` and `parent` are one reserved word
  everywhere, and two agents cannot differ only in case.
- Agents lose `rooms.rs`, `serial.rs` and the room half of `message.rs`.
  Rooms own every write into a room. The TUI reads seats through
  `bingo-collab` (ADR-0070 §5).
- A restart loses nothing a person was promised: a chase, a deadline and
  a report-back come back from their records. A wake still dies with its
  process, by ADR-0019 §8's choice, and now says so.
- Two library crates join the tier (`bingo-collab`, `bingo-alarm`).
  `bingo-services` gains one trait.
- ADR-0021's door is unchanged: `OpenRoom` still calls `seat::seat`. Its
  member names become `Name`s.

Refs: ADR-0012, ADR-0019, ADR-0021, ADR-0022, ADR-0025, ADR-0029,
ADR-0034, ADR-0053, ADR-0068, ADR-0069, ADR-0070; Plan: M122;
audit-2026-10 §2, §3
