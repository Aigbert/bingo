# M113 — The record is versioned

## Goal

ADR-0065. A journal line whose event or item body this build does not
know is kept verbatim instead of failing the whole session; every
`Event` and `ItemBody` variant has a golden fixture, so a change that
alters an existing record fails a test; a breaking change has a place
to go (a version line and a raw-JSON step). Already-readable historical records stay known.

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
   `Deserialize`: capture raw JSON bytes, try the derived
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
6. **Compatibility journals open.** Synthetic unknown event `type` and
   item-body `kind` fixtures each fail replay with `STORAGE` before M113,
   then replay as `Unknown`; a core resume test retains the known history.
   Keep a separate replay test for the pre-`379bd04c`, pre-`4afd43d3` and
   pre-`6471cde6` shapes: all three remain known (recover with
   `git show <sha>^:crates/bingo-sdk/src/event.rs`).
7. ADR-0065 accepted; dated notes on ADR-0005 §2–3 and ADR-0007.

## Files

- `crates/bingo-sdk/src/event.rs`, its record helpers/tests and
  `crates/bingo-sdk/{Cargo.toml,fixtures/record/**,tests/**}`
- `crates/bingo-sdk/src/{state.rs,transcript.rs}`,
  `crates/bingo-core/src/{context.rs,host/tests/resume.rs}`
- `crates/bingo-store-jsonl/src/journal.rs` and tests,
  `crates/bingo-store-jsonl/fixtures/` (old-shape journal)
- surfaces with exhaustive `Event`/`ItemBody` matches
- `docs/adr/{0065,0005,0007}*.md`, `schema/{rpc,plugin}.json`

## Exit criteria

- [ ] One fixture per `Event` and `ItemBody` variant; deleting one fails
      the build or a test (pasted).
- [ ] Unknown and malformed-body fixtures read as `Unknown` and
      re-serialise byte-equal.
- [ ] Synthetic unknown event and item journals fail with `STORAGE` before
      M113, then replay as `Unknown` and resume; historical shapes stay known.
- [ ] A non-JSON middle line still fails with `STORAGE` naming the line.
- [ ] All gates.

## Non-goals

- Bumping `VERSION` (M115 is the first). Migrating old shapes into new
  ones (ADR-0065 §7: readable shapes stay known).
- Changing the wire beyond the opaque `Unknown`.

## Risks

- A hand-written `Deserialize` on a large tagged enum costs compile time
  and may disturb `schemars`; `scripts/budget.sh` checks the warm
  `cargo check -p bingo-core` time. If it breaks the budget, stop.
- Raw capture and serde buffering add decoding work; byte-preservation
  tests and the warm-check budget cover the implementation tradeoff.
- Re-verify at start: M112 has landed; variant counts; where `ItemBody`
  is defined.


## Re-verification (2026-10-13, `eddd0ebc`)

M112's `Transcript` exists; `Event` has 23 variants and `ItemBody` 13,
both in `event.rs`. Both RPC schemas consume them. The historical probe
recorded in `face05fb` replayed five frames successfully: `childSession`
is ignored, old display-summary becomes `View::Custom`, Question keeps
its wire shape. The user approved option (1): preserve these known forms
and prove the pre-change `STORAGE` failure with synthetic unknown kinds.
The protocol stop is resolved; continue at contracts and failing tests.

Byte preservation requires retaining raw JSON alongside the attempt to
read a known shape: enable the existing `serde_json` dependency's
`raw_value` feature (no new crate). An opaque wrapper's field is private;
only deserialisation creates it. No duplicated live enum is needed.


## Scope decision (2026-10-13)

The user approved the standing M130 rule: mechanically update tests and
fixtures outside Files that only construct/name a changed type. For M113,
exhaustive matches anywhere may gain the ADR-0065 Unknown behavior; list
every site here. Stop only when a site's behavior needs a new decision.
The test-helper blocker is resolved. Additional sites so far:
- `crates/bingo-core/src/test_support.rs::kind`: label Unknown in tests.

Evidence collected so far (trimmed; these are not all milestone gates):

```text
$ cargo test -p bingo-sdk --test record --locked  # fixture temporarily removed
error: couldn't read `.../fixtures/record/event/notice.json`: No such file or directory
$ cargo test -p bingo-store-jsonl journal::compatibility --locked -- --nocapture # before implementation
historical_shapes_stay_known ... ok
an_unknown_event_journal_replays ... FAILED  # Storage, line 3: unknown variant `futureEvent`
an_unknown_item_journal_replays ... FAILED   # Storage, line 3: unknown variant `futureItem`
$ cargo test -p bingo-sdk --test record --test unknown_record --locked # after implementation
record: test result: ok. 2 passed; 0 failed
unknown_record: test result: ok. 2 passed; 0 failed
$ cargo check --workspace --all-targets --locked
error[E0004]: non-exhaustive patterns: `&bingo_sdk::ItemBody::Unknown { .. }` not covered
  --> crates/bingo-core/src/test_support.rs:273:22
error: could not compile `bingo-core` (lib test) due to 1 previous error
```
