# ADR-0065 — The record is not the live type

Status: proposed · 2026-10-13 · Plan: M113 · Amends: ADR-0005 §2–3

## Context

ADR-0005 §2 promised that a format change is a new `version` and a
migrator. The header has said `version: 1` since the first journal
(`bingo-store-jsonl/src/journal.rs`, `VERSION`), while the type it
serialises has changed: `379bd04c` removed the optional `childSession`
field, `4afd43d3` replaced `Display` with `View`, and `6471cde6`
extracted `Question` without changing its wire shape. Re-verification
found all three still readable. The journal line is whatever `serde`
makes of today's `Event`; a genuinely unknown event or item kind instead
fails `replay` with `STORAGE` on a middle line (§3: "an unreadable line
anywhere else is a STORAGE error"). Thus a newer binary adding a variant
can write a journal an older binary cannot resume.

The fixture `bingo-core/fixtures/frames-v1.jsonl` covers part of the
vocabulary; nothing makes a change to `Event` or `ItemBody` fail a test
when it changes how an existing record reads.

## Decision

1. **The envelope is the record; the event is a payload.** A journal line
   is a `Frame` envelope (`seq`, `ts`, `session`, `cause`) around an
   `event`. A line that is not JSON, or whose envelope does not parse, is
   corruption, exactly as §3 says (a torn last line is still dropped).
   A line whose envelope parses and whose `event` does not is **kept**,
   as `Event::Unknown { raw }`: the JSON verbatim.
2. **The same one level down.** An item whose `body` does not parse is
   kept as `ItemBody::Unknown { raw }`, with its id, turn and status.
3. **Unknown is carried, never interpreted.** Reducers ignore an unknown
   event; the transcript keeps an unknown item in place and the model
   view skips it. Serialising an unknown writes `raw` back byte for
   byte, so replay to a client and a re-read journal are unchanged. No
   producer may construct one; only deserialisation does.
4. **Golden fixtures are the contract.** Every `Event` variant and every
   `ItemBody` variant has a fixture line under
   `bingo-sdk/fixtures/record/`. A test reads each into its known variant
   (never `Unknown`) and writes it back equal; an exhaustive `match`
   helper with no wildcard arm makes a new variant without a fixture a
   compile error in that test.
5. **What bumps the version.** Adding a variant or an optional field is
   not breaking: older readers keep it as unknown. Renaming, removing or
   restructuring is breaking, and is detected by a fixture that no
   longer reads; it requires `VERSION + 1` and a step
   `fn(serde_json::Value) -> serde_json::Value` from the old shape to the
   new, applied to raw lines before they are deserialised. Steps chain.
6. **A journal is never rewritten; a version line is appended.** When a
   binary appends to a journal whose last version is older than its own,
   it first appends a version line (the header's shape, without
   `session`); lines after it are read at that version. A reader meeting
   a version newer than its own refuses with `STORAGE` naming both, as
   today. The reader for version lines ships with this ADR, before any
   writer needs one.
7. **No migrator for the past.** The shapes removed before this ADR read
   as `Unknown` when this build cannot read them; their sessions open.
   2026-10-13 clarification: the three cited changes remain known, as the
   M113 `cited_historical_shapes_already_replay` probe at `eddd0ebc`
   demonstrated (5 frames replayed, no `STORAGE`; recorded in `face05fb`).
   `Unknown` covers a kind this build does not know, for example a record
   written by a newer build; already-readable shapes stay known.

## Consequences

- A session written before 2026-09-04 resumes instead of failing; the
  frames whose shape left the vocabulary are visible as unknown to a
  client that asks, and to nobody else.
- An unknown event takes a `seq` like any other, so `seq` stays gapless
  for durable frames (ADR-0002 §2).
- Every `Event` or `ItemBody` change now touches a fixture; a breaking
  change also touches the migration chain. That cost is the point.
- `Event` gains a variant clients must tolerate. The RPC schema
  (ADR-0007) documents it as opaque; a client that matches the event
  type exhaustively gains one arm that ignores it.
- ADR-0067 is the first breaking change (pictures become references) and
  the first version line.

Refs: ADR-0002, ADR-0005, ADR-0007, ADR-0067; Plan: M113; audit-2026-10 §1
