# M113 — The record is versioned

## Goal

ADR-0065. A journal line whose event or item body this build does not
know is kept verbatim instead of failing the whole session; every
`Event` and `ItemBody` variant has a golden fixture, so a change that
alters an existing record fails a test; a breaking change has a place
to go (a version line and a raw-JSON step). Sessions written before
2026-09-04 open again.

## Bricks, in build order

1. **Fixtures first.** `crates/bingo-sdk/fixtures/record/event/<variant>.json`
   and `.../item/<kind>.json`, one per variant, written from today's
   serialisation (generate once with a throwaway test, then commit the
   files and delete the generator). A test reads each into its known
   variant and writes it back equal. An exhaustive helper
   `fn fixture_name(e: &Event) -> &'static str` with no wildcard arm is
   the compile-time check that a new variant needs a fixture.
2. **Unknown tolerance, failing test first.** Fixtures for an unknown
   event `type`, a known `type` with fields that do not parse, and the
   same for `ItemBody` `kind`. Expected: the `Frame` envelope parses;
   `event` is `Event::Unknown { raw }`; an item keeps id, turn and
   status with `ItemBody::Unknown { raw }`.
3. **The implementation.** `Event` and `ItemBody` get a hand-written
   `Deserialize`: deserialise into `serde_json::Value`, try the derived
   shape (the existing enum renamed behind a private `Known` type or via
   `#[serde(remote)]`, whichever keeps the schema derive working), and
   fall back to `Unknown`. `Serialize` writes `raw` back verbatim.
   `JsonSchema` documents `Unknown` as an opaque object. Constructing
   `Unknown` outside deserialisation is impossible (private field).
4. **Reducers ignore it.** `SessionState::apply` → `Applied::Nothing`;
   `Transcript` keeps an unknown item in place; the model view skips it;
   `is_durable` treats `Unknown` as durable (it came from a journal).
   Exhaustive matches in surfaces gain one ignoring arm.
5. **The version line.** `bingo-store-jsonl/src/journal.rs` `frames()`
   recognises a line `{"format":"bingo-journal","version":N}` anywhere
   after the header and reads following lines at version N; a migration
   chain `steps: &[fn(Value) -> Value]` (empty today) runs on each raw
   line from its version to `VERSION` before deserialising. A version
   newer than `VERSION` is `STORAGE` naming both, as today. The writer
   that appends a version line is written and tested but called by no
   one until M115. Corruption rules (ADR-0005 §3) unchanged; tests for
   a torn last line and a bad middle line stay green.
6. **Old journals open.** A fixture journal built from the shapes
   `379bd04c`, `4afd43d3` and `6471cde6` removed (recover them with
   `git show <sha>^:crates/bingo-sdk/src/event.rs`; write a minimal
   frame of each by hand) replays; those frames are `Unknown`, the rest
   fold. Resume of it in a core test succeeds.
7. ADR-0065 accepted; dated notes on ADR-0005 §2–3 and ADR-0007.

## Files

- `crates/bingo-sdk/src/{event.rs,item*.rs}` (wherever `ItemBody`
  lives), `crates/bingo-sdk/fixtures/record/**`
- `crates/bingo-sdk/src/{state.rs,transcript.rs}`,
  `crates/bingo-core/src/context.rs`
- `crates/bingo-store-jsonl/src/journal.rs` and tests,
  `crates/bingo-store-jsonl/fixtures/` (old-shape journal)
- surfaces with exhaustive `Event`/`ItemBody` matches
- `docs/adr/{0065,0005,0007}*.md`, the RPC schema snapshot if one exists

## Exit criteria

- [ ] One fixture per `Event` and `ItemBody` variant; deleting one fails
      the build or a test (pasted).
- [ ] Unknown and malformed-body fixtures read as `Unknown` and
      re-serialise byte-equal.
- [ ] The old-shape journal replays and resumes (test output pasted;
      before this milestone the same test fails with `STORAGE`).
- [ ] A non-JSON middle line still fails with `STORAGE` naming the line.
- [ ] All gates.

## Non-goals

- Bumping `VERSION` (M115 is the first). Migrating old shapes into new
  ones (ADR-0065 §7: they read as unknown).
- Changing the wire beyond the opaque `Unknown`.

## Risks

- A hand-written `Deserialize` on a large tagged enum costs compile time
  and may disturb `schemars`; `scripts/budget.sh` checks the warm
  `cargo check -p bingo-core` time. If it breaks the budget, stop.
- Trying the known shape on every line doubles parsing for unknown ones
  only; known lines parse once from the `Value`.
- Re-verify at start: M112 has landed; variant counts; where `ItemBody`
  is defined.


## Re-verification stopped (2026-10-13, `eddd0ebc`)

Protocol steps 2 and 6: the historical compatibility requirement in brick 6
and the third exit criterion contradict the current reader. No production
code was changed; ADR-0065 remains proposed and every exit stays unticked.
M112's `Transcript` exists; `Event` has 23 variants and `ItemBody` has 13,
both in `bingo-sdk/src/event.rs`. Both `schema/rpc.json` and
`schema/plugin.json` consume these types.

### Risks: compatibility decision needs resolution

The cited changes do not produce the promised pre-M113 `STORAGE` failure:

- `git show 379bd04c -- crates/bingo-sdk/src/event.rs`: removes the optional
  `ToolCall.child_session` field, not an event variant. Today's serde reader
  ignores `childSession` and returns a known `ToolCall`.
- `git show 4afd43d3 -- crates/bingo-sdk/src/event.rs`: replaces `Display`
  with `View`. The old `summary` display now reads through the existing
  `View::Custom` fallback (`bingo-sdk/src/view.rs`); its item stays known.
- `git show 6471cde6 -- crates/bingo-sdk/src/event.rs`: extracts `Question`
  with its wire shape explicitly preserved. The existing sdk test
  `a_bare_question_on_the_wire_is_the_shape_it_always_was` pins that fact.
  This old JSON cannot become `Unknown` while identical current JSON stays
  known. ADR-0065's context describes this extraction as breaking, and its
  historical-unknown decision (§7), as instantiated by brick 6, needs
  clarification before implementation.

Options (not applied): (1) retain these already-readable shapes as known,
correct the historical premise in ADR-0065 and brick 6, and use genuinely
unreadable historical records or explicitly synthetic unknown records for
the pre-change failure; (2) require legacy shapes to become unknown, which
needs a new compatibility decision about existing `View` behavior and the
indistinguishable Question wire shape. Do not silently change either.

### Re-verification evidence (not milestone verification)

A temporary journal unit test used minimal frames recovered from each
`git show <sha>^:crates/bingo-sdk/src/event.rs`, surrounded by the clean
fixture's header/session head and a final valid close frame. Thus no old
frame could disappear under the torn-tail rule. It asserted the known
variants and all five replayed frames. The temporary probe was removed.
Its first run had an incorrectly encoded `AnswerSpec`; after correcting
that probe-only typo to the historical strings, the result was:

```text
$ cargo test -p bingo-store-jsonl cited_historical_shapes_already_replay --locked -- --nocapture
379bd04c: known ToolCall; 4afd43d3: known ToolCall with View::Custom; 6471cde6: known Question; replay: Ok(5 frames), no STORAGE
test journal::m113_reverification::cited_historical_shapes_already_replay ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 39 filtered out
```

The milestone gates, Windows checks, fixtures and implementation were not
run/completed after this protocol stop. No ADR was accepted.
