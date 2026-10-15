# ADR-0090 — The crew acts through services

Status: proposed · 2026-10-15 · Plan: M176 · Amends: ADR-0082 §3 (the crew view)

## Context

Agents, rooms and tasks are three plugins with three tables. Stopping an
agent, seating one and ticking a task is three commands and three reads.
One panel wants all of it, and a panel is a fourth plugin: it may not
import `bingo-agents`, `bingo-rooms` or `bingo-tasks` (ADR-0001). The
verbs exist in their plugins (`stop.rs`, `dismiss.rs`, `seat.rs`,
`update.rs`). No kernel door is asked for; the ratchet does not arise.

## Decision

1. **Three typed services in `bingo-services`**, each a key, a
   capability, a native-or-wire `find(host)` and a fixture (`room_post`):
   `agents-control` (`roster`, `stop`, `dismiss`, `reopen`, `message`
   without switching), `rooms-control` (`rooms`, `seat`, `unseat`,
   `set_purpose`, `close`), `tasks-write` (`board`, `add`, `set_status`,
   `reassign`). Each refuses as its command does: ADR-0060's starter,
   ADR-0053 §5's holder or opener, ADR-0068's owner.
2. **The owning plugin registers it; its command calls the same
   function.** `/agents stop` and the panel's `s` are one code path.
3. **`bingo-mod-crew` is the crew's plugin view** (ADR-0082 §3), tabs
   agents | rooms | tasks; an absent service hides its tab.
4. **`ctrl+n` opens it**, bound to `/crew` (ADR-0086 §5); a click on the
   crew strip opens it while the mod is on, else the palette's group.

## Consequences

- Three fixtures and wire faces; a JS or process plugin drives the crew
  the same way, and the tables stay as the plain fallback.
- The three plugins now bring a service, so they switch at the next
  start (ADR-0088 §1); the mod brings none and switches now.
- A verb added later reaches the panel only through its service.

Refs: ADR-0001, ADR-0053, ADR-0060, ADR-0068, ADR-0082, ADR-0086, ADR-0088.
