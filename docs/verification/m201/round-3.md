# M201 round 3 verification

## Rebase and generated contracts

Rebased first with `git rebase 32d6113d`, without conflicts. Regenerated with
`BINGO_UPDATE_SCHEMA=1 cargo test ... --locked` for these targets:

- `-p bingo-sdk --lib --test why_and_time`: 231 and 2 passed.
- `-p bingo-plugin-rpc --lib`: 132 passed.
- `-p bingo-surface-rpc --lib schema`: 6 passed.
- `-p bingo-mods --lib schema_and_types_are_generated_from_the_same_rows`: 1 passed.

No generated schema or TypeScript differed from the rebased files. M193's adjacent
`wire.ts` fields survived regeneration; no generated correction needed folding.

## Regression evidence

`cargo test -p bingo-core --lib session::tests::recording --locked` first returned
`2 passed; 4 failed`. These four tests failed against the round-2 implementation:

- `publishing_does_not_rewrite_an_interruption`: publish stamped the marker.
- `emitted_receipts_refresh_once_even_when_already_recorded`: duplicate skipped refresh.
- `command_receipts_refresh_once_even_when_already_recorded`: new receipt skipped refresh.
- `an_answered_denial_refreshes_once`: answer's Deny skipped refresh.

All six recording tests pass after the fix. `gate_receipts_refresh_once_even_when_already_recorded`
was already green and guards the accepted behavior. Each path counts policy `describe`
calls; new and duplicate receipts each refresh exactly once, while duplicates publish
no extra receipt. Dedup is permission-only and retains the item's own turn.

`a_dropped_turn_closes_its_open_ask` drives the actor's `Ended::Dropped` path with
an open ask. It checks the stamp, consumption of `stopped_ask`, no invented answer,
and the final clock. It passed with the old publish writer. A deliberate mutation
removing the new dropped-path stamp made its stamp assertion fail; restoring it passed.
`emit` and `close_dropped_turn` now consume the stamp, and publish does no rewriting.

Strengthened guards for behavior that was already correct (no production fix needed):

- `a_read_matching_session_records_no_allow_reason`: confirms the installed policy
  returns Session Rule for a writable call and ReadOnly for a read; the writable call
  also traverses the gate without asking and retains that Session Rule.
- `two_answered_asks_keep_their_own_opening_instants`: matches receipts to the two
  opened-frame interaction IDs, checks each timestamp, and asserts distinct openings.
- SDK `successive_answered_asks_subtract_both_waits` already pins two waits with literal
  times: elapsed 1650 ms and next second at 4850 ms. No duplicate arithmetic test added.
- `recorded_pre_m201_frames_replay_without_provenance_or_waits`: replays the recorded
  journal plus the three unchanged pre-M201 contract bodies, embedded verbatim in
  frames (excluding their line terminators). All three wait-closing kinds are present,
  all lack `asked_at`; their turn is exactly 2 seconds with no subtraction. The original
  turn remains 6039 microseconds and the journal bytes remain unchanged.
- `a_queued_messages_clock_starts_when_its_turn_lands`: queues during a running turn;
  its item starts after that turn ends, and its clock excludes the queued interval.

The legacy test's construction initially failed on a newline inside an NDJSON frame,
then on an incomplete usage object that decoded as an unknown event. Both test-fixture
construction errors were corrected; the original committed fixtures were not changed.

Focused commands passed:
`cargo test -p bingo-core -p bingo-store-jsonl --lib --locked` (689, 74),
`cargo test -p bingo-sdk --test turn_clock --locked` (11), and
`cargo test -p bingo --test kernel provenance --locked` (4).

## Small refactors and recorded departures

- `agent_turn` delegates request assembly and is 49 lines; `resolve_check` stays 60.
- The unchanged `turn_start_shares_ten_thousand_bodies_without_a_journal` stays in
  `turn/tests/snapshots.rs`: `turn/tests.rs` has 985 lines; restoring its 28-line block and separator
  would exceed `scripts/check_discipline.sh`'s 1000-line file limit (section 3).
- A failed core prompter, including the judge's Mailbox ask, now ends the call with
  "Permission interaction closed" and no receipt. On dev the error became Cancel,
  "Permission denied by the user", and a Deny receipt. The call still never runs.
- Deny and ask rules record `source: Settings`: both lists come only from settings;
  the same `Reason::Rule` represents allow, deny and ask.
- M202 owns the remaining mismatch: the turn's private Interruption retains
  `asked_at: None`, while the journal gets the stamp. Turn-end hooks therefore see
  None on that path. M202's rebuilt turn walk should make them agree.
- The legacy HookOutcome has no Allow arm; full tool.check interceptors supply Hook allows.
- An edited call asked again then explicitly denied still retains the final ask's
  opening in the actor's Deny receipt, as accepted in round 2.

## Final checks

The exact command `git rebase -x 'cargo check --workspace --all-targets --locked' 32d6113d`
completed successfully: nine executions, nine `Finished dev profile` results. In order:

| Logical commit | Check |
|---|---|
| test(sdk): Pin provenance and wait timestamp contracts | exit 0 |
| feat(sdk): Add optional provenance and wait timestamps | exit 0 |
| feat(sdk): Derive turn clocks without human waits | exit 0 |
| feat(permissions): Preserve allow sources and read defaults | exit 0 |
| feat(core): Retain settled receipts and stopped waits | exit 0 |
| feat(tool-fs): Summarize original read sizes | exit 0 |
| test(store): Replay recorded pre-M201 journals | exit 0 |
| docs(docs): Record M201 review fixes and checks | exit 0 |
| docs(docs): Record final M201 validation | exit 0 |

All gates ran sequentially, with nothing excluded:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo check --workspace --all-targets --locked` | exit 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| `cargo test --workspace --locked` | exit 0; summed suites: 6508 passed, 0 failed, 0 ignored |
| `scripts/check_discipline.sh` | discipline ok; existing warnings |
| `scripts/budget.sh` | budget ok; 423 dependencies, 0s warm core, isolation 0; target 17 GB soft warning |
| `cargo deny check` | advisories ok, bans ok, licenses ok, sources ok |

`cargo test -p bingo --test cli --test rpc --test kernel --test plugin_kit --locked`:
CLI/print 309, RPC 104, kernel 32, plugin-kit 35 passed. No black-box re-pins.
`node --test sdk/typescript/src/*.test.ts`: 33 passed, 0 failed.
No wall-clock test failed; no isolated retry was needed.

Windows command for every touched crate:
`cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc --locked`.

- Exit 0: bingo-sdk, bingo-core, bingo-store-jsonl, bingo-agents, bingo-checkpoints,
  bingo-context, bingo-rooms.
- Exit 101, native headers: bingo-permissions (tree-sitter/tree-sitter-bash,
  stdio.h/stdlib.h); bingo-pictures, bingo-tool-fs, bingo-surface-print,
  bingo-surface-tui (aws-lc-sys, stdlib.h/windows.h).
- Exit 101: bingo hits those native header stops and rquickjs-sys passing MSVC
  flags (`/std:c11` and others) to the host cc.

The evidence-only final-doc amendment is also checked with
`cargo check --workspace --all-targets --locked`; no code changes follow the gates.
