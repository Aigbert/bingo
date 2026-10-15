# ADR-0089 — A session is forked from the list

Status: accepted · 2026-10-15 · Plan: M175 · Amends: ADR-0084 §1, §6

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

1. **`SessionSummary.history.first_ask: Option<String>`** (≤200 chars), stamped
   where `messages` is counted by one pure `bingo_sdk::asked(&Item)`: a
   person's full Said text; `first_ask` takes its first line, never a shell or wake note.
2. **`HostApi::fork(session, at: Option<TurnId>) -> SessionId`**: a new
   root journal of the source's frames through `at` (latest completed stored turn when `None`),
   item ids kept, session rewritten, children not copied; `NOT_READY`
   while the source runs a turn. `forked_from: Option<ForkLink { session,
   turn }>` on the summary only; Create cannot claim provenance. On the wire as
   `session/fork` (ADR-0007), not on the plugin bridge.
3. **An eighth site, `Sessions { query, scope: here | all, limit,
   sessions, current }`**, owner `bingo-mod-sessions`; terminal, desktop.
4. **Auto-title fills an empty title only**, after admission at `prompt.submit`, through
   `SessionChange::Title`, with targeted summary lookup. Errors are logged and do not
   reject admitted input. Any title that exists was somebody's choice.
5. **One relative-time spelling**, `bingo_sdk::ago` (`4m ago`, `2h ago`,
   a date past 30 days), for the list and the welcome row alike.

## Consequences

- The durable completion event defines a fork cut. Only an already-live source uses
  the actor; stored preview/fork replay read-only, without claims, repairs or hooks.
  A foreign-locked mid-turn source refuses; an unowned incomplete tail is excluded.
- Targeted `HostApi::summary` and `read` inspect without revival. Stored title
  reconfiguration briefly claims the store, appends one summary and releases it;
  activity time is preserved. Unsupported stores/hosts fail closed.
- Delete reserves idle live actors and claims every stored descendant before any
  removal; refusal cancels all reservations. Confirmation marks busy/locked rows.
- `SessionHistory` groups message count, first ask and fork origin in Rust; serde
  flattens it so their wire keys remain top-level and additive.
- Old journals list with no first ask and no fork link; nothing migrates.
- A fork copies bytes. It is a root, not a child, and outlives its source.
- Deleting from the list is `HostApi::delete`, children first (ADR-0010 §6).
- Selection uses stable row identities; rename is an inline input, not a session switch.
- The palette keeps twenty recents and gains `All sessions…`.

Refs: ADR-0005, ADR-0007, ADR-0010, ADR-0045, ADR-0082, ADR-0084, ADR-0086.
