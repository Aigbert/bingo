# M113 — The record is versioned

## Goal

ADR-0065. Unknown event/item payloads survive replay and serialization;
golden fixtures pin known shapes; version lines and raw-JSON migration
steps provide the seam for breaking changes. Readable historical shapes stay known.

## Bricks, in build order

1. **Fixtures first.** One fixture per known `Event` and `ItemBody` variant
   in `bingo-sdk/fixtures/record/{event,item}`; generated once from current
   serialization, generator deleted. Exhaustive helpers without wildcard
   arms select fixtures; removing a fixture fails compilation.
2. **Unknown contracts first.** Unknown tags and malformed known bodies
   retain their frame/item envelope and serialize byte-equal. Commit the
   fixtures/tests and observe failures before implementing tolerance.
3. **Decode.** Retain raw JSON, try the one derived vocabulary through
   serde remote, fall back to `Unknown { raw }`. The opaque wrapper has a
   private field; only deserialization creates it. Schema: opaque object.
4. **Fold.** Unknown events are durable, advance seq, return `Applied::Nothing`.
   Transcript retains unknown items; the model skips them; surfaces omit
   their presentation while raw JSON output retains the payload.
5. **Version lines.** `journal.rs` reads `{"format":"bingo-journal","version":N}`
   after the header; subsequent frames use N. Chain `fn(Value) -> Value`
   steps before decoding; no steps today, `VERSION` stays 1. Refuse newer
   versions naming both, including at the tail. Keep corruption/torn-tail
   rules. Test the append-only marker writer; no production caller until M115.
6. **Compatibility.** Synthetic unknown event and item-body tags fail with
   `STORAGE` before M113, then replay as Unknown and resume with known history.
   Separately test pre-`379bd04c`, pre-`4afd43d3`, pre-`6471cde6` shapes
   recovered via `git show <sha>^:crates/bingo-sdk/src/event.rs`: all stay known.
7. Accept ADR-0065; add dated notes to ADR-0005 §2–3 and ADR-0007.

## Files

- `crates/bingo-sdk/src/{event.rs,event/**,state.rs,transcript.rs}`,
  `crates/bingo-sdk/{Cargo.toml,fixtures/record/**,tests/**}`
- `crates/bingo-core/src/{context.rs,host/tests/resume.rs,host/tests/resume/**,test_support.rs}`
- `crates/bingo-store-jsonl/src/{journal.rs,journal/**}`, its fixtures
- Surface exhaustive matches and tests listed below; `crates/bingo/tests/versioned_record.rs`
- `docs/adr/{0065,0005,0007}*.md`, `schema/{rpc,plugin}.json`

## Exit criteria

- [x] Every known variant has a fixture; deleting one fails compilation.
- [x] Unknown/malformed payloads retain envelopes and serialize byte-equal.
- [x] Synthetic kinds fail with `STORAGE` before M113, then replay and resume;
      historical shapes stay known; model and surfaces ignore opaque payloads.
- [x] Non-JSON middle-line corruption still returns `STORAGE` naming the line.
- [ ] All gates: final PTY rewind test failed twice; see Verified and Risks.

## Non-goals

Bumping VERSION (M115), migrating already-readable historical shapes,
changing the wire beyond opaque Unknown, or calling the version writer in production.

## Risks

Raw capture/serde buffering adds decoding work; the byte-preservation tests
and warm-check budget cover it. `serde_json` gains its existing `raw_value`
feature to retain bytes, with no new crate dependency. No second live enum.

2026-10-13 stop (protocol §6): final workspace rerun failed the PTY rewind
assertion at `bingo/tests/pty/main.rs:574`; its isolated rerun timed out at
`:407` waiting for `Rewind to`. This timing/interaction test is outside Files;
its repair is not a mechanical changed-type update. Options: wait for M111's
wall-clock test work, then rerun; or authorize a scoped PTY synchronization
repair here. No test change attempted. ADR-0065 remains proposed; brick 7 waits.

## Re-verification and scope decisions (2026-10-13)

At `eddd0ebc`, M112's Transcript exists; Event has 23 known variants and
ItemBody 13, both in `event.rs`. Both RPC schemas consume them. Probe in
`face05fb`: five historical frames replayed, no STORAGE (`childSession`
ignored, display-summary becomes `View::Custom`, Question wire unchanged).
The user approved preserving those shapes and synthetic unknown-kind failures.

The standing M130 rule, approved here: mechanically update tests/fixtures
outside Files that only construct/name a changed type. M113 may also add
Unknown arms anywhere with ADR-0065 behavior; stop for any new behavior decision.
All exhaustive match sites updated:
- `bingo-core/src/test_support.rs::kind`: test label only.
- `bingo-core/src/context.rs::Folder::item`: skip model content.
- `bingo-sdk/src/state.rs::apply`: `Applied::Nothing`.
- `bingo-sdk/tests/record.rs::{fixture_name,item_fixture_name}`: reject
  Unknown in known fixtures (separate opaque fixtures cover Unknown).
- `bingo-surface-print/src/render.rs::{text,completed}`: silent.
- `bingo-surface-print/src/stream_json.rs::{line,completed}`: omit.
- `bingo-surface-tui/src/transcript.rs::item_lines`: empty block.
Additional coverage: print renderer raw/text/stream-json, TUI TestBackend,
`bingo/tests/versioned_record.rs` real RPC resume/replay and process exit.

## Contract evidence (2026-10-13)

```text
$ cargo test -p bingo-sdk --test record --locked # fixture temporarily removed
error: couldn't read `.../fixtures/record/event/notice.json`: No such file or directory
$ cargo test -p bingo-store-jsonl journal::compatibility --locked -- --nocapture # before
historical_shapes_stay_known ... ok
an_unknown_event_journal_replays ... FAILED # Storage, line 3: unknown variant `futureEvent`
an_unknown_item_journal_replays ... FAILED  # Storage, line 3: unknown variant `futureItem`
$ cargo test -p bingo-store-jsonl journal::versions --locked # before
version_lines_between_frames_do_not_take_a_sequence_number ... FAILED
[STORAGE] ... line 3: missing field `seq`
test result: FAILED. 1 passed; 2 failed
```

## Verified (2026-10-13; incomplete at the gate)

Contract failures above precede implementation. Passing commands/output:
```text
cargo test -p bingo-sdk --test record --locked: 2 passed; 0 failed
cargo test -p bingo-sdk --test unknown_record --locked: 6 passed; 0 failed
cargo test -p bingo-store-jsonl --locked: 49 passed; 0 failed
cargo test -p bingo-core a_journal_with_unknown_records --locked: 1 passed
cargo test -p bingo-surface-print -p bingo-surface-tui opaque --locked: 1 + 1 passed
cargo test -p bingo --test versioned_record --locked: 1 passed; 0 failed
cargo fmt --all -- --check: exit 0
cargo check --workspace --all-targets --locked: Finished dev profile in 4.89s
cargo clippy --workspace --all-targets --locked -- -D warnings: Finished in 30.85s
scripts/check_discipline.sh: discipline ok
scripts/budget.sh: dependencies 345/345; warm core 0s; relink 0; budget ok
cargo deny check: advisories ok, bans ok, licenses ok, sources ok
bash target/m113-tui-smoke.sh: tui-smoke ok (only SOCKET changed to a unique name)
```
Windows: `cargo tree -p <crate> -i aws-lc-sys --target x86_64-pc-windows-msvc`
was checked for every touched crate. `cargo check -p <crate> --all-targets
--target x86_64-pc-windows-msvc --locked` passed for sdk, core, store-jsonl,
surface-rpc. Surface-print, surface-tui, plugin-rpc and bingo have aws-lc-sys;
left to Windows CI per the standing decision, with no toolchain installed.

An earlier complete workspace run passed (5018 passed, 3 existing ignored,
1 filtered); the filtered ACP token test passed alone (1 passed). After moving
only the new resume test into a submodule to meet the discipline size limit:
```text
cargo test --workspace --locked -- --test-threads=1 --skip a_proxy_with_a_token_this_run_never_minted_gets_nothing
esc_twice_rewinds_the_turn_and_the_file_it_wrote ... FAILED
and the turn is out of the transcript (pty/main.rs:574)
test result: FAILED. 15 passed; 1 failed [PTY binary]
cargo test -p bingo --test pty --locked esc_twice_rewinds_the_turn_and_the_file_it_wrote -- --exact --test-threads=1
timed out waiting for "Rewind to" (pty/main.rs:407)
test result: FAILED. 0 passed; 1 failed; 15 filtered out
```
The earlier inherited `RUST_TEST_THREADS=1` run broke gateway child readiness;
removing that environment override and using `--test-threads=1` passed all 8
claim tests and the earlier full run. Logs remain in `target/m113-*.log`.
