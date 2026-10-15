# ADR-0089 — A session is forked from the list

Status: proposed · 2026-10-15 · Plan: M175 · Amends: ADR-0084 §1, §6

## Context

`ctrl+r` lists twenty recent roots by title or directory with a raw
timestamp; nobody titles a session, so it is twenty directory names. A
person wants the first ask, the age in words, children under parents,
everything past twenty, and to fork a session at a turn. The ratchet,
for each door. First ask on the summary: refused, the list replays N
journals or caches first asks, a second copy of a journal fact; the
summary already derives `messages` this way (ADR-0005). `HostApi::fork`:
refused, a plugin replays history into a new session, a second record of
one conversation with new ids and new costs. Both doors open.

## Decision

1. **`SessionSummary.first_ask: Option<String>`** (≤200 chars), stamped
   where `messages` is counted by one pure `bingo_sdk::asked(&Item)`: a
   person's typed line, never a shell line or a wake note.
2. **`HostApi::fork(session, at: Option<TurnId>) -> SessionId`**: a new
   root journal of the source's frames through `at` (all when `None`),
   item ids kept, session rewritten, children not copied; `NOT_READY`
   while the source runs a turn. `forked_from: Option<ForkLink { session,
   turn }>` on `SessionSpec` and the summary. On the wire as
   `session/fork` (ADR-0007), not on the plugin bridge.
3. **An eighth site, `Sessions { query, scope: here | all, limit,
   sessions, current }`**, owner `bingo-mod-sessions`; terminal, desktop.
4. **Auto-title fills an empty title only**, at `prompt.submit`, through
   `SessionChange::Title`. Any title that exists was somebody's choice.
5. **One relative-time spelling**, `bingo_sdk::ago` (`4m ago`, `2h ago`,
   a date past 30 days), for the list and the welcome row alike.

## Consequences

- Old journals list with no first ask and no fork link; nothing migrates.
- A fork copies bytes. It is a root, not a child, and outlives its source.
- Deleting from the list is `HostApi::delete`, children first (ADR-0010 §6).
- The palette keeps twenty recents and gains `All sessions…`.

Refs: ADR-0005, ADR-0007, ADR-0010, ADR-0045, ADR-0082, ADR-0084, ADR-0086.
