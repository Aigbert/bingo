# ADR-0077 — One lifecycle vocabulary for an agent

Status: accepted · 2026-10-14 · Plan: M135 · Amends: ADR-0027, ADR-0029, ADR-0060

## Context

What an agent is doing has four vocabularies (audit, Also). The model
reads `busy` or `idle` (`bingo-agents/src/list.rs` `state`, its guide);
`/team` adds `not seated`; the TUI tree says `running`, `done`, `idle`,
`stored`, and under a row `Needs you`, `Running…`, `Done (…)`,
`Starting…` (`tree.rs` `Status`, `activity`); rooms say `live` and
`listening`, which are how a seat hears, not what it does. ADR-0029 was
amended five times and ADR-0060 left "the state words beyond busy/idle"
undecided. The same child is `busy` to the model that spawned it and
`Needs you` to the person — and the model, told `busy`, sends a message
that waits behind a permission prompt it has no word for.

The waiting is the gap. A child's open interaction lives in its
`SessionState`; a host listing (`HostApi::sessions`) carries
`SessionSummary`s and no state, so a model-facing roster cannot say it.

**The ratchet.** One kernel door: `SessionSummary.asking`. Refusing it
forces every listing reader — the agents tools, `/team`, a room's roster
— to open and fold each child's stream to learn whether it waits on an
answer: a second fold of a fact the actor already holds. Yes: the door
opens, beside `busy`, by the same rule.

## Decision

1. **Five words, one type.** `Lifecycle { Working, Asking, Idle, Stored,
   Unseated }` in `bingo-collab` (ADR-0071), spoken `working`,
   `asking`, `idle`, `stored`, `not seated`, from one table. Decided in
   this order, the first that holds:
   - no session holds the seat → `Unseated`;
   - the session's driver is `Log` → no lifecycle at all (ADR-0011 §1);
   - it is not open in this process → `Stored`;
   - an interaction of it is open → `Asking`;
   - a turn of it is running → `Working`;
   - else → `Idle`.
2. **Read from either source.** `Lifecycle::of_state(&SessionState)` reads
   the state's own interactions and turn, as `SessionState::busy` does;
   `Lifecycle::of_summary(&SessionSummary)` reads the stamps. A summary
   cannot tell an open session from a stored one — the host merges both
   in one listing — so a listing never says `stored`: to a model a stored
   child is `idle`, which is what a message to it finds (delivery
   reopens it).
3. **`SessionSummary.asking: bool`.** Stamped by the session actor in
   `stamped()` beside `busy` and from the same place: it is true while
   the actor's interaction table is non-empty. The actor republishes its
   summary when an interaction opens or closes, as it does when a turn
   starts or ends. Nothing else writes it; an old record reads `false`.
4. **Retired words.** `busy`, `running`, `done`, `Starting…` and
   `Needs you` go from every surface and from model-facing text. How the
   last turn ended is said beside `idle` in the `TurnStatus`'s own word
   — `idle · completed`, `idle · interrupted`, `idle · failed` — with
   what was spent after it, and a child that never ran says what it was
   asked beside `idle`. The state word is never replaced by its detail.
5. **What is not lifecycle stays apart.** A seat's ear (`live`,
   `listening · Ns`), a room's `owes an answer`, and the TUI's attention
   and unread marks are their own facts, said beside the state word.
6. **Who speaks it.** The agents tools' `state` field and guide, `/team`'s
   column, and the TUI tree and roster. Rooms name seats and ears, no
   lifecycle state, and gain none. Channels read `busy` to decide and
   say no state word; they keep reading it.
   ADR-0060's refusals read the words: `StopAgent` ends `working` and
   `asking`; `DismissAgent` refuses both.

## Consequences

- One word for one fact in every place a person or a model reads it; a
  model told `asking` knows a message waits behind a person.
- The model-facing `state` values change (`busy` → `working`). Old
  tool results in journals keep the old word; they are history.
- Republishing on every interaction adds two `SessionUpdated` records
  per permission prompt; they are small and already the summary's way.
- The TUI depends on `bingo-collab` for the word table (it already reads
  seats through it, ADR-0071).

Refs: ADR-0011, ADR-0024, ADR-0027, ADR-0029, ADR-0060, ADR-0071,
audit-2026-10 Also.
Non-goals: a state for a turn waiting on a hook or a deadline; colours or
glyphs per state (the TUI's own); a lifecycle for a `Log` session.
