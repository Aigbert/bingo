# ADR-0070 — A plugin places its own view

Status: proposed · 2026-10-13 · Plans: M121, M123 · Amends: ADR-0013 §2,
§4, ADR-0019 §8

## Context

ADR-0013 gave plugins three lanes and a vocabulary, and kept placement
for the surface. The surface then placed eight plugins by reading their
private records by id. About 3.4k production lines of the TUI know a
producer's id, kind and field names, with no fixture shared with that
producer:

- `tasks.rs` (`"bingo.tasks"`/`tasks`, four tool names, `activeForm`);
- `shells.rs` (`"bingo.tools.bash"`/`jobs`);
- `wake.rs` (`"bingo.schedule"`/`wake`, field `at`);
- `permission.rs` (`"bingo.permissions"`, `mode`/`next`);
- `memory.rs` (`"_bingo.context"`/`memory`);
- `skill.rs` (tool `Skill`);
- `seats.rs`, `roster.rs` and `mentions.rs` (`"bingo.rooms"` and about
  twenty field names);
- `input.rs` (`PROMOTE = "bash.promote"`).

The generic lanes grew exceptions for the same reason: `panel.rs` skips
`tasks::is_list`, `rail.rs` skips `shells::is_set`, and `transcript.rs`
asks `tasks::quiet`. The words a line was said in are a closed list of
producers' surface words in `transcript/said.rs` (`QUIET_SURFACES`,
`ROOMS`, `ROOMS_READING`, `WAKE`). Every feature milestone grows the TUI.

The ratchet question: would refusing these sdk types force a second
representation of a kernel-owned fact somewhere else? The facts here are
producers', not the kernel's. The TUI's copy of them is the second
representation, and these types remove it. No `HostApi` verb is added.

## Decision

1. **`bingo_sdk::slot::Slot` names a place, not a feature.** Variants:
   - `Status { text, tone, until: Option<Timestamp> }`: the status line,
     with a countdown when `until` is set.
   - `Activity { verb: Option<String>, view }`: under the activity row;
     `verb` replaces the turn's own word while it stands.
   - `Rail { title, view }` and `Panel { title, view }`: a card, and an
     entry in the panel sheet.
   - `Mode { current, modes: Vec<ActionItem> }`: the mode a person
     cycles. The surface's cycle key fires the item after `current`.
   - `Row { item: ItemId, mark: Option<char>, title, badge,
     body: Option<View>, actions: Vec<ActionItem> }`: what one
     transcript item's row says. `mark` stands in the item's bullet; a
     surface draws it only when it is one cell wide, else the bullet.

   `ActionItem { label, action: Action, key: Option<char> }` is
   ADR-0013 §1's `Actions` item, named. A slot a surface cannot parse
   (one from a newer plugin) is skipped, as a `View` it cannot draw is
   drawn by its fold.
2. **Slots ride in a payload the plugin already publishes**, under one
   sdk-owned member, `slots`. There are three lanes and no new verb:
   `signal` (live, `Null` removes), `extend` (journaled), and a policy's
   `describe` into `ConfigView.plugins[id]` (ADR-0009 §5), which is
   where permissions already publishes its mode. A slot derived from a
   record is written with that record in one write, so no reader sees one
   without the other, as `ToolOutput` carries `parts` and `display`
   together. `slot::with(payload, slots)` builds the payload.
   `slot::placed(&SessionState) -> Vec<Placed>` is the one reader, where
   `Placed` is a slot with its plugin and kind. A payload that carries
   `slots` is drawn only where its slots say. That is the rule
   `is_list` and `is_set` were exceptions to.
3. **`ToolTraits` gains `quiet`.** A quiet call draws no row of its own
   unless it failed. A display it returns still draws. `traits(input)` is
   already per input, so a task call aimed at a room board (`in`) is not
   quiet.
4. **`Origin` gains `voice: Option<Voice>`**: `Said`, `Note` (said, drawn
   under the surface word, as a wake is), `Quiet` (machinery reporting
   in, drawn as a notice) and `Aside` (machinery a member's transcript
   omits; a room's nudge). The producer stamps the voice it means. A
   contributor's piece carries none: the kernel stamps its origin, and
   `Origin::is_context` already omits it everywhere. An origin with no
   voice is read through the surface words `said.rs` holds today, the
   voice records written before this field were drawn in, and is
   otherwise `Said`. That list is frozen as history: no producer of ours
   relies on it, and a client that sends no voice is `Said`.
5. **A record the surface must interpret, not just draw, is a library
   type.** The switcher's seat columns, the `@` targets and a room's
   unread count steer the surface's own navigation, which a `View`
   cannot do. They read typed records from `bingo-collab` (ADR-0071),
   whose module owns the producer's id. The TUI names the type, never the
   id.
6. **The surface still decides how.** ADR-0013 §4 is amended to read: a
   plugin says *which* place; the surface decides what that place looks
   like, its keys, its focus and when it folds away. Print and channels
   draw a slot's text fold or nothing.
7. **The check.** `check_discipline.sh` fails on a `"bingo.` or
   `"_bingo.` literal, or on a tool name from check 4c's list, in TUI
   production code. The exceptions are the TUI's own manifest id, and
   `acp.rs` until ADR-0076 removes it.

## Consequences

- A plugin's view is tested where the plugin is: a slot is a value
  asserted with `assert_eq!` beside the record it derives from. The
  TUI's per-plugin screen tests become producer fixtures plus one
  rendering test per slot.
- A new plugin needs no TUI change to appear in the status line, the
  activity band, the rail, the mode cycle or its own tool rows.
- ADR-0019 §8's countdown comes from `Status.until`, and "animating"
  asks whether any placed status has an `until` ahead of now, instead of
  asking the schedule's record.
- `Origin` changes in the journal. An optional field is not breaking
  (ADR-0065 §5), and it lands after M113's fixtures.
- Interaction remains `Input::Action` and `ToolHost::ask` (ADR-0013 §3).
  A slot's action reaches the kernel by the same path a card's does.

Refs: ADR-0001, ADR-0009, ADR-0013, ADR-0019, ADR-0034, ADR-0038,
ADR-0065, ADR-0071, ADR-0076; Plans: M121, M123; audit-2026-10 §2;
design/tui.md §8
