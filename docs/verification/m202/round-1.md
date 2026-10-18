# M202 round 1 — kernel verification

Scope: bricks 1–5 and 7, from dev `8f0016be`. Brick 6 is deferred until M194
part B. No edits to `bingo-surface-tui` or `bingo-mod-ledger`; no dependency added.

## Implementation and evidence

| Brick | Implementation | Evidence |
| --- | --- | --- |
| 1 | `core/src/round.rs`: a pure five-action table; limit remains 10. | `round::tests::the_reached_call_table`, committed in `e290a457` before the walk. Six flight states × five decisions. |
| 2 | `gate.rs`: `decide` returns a settled gate or an ask with scope, reason, blocked message and changed input. `gate_call` composes that decision with the single asking/change loop. | Existing gate, hook-contract, judge, invoke and provenance tests; `changed_calls_repeat_policy_and_keep_the_replacement`, `cargo_test_then_rm_cannot_escape_the_denial_by_changing_input`, `a_denied_replacement_can_be_changed_again`, `changed_session_approval_keeps_both_receipt_facts`, `a_failed_prompter_closes_the_call_without_a_receipt`. |
| 3 | `turn/round.rs`: decide at reach, hold the walk on a waiting call, drain before asking, record receipts at resolution, mark Running at admission. | `read_lands_before_edit_asks_and_edit_lands_before_bash_is_reached`, `yes_for_the_session_answers_the_next_edit_in_the_same_round`, `a_denial_lands_without_waiting_and_does_not_stop_the_walk`. |
| 4 | `executor.rs`: `InFlight` polls each call at admission, yields landings individually and derives its occupancy from its futures. The round returns outcomes in input order. | `start_polls_the_call_and_each_landing_frees_its_own_slot`, `each_safe_call_lands_at_once_and_the_model_keeps_input_order`, `a_landing_frees_one_slot_without_waiting_for_the_window`, `after_hooks_see_the_whole_round_once_in_input_order`. |
| 5 | Every leaf await in the walk races cancellation. Cancelled calls finish through the unchanged `run_one` biased branch; unreached calls get interrupted outcomes without a decision. | `esc_during_an_ask_never_decides_or_receipts_a_later_call`, `esc_drops_parallel_calls_and_never_reaches_the_call_after_a_writer`, `a_landing_is_reported_while_a_check_waits_and_esc_drops_that_check`, `esc_drops_a_receipt_wait_before_any_later_call_is_decided`, `esc_drops_after_hooks_and_keeps_calls_that_landed`. |
| 7 | `session/running.rs`: one shared `StoppedAsk`, written before cancellation and read through `TurnHost` by both turn builders and dropped-turn cleanup. No rewrite in `emit`. | `permission_stop_agrees_in_outcome_hooks_and_journal`, `question_stop_agrees_in_outcome_hooks_and_journal`, `stop_without_an_ask_agrees_in_outcome_hooks_and_journal`, `closing_with_an_ask_agrees_in_outcome_hooks_and_journal`, `a_dropped_turn_closes_its_open_ask`. |

Round tool doubles are released by oneshot channels. Cancellation tests synchronize on
entry channels; no wall clock releases a tool. The old executor batch tests were replaced
by primitive tests and round tests, including channel-controlled ordering and cancellation.

## Esc before and after

Before implementation, `esc_during_an_ask_currently_decides_and_receipts_a_later_call`
ran against the original kernel: two before-tool checks, one receipt from the later call,
and both calls Interrupted. The contract run passed 17 tests, including that baseline.

The repinned test now requires one check, zero receipts, and both calls Interrupted.
The actor still cancels the open interaction; no Deny or QuestionAnswer is invented for
an unanswered ask. Landed calls remain completed. After-tool hooks still observe the
whole successful round once, and cancellation drops/skips them as before.

Before brick 7, the new permission, question and close tests failed at the turn outcome:
`None` instead of the open ask's timestamp. The no-ask case passed. After the change, the
outcome, turn-end hook items and journal agree. Actor-written Deny receipts still survive
an immediately following esc, retain the ask's turn, and are deduplicated at recording.

Accepted cost: stopping during the second ask of a changed call records that second
ask's `opened_at`, the earliest currently open ask naming the turn. It does not backdate
the interruption to the first ask. Completed permission receipts still preserve the first
ask's time across re-asks and name the latest interaction id.

## M201 review repairs

- `record_item` and its receipt deduplication live beside `emit` in `session.rs`.
  Only `Actor::receipt` refuses a missing or ended turn; ordinary `record_for` works
  between turns. `a_receipt_without_a_turn_is_refused_but_record_for_works_between_turns`
  pins the Interrupted error and the between-turn recording path.
- `an_answered_denial_refreshes_once` also asserts `receipt.turn == opened.turn`.
- `bingo/tests/kernel/provenance.rs` keeps policy assertions in
  `a_read_matching_session_records_no_allow_reason`, not the shared probe.

## Risk audit

- **Hooks:** inspected the `Check` dispatcher, adapter, policy link and tier baseline
  checks (`chain/tool.rs`, `chain/tier.rs`), and every shipped before-tool implementation.
  Checkpoints snapshot a writer's paths; earlier exclusive writers now land before that
  snapshot. Shell hooks use per-invocation payloads and subprocess calls. Context memory
  rows use the current item and session snapshot. No implementation requires an idle
  round. RPC links use distinct chain ids and cancellable requests. Module links use
  per-call continuations; `bingo-mods/src/thread.rs` services concurrent jobs through
  `FuturesUnordered`, including a check while another call awaits `next`. The shipped
  TypeScript check examples inspect only their call input. Arbitrary third-party hooks
  remain governed by the declared concurrency traits; this adds no isolation guarantee.
- **Teammates:** child sessions use `run_turn` and the same round walk. The child actor
  still publishes InteractionOpened and its asking summary, which the root tree forwards.
  The existing core test
  `a_child_opened_after_the_attachment_streams_from_its_head_and_is_answered_through_the_root`
  and agents test `an_open_prompt_lists_as_asking` cover the parent-facing path. No new
  parent state or inferred ask is introduced.
- **Esc latency:** hook gathering, deciding (including concurrent landings), asking,
  receipt recording, landing waits and after-tool hooks are all raced. Cancellation
  cleanup polls only `run_one`'s ready cancellation branch, with no tool poll or await.
  The actor remembers the earliest open ask before interrupt, close (including its own
  token), abandonment and storage failure; dropped-turn cleanup reads the same value.
- **Old journals:** recovery previously ended the turn but left its Pending/Running items
  open. `session/recovery.rs` now completes every unfinished item as Interrupted before
  TurnLost. `resumed_pending_calls_close_before_the_lost_turn` resumes both statuses and
  checks frame order. Existing journal/record formats are unchanged.

## Departures and boundaries

- No planner decision was reopened and no prototype conflict was found.
- Removed `execute_chain` and `execute`: after the walk replaces their sole production
  caller, only their own tests remained. Primitive coverage moved to `executor/tests.rs`;
  scheduling coverage moved to the actual round. `run_one` is unchanged.
- Put the walk in `turn/round.rs` to keep `turn.rs` focused. Moving `record_item` into
  `session.rs` crossed its 1,000-line discipline limit, so recovery moved to its own
  module. These modules own free functions, keeping Actor and Turn within the existing
  three-file inherent-impl limit. The gate and executor use only existing dependencies.
- Brick 6, its TUI scenario and terminal-byte validation belong to round 2. No captures
  were edited and no terminal application was launched.

## Commands and results

All commands were run sequentially; no concurrent Cargo invocations from this worktree.
The first workspace run passed all 6,522 tests across 125 test executables.

One subsequent workspace run timed out in
`agents::a_resumed_root_is_told_which_child_was_mid_turn_when_the_process_ended`:

```text
the resumed root did not finish recovering the interrupted child
stdout:
stderr:
test result: FAILED. 308 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 64.32s
```

The test's 60-second deadline expired. Reran it once, alone, without changing it:

```text
$ cargo test -p bingo --test cli agents::a_resumed_root_is_told_which_child_was_mid_turn_when_the_process_ended --locked -- --exact --nocapture
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 308 filtered out; finished in 0.78s
```

M194's existing verification also records this recovery-test timeout. No process outside
a test's own children was signalled and no test deadline was changed. The final full
workspace rerun is recorded below.

Initial verification found two implementation issues, both fixed: Clippy's large enum
warning (boxed the gate-decision payloads) and discipline's inherent-impl/file-size limits
(module-owned round and recovery functions). No gate was relaxed.

```text
$ cargo fmt --all -- --check
(exit 0; no output)
$ cargo check --workspace --all-targets --locked
Finished `dev` profile [unoptimized + debuginfo] target(s) in 14.80s
$ cargo clippy --workspace --all-targets --locked -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 19.19s
$ scripts/check_discipline.sh
discipline ok
$ scripts/budget.sh
dependencies (unique, normal): 423 (max  423)
warm cargo check -p bingo-core: 0s (max  20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
target/debug: 7 GB (soft max  5)
warn: target/debug exceeds the soft limit
test binaries: 128
budget ok
$ cargo deny check
advisories ok, bans ok, licenses ok, sources ok
$ cargo check -p bingo-core --all-targets --target x86_64-pc-windows-msvc --locked
Finished `dev` profile [unoptimized + debuginfo] target(s) in 17.35s
```

Discipline has existing advisories about long files/functions and orphan snapshot names;
no new production function exceeds its warning threshold. Budget has the debug-directory soft warning
shown above; no dependencies were added.

The other touched crate is `bingo` (its provenance test). Its Windows check was attempted:

```text
$ cargo check -p bingo --all-targets --target x86_64-pc-windows-msvc --locked
error: failed to run custom build command for `rquickjs-sys v0.14.0`
clang: error: no such file or directory: '/DWIN32_LEAN_AND_MEAN'
clang: error: no such file or directory: '/std:c11'
clang: error: no such file or directory: '/experimental:c11atomics'
error: failed to run custom build command for `aws-lc-sys v0.44.0`
fatal error: 'stdlib.h' file not found
fatal error: 'windows.h' file not found
```

This is the permitted native cross-toolchain stop, not a successful Windows check for
`bingo`. No Windows-specific Rust API was added; the kernel cross-check passed.

Final full rerun, on the module-owned implementation:

```text
$ cargo test --workspace --locked
Running tests/cli/main.rs (target/debug/deps/cli-642f2c93eed268f4)
test result: ok. 309 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 22.60s
Running tests/kernel/main.rs (target/debug/deps/kernel-10de544286a7388c)
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
Running tests/rpc/main.rs (target/debug/deps/rpc-a04f4d2dcf12e295)
test result: ok. 104 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.05s
Running unittests src/lib.rs (target/debug/deps/bingo_core-cee99a17c66e0088)
test result: ok. 703 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.90s
Running unittests src/lib.rs (target/debug/deps/bingo_jsonrpc-53833d96d212762b)
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
Running unittests src/lib.rs (target/debug/deps/bingo_plugin_rpc-8a35b749feb4e2ab)
test result: ok. 132 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.34s
Running unittests src/lib.rs (target/debug/deps/bingo_surface_rpc-41f9de6ad5e4b07c)
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
Running tests/wire/main.rs (target/debug/deps/wire-be5120e3e5103ad3)
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.65s
```

All 125 test executables completed: 6,522 passed, zero failed, zero ignored (sum of
Cargo's result summaries). The recovery test also passed in this full rerun.

## Wire coverage and re-pins

No black-box expectation was re-pinned. Only the kernel's esc baseline was re-pinned,
for the intentional removal of the later check and receipt. The executor's fixed-batch
expectations were replaced with primitive/round coverage of sliding admission.

The workspace run includes the actual CLI (309 tests), RPC integration (104), kernel
integration (32), RPC wire (43), print surface (103), ACP provider (152), channels (116),
channel adapter (35) and Feishu adapter (138) suites. The fake ACP agent executable was
present, so the ACP black-box tests were not skipped for a missing fixture.

Examples of unchanged black-box contracts that passed:

- `print_streams_prose_to_stdout_and_nothing_else`;
  `stream_json::stream_json_is_init_then_messages_then_result_and_nothing_else`.
- `rpc::stdout_carries_json_rpc_lines_only_and_an_unknown_method_is_refused`;
  `rpc::open_submit_and_the_events_arrive_in_seq_order_with_the_clients_intent`;
  `rpc::a_permission_is_answered_over_the_wire_and_the_tool_runs`.
- `acp::bridge::an_interrupt_drops_a_bridged_call_and_the_child_serves_the_next_turn`;
  `acp_asked::a_session_allow_answers_twice_with_once_and_asks_the_person_once`.
- `channels::a_message_opens_a_session_and_the_answer_streams_into_one_edited_message`;
  `channels::a_permission_is_buttons_and_a_click_answers_it`;
  `channels::a_question_answered_at_another_surface_edits_the_card`.
