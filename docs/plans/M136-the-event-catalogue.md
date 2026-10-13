# M136 — The event catalogue

## Goal

ADR-0078 §1: one SDK catalogue names kernel and service-face events, generates
JSON Schema and TypeScript, and raises chains and observations at existing doors. M136 only; the embedded runtime is M137. No installed link means unchanged behaviour.

## Bricks

1. Contract and failing tests first: `catalogue!` defines entries and `ALL` once.
   Entries carry kind, schemas, matcher fields, pins and acts; `LinkPoint` is a static reference. A discipline check rejects stray literals.
2. Claude spelling: flattened `ToolCall` carries pinned `tool`/`tool_use_id`; output
   carries `isError`; reject colliding schemas at gather and arguments before flattening.
3. Pure `Pattern`, conservative scalar `prefilter`, `pinned_changed`, and tier checks;
   Rust and TypeScript share `test/cases.json`.
4. JSON `Interceptor::chain`/`observe`; seven typed defaults bridge through it.
   The existing chain skips invalid shapes, pinned rewrites and excess authority once.
5. New chains: tool/command descriptions, autocomplete, send/receive, append,
   completion, command execution and settings. `tool.describe` keeps ToolSpec's `name`.
   Only owners rewrite their extension rows. Completion text follows `TurnCompleted`.
   Model/thinking and live plugin switches call `config::set` before any reconfigure/write;
   the standalone settings CLI has no live session or chain.
6. Owned, ordered observations: session start/end, client port attach/drop, turn start
   and `TurnUsage` measurement (never additive `UsageCharged`). A port owns attachment;
   `HostApi::close` has no client identity. Startup precedes the first attachment.
7. Service faces declare noun/method schemas and unsafe metadata. Registration rejects
   names claimed by the kernel or another owner. Wire lookups raise; typed lookups do not.
   Dynamic faces own schemas/names; static entries keep Rust schema functions without leaks.
8. Protocol-2 subscriptions and `intercept/*` cover dynamic names; `hook/observe` carries observations. `bingo-schema-ts::render` consumes generated events.json.

## Files

- SDK: `catalogue.rs`, `catalogue/*`, `intercept.rs`, `intercept/json.rs`, `service.rs`, `service/face.rs`, `lib.rs`.
- Core: `chain/*`, `host.rs`, `host/interception.rs`, `session.rs`, `session/{catalogue,mailbox,gate,inputs,commands}.rs`, `config.rs`, `commands/{model,think,plugins}.rs`, `plugins.rs`, `turn.rs`, `turn/late.rs`, `lib.rs`.
- Services: `team.rs`, `mcp.rs`, Cargo.toml/lock. Existing workspace `schemars` derives adapter face schemas; no new external crate.
- Plugin RPC: `intercept`, `wire`, `bridge`, `host`, `hook`, `service`, `schema`, `events_ts`, `validate` (including its unit tests), `lib`; stub plugin fixtures.
- Tests: SDK intercept fixtures; core chain/host/session fixtures and new catalogue tests; RPC hook/reload fixtures; `crates/bingo/tests/catalogue.rs`.
- Generated: `schema/{events,plugin,plugin-2}.json`, `sdk/typescript/src/{events,wire}.ts`.
- TypeScript: `on`, `on.test`, `plugin`, `rpc`, `rpc.test`, `index`; shared `test/cases.json`; `scripts/check_discipline.sh`; named ADR notes.

## Exit criteria

- [x] SDK catalogue tests pass; deleting an events.json entry fails (shown once); final workspace rerun passes the guard.
- [x] Composed binary catalogue passes; reserved `tool_use_id` fixture fails (shown once).
- [x] Rewrite descriptions/received text; skip pins/tighten answers once; compose completion text beneath the answer.
- [x] Observe start, attach, turn.start, measure, detach, end in actual lifecycle order.
- [x] Real MCP wire face raises `mcp.servers`; duplicate ownership names both owners; final workspace rerun passes both.
- [x] Generated TypeScript and stray Entry drift fail once; shared matcher/SDK tests pass.
- [x] Existing tests and native gates pass; MSVC checks cover touched crates, with step-7 C-header exceptions recorded below.

## Non-goals

Embedded runtime/registration, op/UI events, mention/attachment resolution and typed-service interception. No live/credential tests, publishing or foreign processes.

## Risks

The user's 2026-10-13 decision resolves the initial protocol stop: v0.6.6 ships protocol 1,
so unreleased protocol 2 is amended in place. Rename the old hook `turn.stop` unchanged;
`turn.complete` becomes metadata → completion text. Amend only these frozen entries and
referenced definitions in one commit, regenerate current schemas with BINGO_UPDATE_SCHEMA=1,
and retain the additive-only test. ADR-0075 records the dated pre-release exception.
Keep shared edits local for M134/M125; gather links once per round, own actor work, and keep held deliveries held.

**Stopped 2026-10-14, protocol step 6.** The workspace gate reaches
`plugin_kit::typescript_acceptance_runs_outside_the_workspace` and fails because
`plugin validate` prints `InitializeResult.links` but not the new `subscriptions`.
The proper fix is `bingo-plugin-rpc/src/validate.rs::findings`, outside Files.
Recommended next scope: report each subscription pattern (including `tool.check`
and `mcp.*`), add a findings test there, and preserve the existing acceptance assertion.
Alternative: defer the process subscription migration to a separately scoped change;
that leaves M136's dynamic wire criterion incomplete. No acceptance assertion was changed.

## Verified — re-verification only; implementation blocked

Read the protocol, audit, parity design and ADRs 0078, 0074, 0079, 0075,
0069 and 0068. Initial tree clean, requested branch at `60deb2b9`.
Corrected call sites, M119 adapter paths, M133's `bingo-schema-ts::render`
(`wire_ts.rs` is the drift-test example) and matcher cases (`on.test.ts`).
Read-only Python inspection resolved method references in both schemas:

```text
FROZEN intercept/turn.complete: input=null; answer=#/$defs/StopResult
CURRENT intercept/turn.complete: input=null; answer=#/$defs/StopResult
FROZEN intercept/turn.complete/after: result=#/$defs/StopResult
CURRENT intercept/turn.complete/after: result=#/$defs/StopResult
```

Implementation and gates not run: steps 2/6 require stopping on this conflict.
No schemas regenerated; no live or credential tests run.

Resumed on the user's pre-release amendment decision; the earlier stop is resolved.

## Verified (2026-10-14) — partial; stopped by protocol

Branch `m136-the-event-catalogue`, base `60deb2b9`; implementation through `f819be88`.
The frozen amendment is solely in `d9aa5cb8`; its additive-only check still passes.
Initial contracts failed before implementation. Trimmed output below is pasted from logs.
```text
cargo test -p bingo-sdk catalogue::                    # initial contract
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 127 filtered out; finished in 0.01s
cargo test -p bingo-sdk face_names_cannot_be_claimed_twice
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 131 filtered out; finished in 0.02s
cargo test -p bingo-plugin-rpc initialization_retains_open_catalogue_subscriptions
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 143 filtered out; finished in 0.03s
cargo test -p bingo-sdk catalogue::                    # preliminary positive
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 127 filtered out; finished in 0.02s
cargo test -p bingo-core                              # before final gate/queue fixes
test result: ok. 481 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.96s
cargo test -p bingo-core host::tests::catalogue --locked
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 480 filtered out; finished in 0.01s
cargo test -p bingo --test catalogue --locked
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s
cargo test -p bingo-plugin-rpc --locked
test result: ok. 144 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.34s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
cargo test -p bingo-plugin-rpc protocol_two_keeps_every_frozen_shape --locked
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 143 filtered out; finished in 0.01s
npm test --prefix sdk/typescript
ℹ pass 11
ℹ fail 0
cargo test -p bingo-sdk catalogue:: --locked            # deliberately deleted entry
test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 128 filtered out; finished in 0.04s
cargo test -p bingo-plugin-rpc events_ts:: --locked     # deliberate hand edit
assertion `left == right` failed: events.ts drift
BINGO_TEST_RESERVED_TOOL=1 cargo test -p bingo --test catalogue built_in_tool_inputs_leave_the_envelope_fields_free --locked
assertion `left == right` failed: fixture collides with the tool.call envelope
scripts/check_discipline.sh                           # deliberate stray Entry
crates/bingo-sdk/src/catalogue_negative.rs:1: declare catalogue Entry literals in bingo-sdk/src/catalogue.rs
discipline FAILED
cargo fmt --all -- --check                            # final code: exit 0, no output
cargo check --workspace --all-targets --locked
Finished `dev` profile [unoptimized + debuginfo] target(s) in 22.90s
cargo clippy --workspace --all-targets --locked -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 27.08s
cargo test --workspace --locked -- --skip acp::bridge::a_call_with_no_turn_in_flight_is_refused_with_a_reason --test-threads=1
test result: ok. 252 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 108.21s
test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
assertion failed: validated.contains("protocol 2") && validated.contains("tool.check")
cargo test -p bingo --test cli acp::bridge::a_call_with_no_turn_in_flight_is_refused_with_a_reason --locked -- --exact --nocapture
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 252 filtered out; finished in 0.38s
scripts/check_discipline.sh                           # preliminary, before final fixes
discipline ok
scripts/budget.sh                                     # dependency-change check
dependencies (unique, normal): 355 (max  355)
budget ok
```
All intentional corruptions were restored; current schemas/types were generated with
BINGO_UPDATE_SCHEMA=1. Final workspace completion, SDK guard/collision reruns, discipline,
budget, deny, MSVC and private-tmux smoke remain pending. The prepared smoke was not run.
ADR-0078 remains proposed pending completion. Only local fixtures were used; no live services/user credentials, push, merge or PR.


## Scope addition (2026-10-14)

The user approved `crates/bingo-plugin-rpc/src/validate.rs` and its unit test.
Validation will report subscriptions, derive named matches from `catalogue::ALL`,
and retain dynamic noun patterns without a second event list. Keep the existing
TypeScript acceptance assertion. The previous step-6 stop is resolved.
Do not rebase onto moved dev; the user owns merging and conflict resolution.
Per the user's explicit request, append full gate output in Verified; this takes
precedence over the plan's usual 150-line limit. No push, merge or PR.

Resumed workspace testing exposed extension writes overtaking command acknowledgements.
Keep mutations and observations on separate owned queues; acknowledge a command after
its queued writes. This fix stays in the existing core Files, preserves the board
assertions, and mechanically adapts the storage-test queue fixture to the new type.


## Verified (2026-10-14) — M136 complete

Branch `m136-the-event-catalogue`, original base `60deb2b9`, implementation through
`e8c42ddd`; ADR acceptance recorded in `ab454a31`. No rebase onto moved dev.
The approved validation scope is implemented: exact, wildcard and dynamic
subscriptions are reported from `catalogue::ALL`, with no duplicate event list.
Its new contract was seen failing before the fix. The unchanged TypeScript
acceptance and board tests pass. Command acknowledgements now follow queued
extension writes; an observer cannot hold up the acknowledgement.

All native gates pass, run one at a time. Final workspace output records 5,190
passed, zero failed and four existing ignored manual/stress tests. The known
ACP bridge test is explicitly excluded from that run and passes alone (one test).
The 11 TypeScript tests pass. The full workspace run includes catalogue/schema
fixtures, the additive-only protocol-2 freeze test, generated TypeScript drift,
service ownership collisions, the MCP face, CLI black-box tests and all PTY tests.
No PTY rerun was needed. The private tmux smoke passes; its copy differs from
`scripts/tui-smoke.sh` only in the worktree ROOT and unique SOCKET
`bingo-m136-3df8db17331b41f3a650c98b5c5e0d26`, and cleans up its own server.

Windows checks pass for `bingo-sdk`, `bingo-core` and `bingo-services`.
Per execution-protocol step 7, Windows CI must finish `bingo-plugin-rpc`
(`aws-lc-sys`: missing `stdlib.h`/`windows.h`) and `bingo`
(`tree-sitter`, `tree-sitter-bash`: missing `stdio.h`/`stdlib.h`;
`aws-lc-sys`: missing `stdlib.h`/`windows.h`). No Windows Rust error was reported
before those dependency build failures; the blocked crates are not claimed as
cross-compiled. No toolchain was installed. Budget and dependency policy pass;
the budget's 7 GB debug-directory warning is soft, and dependencies remain 355.
The plan-length warning is expected under the user's full-output requirement.

ADR-0078 is accepted only for M136's catalogue, noun faces and face metadata;
later runtime, op/UI and grant implementation remains proposed. The other named
ADR notes record the implemented amendments. The frozen protocol-2 change remains
in one commit, `d9aa5cb8`; no later change edits the frozen schema.
All exit criteria are ticked, subject only to the documented Windows CI checks.
Only local fixtures were used: no live or credential tests, terminal application,
foreign process, push, merge or PR. Stop at M136 per protocol.

Complete output from this resumed verification follows, including the initial
contract failure, the diagnosed board regression and the successful reruns.
Historical partial verification above is retained as the record of the prior stop.
Only trailing spaces are normalized for Markdown; no output lines are omitted.

<details>
<summary>validation-contract-before-fix — exit 101</summary>

```text
$ cargo test -p bingo-plugin-rpc validate::tests::subscriptions_accept_every_catalogue_entry_and_dynamic_patterns --locked
      ┌─────────┐  mr boxington
     ╱ ═══════ ╱│
    ╱─────────╱ │
    │ •    (◉)│≡│
    │  ◟▄▄▄◞  │ ╱
    │   ◀◆▶   │╱
    └─────────┘
   Compiling bingo-sdk v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-sdk)
   Compiling bingo-http v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-http)
   Compiling bingo-pictures v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-pictures)
   Compiling bingo-plugin-rpc v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-plugin-rpc)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4.89s
     Running unittests src/lib.rs (target/debug/deps/bingo_plugin_rpc-595e436fe66eb588)

running 1 test
test validate::tests::subscriptions_accept_every_catalogue_entry_and_dynamic_patterns ... FAILED

failures:

---- validate::tests::subscriptions_accept_every_catalogue_entry_and_dynamic_patterns stdout ----

thread 'validate::tests::subscriptions_accept_every_catalogue_entry_and_dynamic_patterns' (6392916) panicked at crates/bingo-plugin-rpc/src/validate.rs:238:13:
missing catalogue subscription prompt.submit: ["catalogue: protocol 2"]
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/core/src/panicking.rs:80:14
   2: bingo_plugin_rpc::validate::tests::subscriptions_accept_every_catalogue_entry_and_dynamic_patterns
   3: bingo_plugin_rpc::validate::tests::subscriptions_accept_every_catalogue_entry_and_dynamic_patterns::{{closure}}
   4: core::ops::function::FnOnce::call_once
   5: <fn() -> core::result::Result<(), alloc::string::String> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.


failures:
    validate::tests::subscriptions_accept_every_catalogue_entry_and_dynamic_patterns

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 144 filtered out; finished in 0.05s

error: test failed, to rerun pass `-p bingo-plugin-rpc --lib`
mbx[cache]: 0 hits, 0 misses, 6 incremental; 0 B downloaded, 0 B uploaded, 0 B stored locally
mbx[savings]: found 1023.3 GiB logical of target/ whose checkouts left long ago. the estate has been settled.
```

</details>

<details>
<summary>validation — exit 0</summary>

```text
$ cargo test -p bingo-plugin-rpc validate:: --locked
      ┌─────────┐  mr boxington
     ╱ ═══════ ╱│
    ╱─────────╱ │
    │ •    (◉)│≡│
    │  ◟▄▄▄◞  │ ╱
    │   ◀◆▶   │╱
    └─────────┘
   Compiling bingo-plugin-rpc v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-plugin-rpc)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.93s
     Running unittests src/lib.rs (target/debug/deps/bingo_plugin_rpc-595e436fe66eb588)

running 3 tests
test validate::tests::protocol_one_names_both_numbers ... ok
test validate::tests::subscriptions_accept_every_catalogue_entry_and_dynamic_patterns ... ok
test validate::tests::missing_entry_and_bad_name_name_the_field ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 142 filtered out; finished in 0.00s

     Running tests/plugin/main.rs (target/debug/deps/plugin-30b41f3f0feaf9b5)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 49 filtered out; finished in 0.00s

mbx[cache]: 0 hits, 0 misses, 3 incremental; 0 B downloaded, 0 B uploaded, 0 B stored locally
mbx[savings]: 1023.3 GiB logical of target/ was waiting for checkouts that are never coming back. it has stopped waiting.
```

</details>

<details>
<summary>plugin-kit — exit 0</summary>

```text
$ cargo test -p bingo --test plugin_kit --locked
      ┌─────────┐  mr boxington
     ╱ ═══════ ╱│
    ╱─────────╱ │
    │ •    (◉)│≡│
    │  ◟▄▄▄◞  │ ╱
    │   ◀◆▶   │╱
    └─────────┘
   Compiling bingo-plugin-rpc v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-plugin-rpc)
   Compiling bingo v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 3.29s
     Running tests/plugin_kit.rs (target/debug/deps/plugin_kit-3b018348024e3630)

running 5 tests
test missing_entry_is_a_clean_validation_failure ... ok
test scenario_refuses_a_plugin_that_did_not_start ... ok
test wordcount_validates_with_the_existing_protocol ... ok
test wrong_expectation_names_the_step_and_exits_one ... ok
test typescript_acceptance_runs_outside_the_workspace ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s

mbx[cache]: 0 hits, 0 misses, 1 not looked up, 2 incremental; 0 B downloaded, 0 B uploaded, 1.6 MiB stored locally
mbx[savings]: across 3908 builds, rustc has been excused from 2d 16h of its duties.
```

</details>

<details>
<summary>fmt — exit 0</summary>

```text
$ cargo fmt --all -- --check
```

</details>

<details>
<summary>check — exit 0</summary>

```text
$ cargo check --workspace --all-targets --locked
      ┌─────────┐  mr boxington
     ╱ ═══════ ╱│
    ╱─────────╱ │
    │ •    (◉)│≡│
    │  ◟▄▄▄◞  │ ╱
    │   ◀◆▶   │╱
    └─────────┘
    Checking bingo-plugin-rpc v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-plugin-rpc)
    Checking bingo v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.88s
mbx[cache]: 0 hits, 0 misses, 23 incremental; 0 B downloaded, 0 B uploaded, 0 B stored locally
mbx[gc]: removed 4 target directories (26.9 GiB logical, 4 abandoned and 0 live); 29.2 GiB logical remain
mbx[gc]: evicted 5201 objects and 3027 action results (8.4 GiB logical); 45.0 GiB logical remain
mbx[savings]: 252296 compilations dispensed from stock. inventory immaculate.
```

</details>

<details>
<summary>clippy — exit 0</summary>

```text
$ cargo clippy --workspace --all-targets --locked -- -D warnings
      ┌─────────┐  mr boxington
     ╱ ═══════ ╱│
    ╱─────────╱ │
    │ •    (◉)│≡│
    │  ◟▄▄▄◞  │ ╱
    │   ◀◆▶   │╱
    └─────────┘
    Checking bingo-plugin-rpc v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-plugin-rpc)
    Checking bingo v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.00s
mbx[cache]: 0 hits, 0 misses, 23 incremental; 0 B downloaded, 0 B uploaded, 0 B stored locally
mbx[savings]: 1.3 TiB logical of build debris has left the building.
```

</details>

<details>
<summary>workspace-tests — exit 101</summary>

```text
$ cargo test --workspace --locked -- --skip acp::bridge::a_call_with_no_turn_in_flight_is_refused_with_a_reason --test-threads=1
      ┌─────────┐  mr boxington
     ╱ ═══════ ╱│
    ╱─────────╱ │
    │ •    (◉)│≡│
    │  ◟▄▄▄◞  │ ╱
    │   ◀◆▶   │╱
    └─────────┘
   Compiling bingo-plugin-rpc v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-plugin-rpc)
   Compiling bingo v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 6.75s
     Running unittests src/main.rs (target/debug/deps/bingo-735e7126a68a01d2)

running 22 tests
test acp_proxy::tests::a_proxy_nobody_spawned_says_which_word_it_is_missing ... ok
test channels::tests::only_an_adapter_that_signs_is_accepted_and_the_refusal_lists_the_ones_that_do ... ok
test channels::tests::the_receipt_names_the_file_and_the_key_and_no_secret ... ok
test env::tests::no_environment_variable_is_named_in_the_binary ... ok
test mcp::tests::a_name_nobody_configured_is_refused_with_the_names_that_are ... ok
test mcp::tests::a_pair_that_is_not_a_pair_is_refused_by_the_text_that_was_typed ... ok
test mcp::tests::a_row_the_plugin_would_refuse_is_refused_here ... ok
test mcp::tests::a_server_with_nothing_configured_says_none_rather_than_nothing ... ok
test mcp::tests::a_stdio_entry_keeps_the_arguments_after_the_command ... ok
test mcp::tests::an_http_entry_is_the_row_the_plugin_reads ... ok
test mcp::tests::get_prints_the_names_of_the_secrets_and_not_their_values ... ok
test mcp::tests::the_listing_is_one_line_per_server_and_says_so_when_there_are_none ... ok
test plugins::schema::tests::committed_scenario_schema_is_generated ... ok
test plugins::schema::tests::scenario_contract_names_every_step ... ok
test plugins::switch::tests::a_listing_is_one_tab_separated_line_per_plugin ... ok
test plugins::switch::tests::a_name_no_listing_knows_is_refused ... ok
test provider::tests::a_key_that_is_not_an_object_is_refused_rather_than_overwritten ... ok
test provider::tests::a_missing_file_is_an_empty_document_and_a_broken_one_is_an_error ... ok
test provider::tests::a_protocol_is_one_of_two_words_however_it_is_typed ... ok
test provider::tests::a_receipt_names_both_files_and_the_command_that_uses_them ... ok
test provider::tests::an_instance_joins_the_document_without_moving_its_neighbours ... ok
test tests::the_highest_layer_that_names_the_measure_is_the_one_read ... ok

test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/acp_asked.rs (target/debug/deps/acp_asked-b4d3a9810271a3a7)

running 1 test
test a_person_answers_the_agents_question_and_the_agent_gets_their_choice ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.44s

     Running tests/acp_bridge.rs (target/debug/deps/acp_bridge-c166c0da5ddb0423)

running 3 tests
test a_proxy_ends_when_its_bridge_does_though_its_stdin_is_open ... ok
test a_proxy_with_a_token_this_run_never_minted_gets_nothing ... ok
test an_agent_reaches_the_shared_tools_through_the_spawned_proxy ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/catalogue.rs (target/debug/deps/catalogue-3d7684a9bbcd808b)

running 2 tests
test built_in_tool_inputs_leave_the_envelope_fields_free ... ok
test the_real_mcp_wire_face_raises_its_declared_event ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

     Running tests/channel_claim.rs (target/debug/deps/channel_claim-ba0e4b68f67773be)

running 1 test
test a_live_channel_host_excludes_another_and_a_killed_one_releases ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

     Running tests/channels.rs (target/debug/deps/channels-24ad4b705114f095)

running 16 tests
test a_blocklisted_principal_in_a_group_is_answered_with_nothing ... ok
test a_chats_own_rule_engages_without_the_mention_the_others_need ... ok
test a_group_is_ignored_until_the_bot_is_mentioned ... ok
test a_legacy_channel_pid_file_refuses_loudly ... ok
test a_message_opens_a_session_and_the_answer_streams_into_one_edited_message ... ok
test a_new_session_keeps_the_thread_and_answers_the_next_message ... ok
test a_path_that_is_not_there_is_refused_in_words_the_model_reads ... ok
test a_permission_is_buttons_and_a_click_answers_it ... ok
test a_prompt_from_a_chat_carries_its_principal_and_its_conversation ... ok
test a_question_answered_at_another_surface_edits_the_card ... ok
test a_queued_command_returns_its_result_to_the_chat ... ok
test loopback_without_the_switch_is_refused_before_listening ... ok
test only_an_admin_is_heard_in_a_group_that_admits_only_admins ... ok
test the_model_posts_a_file_into_the_chat_it_is_talking_in ... ok
test the_working_sign_brackets_the_turn_a_message_started ... ok
test without_buttons_the_numbered_rung_is_drawn_and_a_reply_answers_it ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.22s

     Running tests/cli/main.rs (target/debug/deps/cli-b6afe1881793096f)

running 252 tests
test a_failed_turn_is_one_error_line_and_exit_1 ... ok
test a_form_crosses_the_json_wire_as_one_frame ... ok
test a_missing_prompt_and_an_unknown_provider_are_errors_before_any_turn ... ok
test a_pipe_without_print_still_runs_headlessly ... ok
test a_print_run_never_plays_the_opening ... ok
test a_question_is_declined_when_nobody_is_at_the_keyboard ... ok
test a_slow_bash_command_streams_its_tail_as_deltas ... ok
test a_tool_round_reads_the_file_and_reports_on_stderr ... ok
test acp::a_turn_through_an_adapter_streams_text_thought_and_the_agents_own_call ... ok
test acp::an_interrupt_cancels_the_turn_and_the_child_serves_the_next_one ... ok
test acp::asking::a_bypass_session_answers_the_agents_own_allow_option ... ok
test acp::asking::a_dont_ask_session_answers_the_agents_own_reject_option ... ok
test acp::asking::a_headless_run_puts_the_question_and_falls_closed_when_nobody_answers ... ok
test acp::asking::an_elicitation_is_still_declined_and_said_once ... ok
test acp::bridge::a_bridged_call_posts_to_the_parent_and_is_journaled_under_the_turn ... ok
test acp::bridge::a_persons_own_servers_are_forwarded_verbatim_and_leave_the_offer ... ok
test acp::bridge::a_row_that_keeps_its_servers_home_serves_their_tools_on_the_bridge ... ok
test acp::bridge::a_row_that_names_its_tools_is_offered_only_those ... mbx[progress]: 15s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test acp::bridge::a_row_this_agent_cannot_take_is_skipped_and_named ... ok
test acp::bridge::a_tool_a_plugin_registered_reaches_the_bridge_with_no_edit_here ... ok
test acp::bridge::acp_reads_only_enabled_rows_from_the_real_mcp_owner ... ok
test acp::bridge::an_interrupt_drops_a_bridged_call_and_the_child_serves_the_next_turn ... ok
test acp::bridge::the_bridge_offers_the_sessions_tools_and_not_the_agents_own_hands ... ok
test acp::catalogue::a_cold_refresh_lists_the_agents_own_models ... ok
test acp::catalogue::a_live_sessions_declaration_supersedes_the_cold_one ... ok
test acp::catalogue::a_probe_that_cannot_start_the_adapter_serves_the_label_and_says_so ... ok
test acp::compacting_an_acp_session_is_refused_in_the_adapters_name ... ok
test acp::knobs::a_model_change_reaches_the_agent_as_its_own_value ... ok
test acp::knobs::a_respawned_child_is_set_from_the_row_again ... ok
test acp::knobs::a_row_effort_and_a_later_change_are_two_messages ... ok
test acp::knobs::a_row_option_reaches_the_agent_once_before_its_first_prompt ... ok
test acp::knobs::a_spawn_that_names_the_agents_model_opens_the_session_already_set ... ok
test acp::knobs::a_thinking_change_reaches_the_agent_before_the_next_prompt ... ok
test acp::knobs::an_adapter_with_only_the_legacy_door_is_set_through_it ... ok
test acp::knobs::an_agent_with_neither_knob_gets_no_config_call_and_one_notice ... ok
test acp::knobs::an_option_the_agent_never_declared_is_one_notice_and_no_call ... ok
test acp::knobs::the_agent_label_applies_nothing ... ok
test acp::knobs::the_model_list_is_the_agents_own_once_a_session_has_opened ... ok
test acp::life::a_child_that_dies_at_the_prompt_is_replaced_and_asked_again ... ok
test acp::life::a_child_that_dies_mid_stream_fails_the_turn_it_was_answering ... ok
test acp::life::an_adapter_that_died_between_turns_is_replaced_and_said ... ok
test acp::the_agents_own_count_its_window_and_its_own_compaction_reach_every_surface ... ok
test acp::the_agents_session_id_is_journaled_once_as_an_extension ... ok
test acp::the_restore_ladder_climbs_resume_then_load_then_a_file ... ok
test acp::two_turns_of_one_session_ride_one_child_and_one_agent_session ... ok
test again::a_bounced_post_lands_on_the_word_after_it ... ok
test agents::a_child_has_no_spawn_agent_to_call ... ok
test agents::a_child_that_failed_is_an_error_result_for_the_root ... ok
test agents::a_childs_permission_is_refused_off_a_tty_in_every_output_format ... ok
test agents::a_dismissed_agent_s_session_is_deleted_from_the_data_dir ... ok
test agents::a_finished_background_agent_wakes_the_run_that_spawned_it ... ok
test agents::a_foreground_agent_answers_the_root_and_stdout_stays_the_root_s ... ok
test agents::a_project_definition_names_the_agent_it_starts ... ok
test agents::a_project_s_roles_are_seated_before_the_root_s_first_turn ... ok
test agents::a_resumed_root_is_told_which_child_was_mid_turn_when_the_process_ended ... ok
test agents::a_wake_that_finds_the_parent_busy_is_never_lost ... mbx[progress]: 30s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test agents::an_unwoken_member_has_no_turn_and_its_brief_is_not_journalled ... ok
test agents::dismissing_a_child_keeps_print_stdout_valid_ndjson ... ok
test agents::dismissing_is_gated_and_a_refused_dismissal_deletes_nothing ... ok
test agents::stopping_an_idle_agent_is_an_error_result_that_says_so ... ok
test agents::the_child_s_reply_is_the_tool_call_s_result ... ok
test agents::the_mint_names_the_root_and_leaves_a_seat_its_own_name ... ok
test agents::the_same_roles_come_back_on_continue ... ok
test an_edit_is_asked_and_denied_off_a_tty_under_the_default_policy ... ok
test anthropic_without_credentials_fails_before_any_turn ... ok
test asking::a_permission_a_rule_could_silence_offers_the_session_rung ... ok
test asking::a_permission_no_rule_could_silence_offers_no_session_rung ... ok
test bash_runs_only_when_the_flags_allow_it ... ok
test batch::two_commands_in_one_step_both_come_back ... ok
test board::a_member_reads_a_room_s_board_claims_a_task_and_the_parent_sees_it ... ok
test board::a_session_s_own_list_is_untouched_by_the_board ... ok
test board::a_task_left_by_a_member_that_is_gone_reads_as_gone ... ok
test checkpoints::a_file_a_turn_created_is_removed_and_the_snapshots_outlive_nothing ... ok
test checkpoints::rewinding_two_turns_puts_the_file_back_to_before_the_first_of_them ... ok
test compaction_prefix::manual_summary_uses_the_old_baseline_and_next_request_refreshes_it ... ok
test compaction_prefix::rejected_summary_neither_executes_tools_nor_refreshes_the_baseline ... ok
test context::a_memory_the_model_writes_by_hand_is_what_memory_lists ... ok
test context::an_overflow_after_many_rounds_is_summarised_and_the_turn_goes_on ... ok
test context::an_overflow_is_retried_once_and_the_window_is_learned_on_disk ... ok
test context::the_context_warning_is_said_once_near_the_line ... ok
test experience::a_commit_is_proposed_as_the_file_it_would_write ... ok
test experience::an_outcome_without_evidence_is_an_input_error ... ok
test experience::commit_query_outcome_and_revise_round_trip_on_disk ... ok
test experience::the_command_folds_the_library_into_a_table ... ok
test gateway::a_gateway_stopped_the_instant_it_is_up_still_gives_its_pidfile_back ... ok
test gateway::a_pasted_channel_secret_lands_in_auth_json_and_doctor_names_its_source ... ok
test gateway::a_schedule_fires_with_no_terminal_attached ... ok
test gateway::a_second_start_refuses_and_names_the_pid_that_already_has_it ... ok
test gateway::a_stale_pidfile_is_reported_dead_and_doctor_fix_clears_it ... ok
test gateway::a_start_with_no_channel_refuses_before_anything_moves ... ok
test gateway::a_supervisor_that_refuses_leaves_no_service_file_behind ... ok
test gateway::channels_add_asks_for_both_and_writes_each_where_it_belongs ... ok
test gateway::doctor_names_a_missing_credential_and_never_prints_one ... ok
test gateway::install_writes_a_secret_free_service_and_the_verbs_then_delegate ... ok
test gateway::start_status_stop_round_trips_on_a_real_detached_process ... ok
test gateway::stopping_gives_back_the_schedule_runner_claim_and_the_pidfile ... ok
test gateway::tracing_lines_land_in_the_gateway_log_at_info_and_at_warn ... ok
test hooks::a_pre_tool_use_hook_that_denies_stops_the_write_and_tells_the_model ... ok
test images::a_picture_over_the_bound_reaches_the_journal_bounded ... mbx[progress]: 45s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test images::a_stream_json_user_line_carries_its_image_blocks ... ok
test images::a_url_that_is_not_a_picture_is_exit_1_and_says_so ... ok
test images::an_image_flag_puts_the_picture_beside_the_prompt_in_the_journal ... ok
test images::an_image_flag_under_stream_json_input_is_refused ... ok
test images::an_image_of_a_wider_format_reaches_the_journal_as_png ... ok
test images::an_image_that_does_not_read_is_exit_1_with_nothing_on_stdout ... ok
test images::an_image_url_is_fetched_and_journaled ... ok
test images::an_image_url_over_the_cap_is_exit_1_with_nothing_on_stdout ... ok
test images::an_image_url_that_is_not_there_is_exit_1_with_nothing_on_stdout ... ok
test instances::a_colliding_instance_name_stops_the_boot_and_is_named ... ok
test instances::a_pasted_key_reaches_the_wire_and_a_logout_takes_it_away ... ok
test instances::an_exported_key_does_not_feed_a_named_instance ... ok
test instances::an_instance_is_reachable_by_provider_and_by_model ... ok
test instances::two_codex_instances_hold_two_credentials_and_refresh_apart ... ok
test jobs::a_bare_foreground_sleep_is_refused_and_starts_nothing ... ok
test jobs::a_default_watch_still_says_it_once_and_counts_nothing ... ok
test jobs::a_finished_job_opens_a_turn_on_a_headless_run ... ok
test jobs::a_job_is_pulled_by_cursor_across_two_turns_and_then_killed ... ok
test jobs::a_one_shot_run_ends_at_its_turn_and_the_job_ends_with_it ... ok
test jobs::a_pinned_watch_wakes_once_and_counts_the_rest_onto_the_end ... ok
test jobs::a_running_command_is_promoted_mid_turn_and_the_call_returns_early ... ok
test jobs::a_tail_f_backgrounds_itself_and_a_plain_command_does_not ... ok
test jobs::an_id_no_job_has_is_an_error_result_the_model_can_correct ... ok
test jobs::an_ongoing_watch_on_a_slow_stream_wakes_on_every_line ... ok
test jobs::notify_all_with_no_condition_is_refused_and_starts_no_job ... ok
test json_output_is_one_frame_per_line_ending_in_turn_completed ... ok
test login::a_401_is_followed_by_one_refresh_and_the_turn_completes ... ok
test login::a_device_sign_in_stores_the_token_and_the_next_turn_uses_it ... ok
test login::a_provider_without_a_sign_in_says_so ... ok
test login::an_unknown_provider_is_refused_by_name ... ok
test login::before_any_sign_in_a_codex_turn_is_refused_by_name ... ok
test login::device_and_paste_exclude_each_other ... ok
test mcp::a_child_process_cannot_be_signed_in_to ... ok
test mcp::a_name_nobody_configured_is_one_error_line_and_exit_1 ... ok
test mcp::a_pasted_code_signs_in_and_the_token_lands_in_auth_json ... ok
test mcp::a_row_the_plugin_would_refuse_is_refused_before_it_is_written ... ok
test mcp::a_run_configured_by_flag_lists_what_that_run_would_dial ... ok
test mcp::a_server_is_added_read_back_listed_and_removed ... ok
test mcp::a_stdio_server_joins_the_ones_that_are_already_there ... ok
test mcp::get_never_prints_a_header_or_an_environment_value ... ok
test mcp::signing_out_of_a_server_nobody_signed_in_to_is_a_no_op ... ok
test mcp::the_print_surface_refuses_a_sign_in_and_names_the_way_through ... ok
test mcp_config_must_exist_and_carry_servers ... ok
test mentions::a_question_left_unanswered_is_chased_when_the_next_process_opens_the_room ... ok
test mentions::the_card_stands_while_a_question_does_and_goes_when_it_is_answered ... ok
test mentions::the_room_table_names_who_owes_an_answer ... ok
test models::a_model_flag_is_not_remembered ... ok
test models::a_spawn_that_names_a_provider_and_a_model_still_lands ... ok
test models::list_models_names_every_provider_its_models_and_its_sign_in_state ... ok
test models::models_refuses_an_argument_it_does_not_know ... ok
test models::the_models_command_lists_the_catalogue_and_refreshes_on_demand ... ok
test models::thinking_ignores_false_capability_metadata_without_rewriting_it ... ok
test models::thinking_off_and_invalid_levels_preserve_model_overrides ... mbx[progress]: 60s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test models::thinking_on_an_unknown_model_is_effective_and_remembered ... ok
test openai_runs_a_tool_round_and_feeds_the_result_back ... ok
test openai_streams_a_text_turn_through_the_same_loop ... ok
test openai_without_credentials_names_the_variable_before_any_turn ... ok
test peers::a_bounced_post_neither_opens_nor_answers_a_mention_debt ... ok
test peers::a_post_reaches_each_member_exactly_once ... ok
test peers::a_sibling_dm_reaches_the_teammate_and_leaves_the_parent_out_of_it ... ok
test peers::a_stale_post_bounces_with_what_it_missed_and_lands_on_the_retry ... ok
test peers::a_standby_member_nothing_wakes_runs_no_turn_at_all ... ok
test peers::an_answer_from_the_parent_wakes_the_child_that_asked ... ok
test peers::one_kickoff_post_runs_a_relay_the_parent_never_dispatches ... mbx[progress]: 75s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test peers::waiting_for_a_standby_agent_is_refused_before_one_is_started ... ok
test persona::a_projects_own_text_replaces_the_stance_entirely ... ok
test persona::a_typo_under_the_key_stops_the_run_and_names_the_field ... ok
test persona::switched_off_the_stance_is_not_in_the_prompt ... ok
test persona::the_page_this_plugin_wrote_is_listed_in_the_prompt ... ok
test persona::the_stance_reaches_the_model_in_the_system_prompt ... ok
test persona::the_stance_sits_between_the_kernels_blocks_and_the_projects_own ... ok
test plan_mode_denies_a_write_and_the_turn_goes_on ... ok
test plugins::a_missing_used_service_is_named_without_disabling_its_consumer ... ok
test plugins::a_needed_plugin_and_a_name_nobody_lists_are_refused_with_the_reason ... ok
test plugins::a_tool_plugin_switched_off_is_written_listed_and_gone_from_the_model_s_tools ... ok
test plugins::an_external_plugin_is_listed_switched_and_never_spawned ... ok
test plugins::an_external_plugins_tier_is_listed_from_user_settings_only ... ok
test prefix::context_snapshots_cannot_name_a_session_started_by_a_skill ... ok
test prefix::knowledge_stays_fixed_between_rounds_and_refreshes_when_resumed ... ok
test print_streams_prose_to_stdout_and_nothing_else ... ok
test provider_add::a_comment_in_the_settings_survives_the_command_that_writes_beside_it ... ok
test provider_add::a_name_already_taken_is_refused_and_nothing_is_written ... ok
test provider_add::a_protocol_that_is_neither_is_refused_by_name ... ok
test provider_add::a_settings_json_left_from_an_older_bingo_crosses_on_the_first_run ... ok
test provider_add::an_added_provider_is_written_down_and_the_next_run_uses_it ... ok
test provider_add::an_anthropic_instance_needs_neither_an_endpoint_nor_a_key ... ok
test rooms::a_member_listens_and_the_room_s_journal_says_what_it_now_hears ... ok
test rooms::a_patient_holder_reads_the_room_at_the_head_of_its_next_turn ... ok
test rooms::a_post_that_calls_on_a_patient_holder_wakes_it_at_once ... ok
test rooms::a_post_that_calls_on_the_holder_wakes_it_and_is_owed_an_answer ... ok
test rooms::a_resumed_member_reads_only_what_its_cursor_left ... ok
test rooms::a_room_that_does_not_seat_the_holder_leaves_it_deaf ... ok
test rooms::a_root_asking_to_share_is_refused_in_words_and_opens_nothing ... ok
test rooms::a_rostered_live_holder_is_woken_by_a_post_and_reads_the_room ... ok
test rooms::a_seat_is_told_what_a_room_is_once_before_its_first_reading ... ok
test rooms::an_agent_opens_a_shared_room_and_its_peer_reads_the_post ... ok
test rooms::an_agent_that_opens_a_standing_name_is_refused_and_the_room_is_untouched ... ok
test rooms::rooms_read_the_real_team_owner_without_any_agent_roles ... ok
test rooms::the_permission_card_names_the_room_its_members_and_where_it_will_hang ... ok
test schedule::a_once_at_fires_and_then_disables_itself ... ok
test schedule::a_person_who_turned_wakes_off_is_offered_no_wake_tool ... ok
test schedule::a_schedule_fires_under_the_permission_mode_its_entry_names ... ok
test schedule::a_short_every_fires_a_real_turn_on_the_schedule_s_own_session ... mbx[progress]: 90s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test schedule::a_wake_comes_back_as_a_turn_of_the_session_that_set_it ... ok
test schedule::an_overdue_schedule_fires_once_however_long_it_was_overdue ... ok
test schedule::ownership::a_live_legacy_pid_is_not_replaced_or_treated_as_a_modern_lease ... ok
test schedule::ownership::a_second_process_stands_by_while_only_the_owner_fires ... ok
test schedule::ownership::an_already_started_standby_takes_over_after_child_kill ... ok
test schedule::ownership::an_already_started_standby_takes_over_after_normal_shutdown ... mbx[progress]: 105s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test schedule::ownership::an_empty_legacy_file_is_preserved_during_its_possible_pid_write_window ... ok
test schedule::ownership::an_unlocked_modern_file_survives_shutdown_and_does_not_block_restart ... ok
test schedule::ownership::doctor_fix_preserves_held_and_released_modern_lease_files ... ok
test schedule::the_create_tool_writes_one_entry_a_person_can_read ... ok
test schedule::the_schedule_command_folds_to_stdout_under_print ... ok
test schedule::the_wake_command_answers_under_print_and_finds_nothing_standing ... ok
test schedule::the_wake_tool_sets_a_wake_on_the_session_and_writes_no_file ... ok
test sessions::a_resumed_session_carries_its_first_ask_and_what_was_said_in_it ... ok
test sessions::a_session_another_process_holds_cannot_be_continued ... ok
test sessions::continue_and_resume_reopen_the_journal_the_last_run_wrote ... ok
test sessions::max_turns_stops_a_tool_loop_with_a_named_error ... ok
test sessions::resuming_an_unknown_session_is_not_found_before_any_turn ... ok
test shell::a_bang_line_journals_one_shell_item_and_spends_no_turn ... ok
test shell::a_bang_line_reaches_a_host_as_the_user_message_it_becomes ... ok
test shell::a_bang_with_nothing_after_it_is_refused ... ok
test skills::a_plugin_s_page_is_a_skill_the_model_can_read_by_name ... ok
test skills::a_project_skill_is_a_command_whose_body_becomes_the_prompt ... ok
test skills::an_unknown_slash_command_is_still_refused ... ok
test skills::the_prompt_lists_a_page_for_every_plugin_that_owns_a_noun ... ok
test skills::the_prompt_lists_the_pages_the_loaded_plugins_wrote ... ok
test stream_json::a_denied_permission_stops_the_tool_and_the_turn_goes_on ... ok
test stream_json::a_failed_turn_does_not_end_the_run ... ok
test stream_json::a_failed_turn_is_a_result_line_with_errors ... ok
test stream_json::a_junk_line_is_a_diagnostic_and_the_next_prompt_still_runs ... ok
test stream_json::a_permission_is_asked_of_the_host_and_an_allow_runs_the_tool ... ok
test stream_json::a_sub_sessions_lines_carry_the_call_that_spawned_it ... ok
test stream_json::an_interrupt_ends_the_running_turn ... ok
test stream_json::stdin_closing_with_nothing_to_do_is_a_clean_exit ... ok
test stream_json::stream_json_is_init_then_messages_then_result_and_nothing_else ... ok
test stream_json::the_host_protocol_needs_the_headless_surface_and_a_way_to_answer ... ok
test stream_json::two_user_lines_are_two_turns_with_one_result_each ... ok
test stream_json::without_the_prompt_tool_a_permission_is_still_refused ... ok
test tasks::the_journal_carries_the_list_from_one_run_to_the_next ... ok
test update::a_headless_run_that_sets_the_measure_is_not_told_the_key_is_unknown ... mbx[progress]: 120s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test update::a_headless_run_that_turns_the_check_off_is_not_told_the_key_is_unknown ... ok
test update::an_api_that_answers_nothing_is_exit_one_with_nothing_on_stdout ... ok
test update::an_archive_that_is_not_what_the_list_says_is_refused ... ok
test update::check_reports_the_two_versions_and_nothing_else ... ok
test update::the_binary_becomes_the_release_it_was_told_about ... ok
test views::a_binary_without_a_script_has_no_fake_provider ... ok
test views::an_edit_carries_its_diff_view_in_the_json_frames ... ok
test views::an_edit_prints_its_diff_under_the_verdict_in_text_mode ... ok
test views::an_instant_command_prints_its_answer_and_leaves ... ok
test web_fetch_hands_the_model_the_page_as_markdown ... ok

test result: ok. 252 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 110.35s

     Running tests/instances.rs (target/debug/deps/instances-cfe75de9a78cc4f8)

running 1 test
test the_catalogue_lists_each_instance_with_its_own_credential ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/login.rs (target/debug/deps/login-6b36d1f037157bc6)

running 1 test
test a_pasted_credential_is_stored_and_the_provider_is_usable_at_once ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

     Running tests/plugin_kit.rs (target/debug/deps/plugin_kit-9f9c221ebdbf0ed9)

running 5 tests
test missing_entry_is_a_clean_validation_failure ... ok
test scenario_refuses_a_plugin_that_did_not_start ... ok
test typescript_acceptance_runs_outside_the_workspace ... ok
test wordcount_validates_with_the_existing_protocol ... ok
test wrong_expectation_names_the_step_and_exits_one ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s

     Running tests/plugin_rpc.rs (target/debug/deps/plugin_rpc-3c61823027b0b6e3)

running 7 tests
test a_host_with_no_plugins_installed_runs_as_it_always_did ... ok
test a_stub_completion_is_in_the_turns_terminal_bill ... ok
test an_explicit_plugin_directory_is_loaded_by_the_cli ... ok
test interrupting_a_remote_first_phase_cancels_its_chain ... ok
test the_plugin_s_command_answers_with_its_view ... ok
test the_plugin_s_tool_counts_the_file_the_session_is_working_in ... ok
test the_plugin_s_tool_reaches_the_model_named_for_the_plugin_and_untrusted ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.82s

     Running tests/pty/main.rs (target/debug/deps/pty-ff2d7b7443e02066)

running 16 tests
test a_bang_line_runs_the_shell_and_leaves_its_output_in_the_transcript ... ok
test a_passthrough_tmux_says_is_off_is_never_asked_through ... ok
test a_picture_an_answer_named_in_markdown_is_read_in_and_sent ... mbx[progress]: 135s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test a_picture_under_tmux_travels_in_the_passthrough_envelope ... ok
test a_terminal_that_answers_only_da1_gets_the_chip ... ok
test a_terminal_that_answers_the_graphics_probe_is_sent_the_picture ... ok
test a_terminal_that_says_ok_and_draws_no_placeholder_gets_the_chip ... ok
test a_tmux_that_carries_nothing_through_gets_the_chip ... mbx[progress]: 150s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test an_answer_that_lands_after_the_probe_is_eaten_and_still_counted ... ok
test esc_twice_rewinds_the_turn_and_the_file_it_wrote ... ok
test every_frame_is_written_inside_a_synchronized_update ... mbx[progress]: 165s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test leaving_prints_the_last_screenful_into_the_shells_own_screen ... ok
test look::a_named_look_is_never_asked_what_ground_the_terminal_has ... ok
test look::a_terminal_whose_ground_turns_light_is_followed_within_one_focus ... ok
test look::a_theme_report_holds_crossterms_parser_and_swallows_what_follows ... ok
test no_print_on_exit_leaves_the_shell_as_it_was ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 54.27s

     Running tests/room_posts.rs (target/debug/deps/room_posts-b8c1c0ccee27f7b0)

running 14 tests
test tests::a_bounced_post_is_posted_again_by_one_word ... ok
test tests::a_call_that_says_both_or_neither_is_refused_as_input ... ok
test tests::a_message_to_a_room_is_a_post ... ok
test tests::a_message_wakes_the_child_and_says_who_wrote_it ... ok
test tests::a_name_nobody_has_is_an_error_result_the_model_can_correct ... ok
test tests::a_post_that_landed_since_bounces_the_repeat_and_the_next_one_lands ... ok
test tests::a_stale_post_bounces_and_lands_nothing ... ok
test tests::a_teammate_is_written_to_by_name ... ok
test tests::again_to_an_agent_is_refused_as_input ... ok
test tests::again_with_nothing_to_repeat_delivers_nothing ... ok
test tests::it_reads_only_and_names_the_agent_a_rule_may_match ... ok
test tests::nothing_lands_in_a_closed_room ... ok
test tests::the_card_says_where_it_went_and_when_it_will_be_read ... ok
test tests::the_description_says_a_direct_message_owes_nothing ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/rooms.rs (target/debug/deps/rooms-5cde94d03d0d3918)

running 2 tests
test a_post_in_a_room_wakes_its_member_and_is_not_fanned_back ... mbx[progress]: 180s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test a_room_member_that_asks_a_person_is_answered_through_the_root ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.94s

     Running tests/rpc.rs (target/debug/deps/rpc-5d46bc2528937ae8)

running 15 tests
test a_background_agent_wakes_the_root_and_says_who_it_is ... ok
test a_foreground_agent_is_a_child_session_on_the_root_s_attachment ... ok
test a_method_before_initialize_is_refused ... ok
test a_permission_is_answered_over_the_wire_and_the_tool_runs ... ok
test a_retry_is_visible_on_the_wire ... ok
test a_session_written_by_a_print_run_reopens_by_id_with_its_items ... ok
test a_shell_line_and_a_permission_mode_dispatch_as_commands ... ok
test an_interrupt_reaches_a_running_turn ... ok
test an_mcp_server_from_mcp_config_offers_its_tool_through_the_gate ... ok
test delete_removes_the_session_from_disk_and_shutdown_exits_zero ... ok
test one_interrupt_ends_the_turn_and_the_command_it_was_running ... ok
test open_submit_and_the_events_arrive_in_seq_order_with_the_clients_intent ... ok
test session_open_takes_a_thinking_level_and_the_config_view_says_it ... ok
test stdout_carries_json_rpc_lines_only_and_an_unknown_method_is_refused ... ok
test the_catalogue_lists_the_fake_provider_the_tools_and_the_model_facts ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.10s

     Running tests/thinking.rs (target/debug/deps/thinking-b8eb61c3a4b898d6)

running 2 tests
test a_child_spawned_at_a_level_asks_for_it_from_its_first_turn ... ok
test a_level_a_turn_sets_lands_on_the_next_turn_and_not_on_its_own ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/versioned_record.rs (target/debug/deps/versioned_record-efe61c67c2cc232e)

running 1 test
test unknown_records_resume_and_replay_over_rpc ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

     Running tests/views.rs (target/debug/deps/views-a9b65d349ea362b1)

running 3 tests
test a_button_fires_its_command_and_the_table_changes ... FAILED
test a_second_run_reads_the_board_back_and_not_the_bar ... FAILED
test a_signal_is_on_the_live_stream_and_never_in_the_journal ... ok

failures:

---- a_button_fires_its_command_and_the_table_changes stdout ----

thread 'a_button_fires_its_command_and_the_table_changes' (6489993) panicked at crates/bingo/tests/views.rs:63:28:
no board in {"_bingo.context": {"context:instructions": Null, "context:memory": Null, "memory": Object {"user": String("/var/folders/5d/ylkbk1nx6mvgq8zgnsjf8lwr0000gn/T/.tmpOh6mPy/.bingo/data/memory/user"), "project": String("/var/folders/5d/ylkbk1nx6mvgq8zgnsjf8lwr0000gn/T/.tmpOh6mPy/.bingo/data/memory/_tmpOh6mPy-05c2cb2411fa1439")}}, "bingo.schedule": {"wake": Null}}
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/core/src/panicking.rs:80:14
   2: views::board::{{closure}}
   3: core::option::Option<T>::unwrap_or_else
   4: views::board
   5: views::a_button_fires_its_command_and_the_table_changes::{{closure}}
   6: <core::pin::Pin<P> as core::future::future::Future>::poll
   7: tokio::runtime::park::CachedParkThread::block_on::{{closure}}
   8: tokio::runtime::park::CachedParkThread::block_on
   9: tokio::runtime::context::blocking::BlockingRegionGuard::block_on
  10: tokio::runtime::scheduler::multi_thread::MultiThread::block_on::{{closure}}
  11: tokio::runtime::context::runtime::enter_runtime
  12: tokio::runtime::scheduler::multi_thread::MultiThread::block_on
  13: tokio::runtime::runtime::Runtime::block_on_inner
  14: tokio::runtime::runtime::Runtime::block_on
  15: views::a_button_fires_its_command_and_the_table_changes
  16: views::a_button_fires_its_command_and_the_table_changes::{{closure}}
  17: core::ops::function::FnOnce::call_once
  18: <fn() -> core::result::Result<(), alloc::string::String> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.

---- a_second_run_reads_the_board_back_and_not_the_bar stdout ----

thread 'a_second_run_reads_the_board_back_and_not_the_bar' (6490108) panicked at crates/bingo/tests/views.rs:235:5:
assertion `left == right` failed
  left: "running"
 right: "done"
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/core/src/panicking.rs:80:14
   2: core::panicking::assert_failed_inner
   3: core::panicking::assert_failed
   4: views::a_second_run_reads_the_board_back_and_not_the_bar::{{closure}}
   5: <core::pin::Pin<P> as core::future::future::Future>::poll
   6: tokio::runtime::park::CachedParkThread::block_on::{{closure}}
   7: tokio::runtime::park::CachedParkThread::block_on
   8: tokio::runtime::context::blocking::BlockingRegionGuard::block_on
   9: tokio::runtime::scheduler::multi_thread::MultiThread::block_on::{{closure}}
  10: tokio::runtime::context::runtime::enter_runtime
  11: tokio::runtime::scheduler::multi_thread::MultiThread::block_on
  12: tokio::runtime::runtime::Runtime::block_on_inner
  13: tokio::runtime::runtime::Runtime::block_on
  14: views::a_second_run_reads_the_board_back_and_not_the_bar
  15: views::a_second_run_reads_the_board_back_and_not_the_bar::{{closure}}
  16: core::ops::function::FnOnce::call_once
  17: <fn() -> core::result::Result<(), alloc::string::String> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.


failures:
    a_button_fires_its_command_and_the_table_changes
    a_second_run_reads_the_board_back_and_not_the_bar

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.40s

error: test failed, to rerun pass `-p bingo --test views`
mbx[cache]: 0 hits, 0 misses, 23 incremental; 0 B downloaded, 0 B uploaded, 0 B stored locally
mbx[savings]: 2d 16h of rustc's calendar cleared across 3917 builds.
```

</details>

<details>
<summary>views-button-alone — exit 101</summary>

```text
$ cargo test -p bingo --test views a_button_fires_its_command_and_the_table_changes --locked -- --exact --nocapture
      ┌─────────┐  mr boxington
     ╱ ═══════ ╱│
    ╱─────────╱ │
    │ •    (◉)│≡│
    │  ◟▄▄▄◞  │ ╱
    │   ◀◆▶   │╱
    └─────────┘
   Compiling bingo v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.06s
     Running tests/views.rs (target/debug/deps/views-2d88d668bc023201)

running 1 test

thread 'a_button_fires_its_command_and_the_table_changes' (6492127) panicked at crates/bingo/tests/views.rs:63:28:
no board in {"_bingo.context": {"context:instructions": Null, "context:memory": Null, "memory": Object {"user": String("/var/folders/5d/ylkbk1nx6mvgq8zgnsjf8lwr0000gn/T/.tmpa69RG0/.bingo/data/memory/user"), "project": String("/var/folders/5d/ylkbk1nx6mvgq8zgnsjf8lwr0000gn/T/.tmpa69RG0/.bingo/data/memory/_tmpa69RG0-f21e1e0ab141f153")}}, "bingo.schedule": {"wake": Null}}
stack backtrace:
   0: __rustc::rust_begin_unwind
             at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/std/src/panicking.rs:689:5
   1: core::panicking::panic_fmt
             at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/core/src/panicking.rs:80:14
   2: views::board::{{closure}}
   3: core::option::Option<T>::unwrap_or_else
   4: views::board
   5: views::a_button_fires_its_command_and_the_table_changes::{{closure}}
   6: <core::pin::Pin<P> as core::future::future::Future>::poll
   7: tokio::runtime::park::CachedParkThread::block_on::{{closure}}
   8: tokio::runtime::park::CachedParkThread::block_on
   9: tokio::runtime::context::blocking::BlockingRegionGuard::block_on
  10: tokio::runtime::scheduler::multi_thread::MultiThread::block_on::{{closure}}
  11: tokio::runtime::context::runtime::enter_runtime
  12: tokio::runtime::scheduler::multi_thread::MultiThread::block_on
  13: tokio::runtime::runtime::Runtime::block_on_inner
  14: tokio::runtime::runtime::Runtime::block_on
  15: views::a_button_fires_its_command_and_the_table_changes
  16: views::a_button_fires_its_command_and_the_table_changes::{{closure}}
  17: core::ops::function::FnOnce::call_once
  18: <fn() -> core::result::Result<(), alloc::string::String> as core::ops::function::FnOnce<()>>::call_once
             at /rustc/ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96/library/core/src/ops/function.rs:250:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
test a_button_fires_its_command_and_the_table_changes ... FAILED

failures:

failures:
    a_button_fires_its_command_and_the_table_changes

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.28s

error: test failed, to rerun pass `-p bingo --test views`
mbx[cache]: 0 hits, 0 misses, 1 not looked up; 0 B downloaded, 0 B uploaded, 11.7 MiB stored locally
mbx[savings]: your deleted worktrees left 1.0 TiB logical behind. left. past tense.
```

</details>

<details>
<summary>views-order — exit 0</summary>

```text
$ cargo test -p bingo --test views --locked -- --test-threads=1
      ┌─────────┐  mr boxington
     ╱ ═══════ ╱│
    ╱─────────╱ │
    │ •    (◉)│≡│
    │  ◟▄▄▄◞  │ ╱
    │   ◀◆▶   │╱
    └─────────┘
   Compiling bingo-core v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-core)
   Compiling bingo-gateway v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-gateway)
   Compiling bingo v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 8.07s
     Running tests/views.rs (target/debug/deps/views-2d88d668bc023201)

running 3 tests
test a_button_fires_its_command_and_the_table_changes ... ok
test a_second_run_reads_the_board_back_and_not_the_bar ... ok
test a_signal_is_on_the_live_stream_and_never_in_the_journal ... mbx[progress]: 15s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.37s

mbx[cache]: 0 hits, 0 misses, 4 incremental; 0 B downloaded, 0 B uploaded, 0 B stored locally
mbx[savings]: 274.0 GiB of target/ is an elaborate illusion. the disk is in on it.
```

</details>

<details>
<summary>core-catalogue — exit 0</summary>

```text
$ cargo test -p bingo-core host::tests::catalogue --locked
      ┌─────────┐  mr boxington
     ╱ ═══════ ╱│
    ╱─────────╱ │
    │ •    (◉)│≡│
    │  ◟▄▄▄◞  │ ╱
    │   ◀◆▶   │╱
    └─────────┘
   Compiling bingo-sdk v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-sdk)
   Compiling bingo-core v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-core)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 10.91s
     Running unittests src/lib.rs (target/debug/deps/bingo_core-d7e91e10cc2d3356)

running 3 tests
test host::tests::catalogue::a_denied_setting_changes_neither_the_session_nor_the_file ... ok
test host::tests::catalogue::a_passive_observer_cannot_hold_a_command_acknowledgement ... ok
test host::tests::catalogue::lifecycle_facts_are_ordered_and_completion_text_follows_the_answer ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 480 filtered out; finished in 0.01s

mbx[cache]: 0 hits, 0 misses, 2 incremental; 0 B downloaded, 0 B uploaded, 0 B stored locally
mbx[savings]: 1.0 TiB logical of target/ belonged to checkouts that no longer exist. neither does it.
```

</details>

<details>
<summary>fmt-final — exit 0</summary>

```text
$ cargo fmt --all -- --check
```

</details>

<details>
<summary>check-final — exit 0</summary>

```text
$ cargo check --workspace --all-targets --locked
      ┌─────────┐  mr boxington
     ╱ ═══════ ╱│
    ╱─────────╱ │
    │ •    (◉)│≡│
    │  ◟▄▄▄◞  │ ╱
    │   ◀◆▶   │╱
    └─────────┘
    Checking bingo-core v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-core)
    Checking bingo-gateway v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-gateway)
    Checking bingo v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.06s
mbx[cache]: 0 hits, 0 misses, 24 incremental; 0 B downloaded, 0 B uploaded, 0 B stored locally
mbx[savings]: 274.2 GiB of outputs, one copy, several very confident checkouts.
```

</details>

<details>
<summary>clippy-final — exit 0</summary>

```text
$ cargo clippy --workspace --all-targets --locked -- -D warnings
      ┌─────────┐  mr boxington
     ╱ ═══════ ╱│
    ╱─────────╱ │
    │ •    (◉)│≡│
    │  ◟▄▄▄◞  │ ╱
    │   ◀◆▶   │╱
    └─────────┘
    Checking bingo-core v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-core)
    Checking bingo-gateway v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-gateway)
    Checking bingo v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.94s
mbx[cache]: 0 hits, 0 misses, 24 incremental; 0 B downloaded, 0 B uploaded, 0 B stored locally
mbx[savings]: cleaned up 1.0 TiB logical after checkouts that left without saying goodbye.
```

</details>

<details>
<summary>workspace-final — exit 0</summary>

```text
$ cargo test --workspace --locked -- --skip acp::bridge::a_call_with_no_turn_in_flight_is_refused_with_a_reason --test-threads=1
      ┌─────────┐  mr boxington
     ╱ ═══════ ╱│
    ╱─────────╱ │
    │ •    (◉)│≡│
    │  ◟▄▄▄◞  │ ╱
    │   ◀◆▶   │╱
    └─────────┘
   Compiling bingo-core v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-core)
   Compiling bingo-gateway v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-gateway)
   Compiling bingo v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 13.16s
     Running unittests src/main.rs (target/debug/deps/bingo-735e7126a68a01d2)

running 22 tests
test acp_proxy::tests::a_proxy_nobody_spawned_says_which_word_it_is_missing ... ok
test channels::tests::only_an_adapter_that_signs_is_accepted_and_the_refusal_lists_the_ones_that_do ... ok
test channels::tests::the_receipt_names_the_file_and_the_key_and_no_secret ... ok
test env::tests::no_environment_variable_is_named_in_the_binary ... ok
test mcp::tests::a_name_nobody_configured_is_refused_with_the_names_that_are ... ok
test mcp::tests::a_pair_that_is_not_a_pair_is_refused_by_the_text_that_was_typed ... ok
test mcp::tests::a_row_the_plugin_would_refuse_is_refused_here ... ok
test mcp::tests::a_server_with_nothing_configured_says_none_rather_than_nothing ... ok
test mcp::tests::a_stdio_entry_keeps_the_arguments_after_the_command ... ok
test mcp::tests::an_http_entry_is_the_row_the_plugin_reads ... ok
test mcp::tests::get_prints_the_names_of_the_secrets_and_not_their_values ... ok
test mcp::tests::the_listing_is_one_line_per_server_and_says_so_when_there_are_none ... ok
test plugins::schema::tests::committed_scenario_schema_is_generated ... ok
test plugins::schema::tests::scenario_contract_names_every_step ... ok
test plugins::switch::tests::a_listing_is_one_tab_separated_line_per_plugin ... ok
test plugins::switch::tests::a_name_no_listing_knows_is_refused ... ok
test provider::tests::a_key_that_is_not_an_object_is_refused_rather_than_overwritten ... ok
test provider::tests::a_missing_file_is_an_empty_document_and_a_broken_one_is_an_error ... ok
test provider::tests::a_protocol_is_one_of_two_words_however_it_is_typed ... ok
test provider::tests::a_receipt_names_both_files_and_the_command_that_uses_them ... ok
test provider::tests::an_instance_joins_the_document_without_moving_its_neighbours ... ok
test tests::the_highest_layer_that_names_the_measure_is_the_one_read ... ok

test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/acp_asked.rs (target/debug/deps/acp_asked-b4d3a9810271a3a7)

running 1 test
test a_person_answers_the_agents_question_and_the_agent_gets_their_choice ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s

     Running tests/acp_bridge.rs (target/debug/deps/acp_bridge-c166c0da5ddb0423)

running 3 tests
test a_proxy_ends_when_its_bridge_does_though_its_stdin_is_open ... ok
test a_proxy_with_a_token_this_run_never_minted_gets_nothing ... ok
test an_agent_reaches_the_shared_tools_through_the_spawned_proxy ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/catalogue.rs (target/debug/deps/catalogue-3d7684a9bbcd808b)

running 2 tests
test built_in_tool_inputs_leave_the_envelope_fields_free ... ok
test the_real_mcp_wire_face_raises_its_declared_event ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

     Running tests/channel_claim.rs (target/debug/deps/channel_claim-ba0e4b68f67773be)

running 1 test
test a_live_channel_host_excludes_another_and_a_killed_one_releases ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

     Running tests/channels.rs (target/debug/deps/channels-24ad4b705114f095)

running 16 tests
test a_blocklisted_principal_in_a_group_is_answered_with_nothing ... ok
test a_chats_own_rule_engages_without_the_mention_the_others_need ... ok
test a_group_is_ignored_until_the_bot_is_mentioned ... ok
test a_legacy_channel_pid_file_refuses_loudly ... ok
test a_message_opens_a_session_and_the_answer_streams_into_one_edited_message ... ok
test a_new_session_keeps_the_thread_and_answers_the_next_message ... mbx[progress]: 15s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test a_path_that_is_not_there_is_refused_in_words_the_model_reads ... ok
test a_permission_is_buttons_and_a_click_answers_it ... ok
test a_prompt_from_a_chat_carries_its_principal_and_its_conversation ... ok
test a_question_answered_at_another_surface_edits_the_card ... ok
test a_queued_command_returns_its_result_to_the_chat ... ok
test loopback_without_the_switch_is_refused_before_listening ... ok
test only_an_admin_is_heard_in_a_group_that_admits_only_admins ... ok
test the_model_posts_a_file_into_the_chat_it_is_talking_in ... ok
test the_working_sign_brackets_the_turn_a_message_started ... ok
test without_buttons_the_numbered_rung_is_drawn_and_a_reply_answers_it ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.23s

     Running tests/cli/main.rs (target/debug/deps/cli-b6afe1881793096f)

running 252 tests
test a_failed_turn_is_one_error_line_and_exit_1 ... ok
test a_form_crosses_the_json_wire_as_one_frame ... ok
test a_missing_prompt_and_an_unknown_provider_are_errors_before_any_turn ... ok
test a_pipe_without_print_still_runs_headlessly ... ok
test a_print_run_never_plays_the_opening ... ok
test a_question_is_declined_when_nobody_is_at_the_keyboard ... ok
test a_slow_bash_command_streams_its_tail_as_deltas ... ok
test a_tool_round_reads_the_file_and_reports_on_stderr ... ok
test acp::a_turn_through_an_adapter_streams_text_thought_and_the_agents_own_call ... ok
test acp::an_interrupt_cancels_the_turn_and_the_child_serves_the_next_one ... ok
test acp::asking::a_bypass_session_answers_the_agents_own_allow_option ... ok
test acp::asking::a_dont_ask_session_answers_the_agents_own_reject_option ... ok
test acp::asking::a_headless_run_puts_the_question_and_falls_closed_when_nobody_answers ... ok
test acp::asking::an_elicitation_is_still_declined_and_said_once ... ok
test acp::bridge::a_bridged_call_posts_to_the_parent_and_is_journaled_under_the_turn ... ok
test acp::bridge::a_persons_own_servers_are_forwarded_verbatim_and_leave_the_offer ... ok
test acp::bridge::a_row_that_keeps_its_servers_home_serves_their_tools_on_the_bridge ... ok
test acp::bridge::a_row_that_names_its_tools_is_offered_only_those ... ok
test acp::bridge::a_row_this_agent_cannot_take_is_skipped_and_named ... ok
test acp::bridge::a_tool_a_plugin_registered_reaches_the_bridge_with_no_edit_here ... ok
test acp::bridge::acp_reads_only_enabled_rows_from_the_real_mcp_owner ... ok
test acp::bridge::an_interrupt_drops_a_bridged_call_and_the_child_serves_the_next_turn ... ok
test acp::bridge::the_bridge_offers_the_sessions_tools_and_not_the_agents_own_hands ... ok
test acp::catalogue::a_cold_refresh_lists_the_agents_own_models ... ok
test acp::catalogue::a_live_sessions_declaration_supersedes_the_cold_one ... ok
test acp::catalogue::a_probe_that_cannot_start_the_adapter_serves_the_label_and_says_so ... ok
test acp::compacting_an_acp_session_is_refused_in_the_adapters_name ... ok
test acp::knobs::a_model_change_reaches_the_agent_as_its_own_value ... ok
test acp::knobs::a_respawned_child_is_set_from_the_row_again ... ok
test acp::knobs::a_row_effort_and_a_later_change_are_two_messages ... ok
test acp::knobs::a_row_option_reaches_the_agent_once_before_its_first_prompt ... ok
test acp::knobs::a_spawn_that_names_the_agents_model_opens_the_session_already_set ... ok
test acp::knobs::a_thinking_change_reaches_the_agent_before_the_next_prompt ... ok
test acp::knobs::an_adapter_with_only_the_legacy_door_is_set_through_it ... ok
test acp::knobs::an_agent_with_neither_knob_gets_no_config_call_and_one_notice ... ok
test acp::knobs::an_option_the_agent_never_declared_is_one_notice_and_no_call ... ok
test acp::knobs::the_agent_label_applies_nothing ... ok
test acp::knobs::the_model_list_is_the_agents_own_once_a_session_has_opened ... ok
test acp::life::a_child_that_dies_at_the_prompt_is_replaced_and_asked_again ... ok
test acp::life::a_child_that_dies_mid_stream_fails_the_turn_it_was_answering ... ok
test acp::life::an_adapter_that_died_between_turns_is_replaced_and_said ... ok
test acp::the_agents_own_count_its_window_and_its_own_compaction_reach_every_surface ... ok
test acp::the_agents_session_id_is_journaled_once_as_an_extension ... ok
test acp::the_restore_ladder_climbs_resume_then_load_then_a_file ... ok
test acp::two_turns_of_one_session_ride_one_child_and_one_agent_session ... ok
test again::a_bounced_post_lands_on_the_word_after_it ... ok
test agents::a_child_has_no_spawn_agent_to_call ... ok
test agents::a_child_that_failed_is_an_error_result_for_the_root ... ok
test agents::a_childs_permission_is_refused_off_a_tty_in_every_output_format ... mbx[progress]: 30s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test agents::a_dismissed_agent_s_session_is_deleted_from_the_data_dir ... ok
test agents::a_finished_background_agent_wakes_the_run_that_spawned_it ... ok
test agents::a_foreground_agent_answers_the_root_and_stdout_stays_the_root_s ... ok
test agents::a_project_definition_names_the_agent_it_starts ... ok
test agents::a_project_s_roles_are_seated_before_the_root_s_first_turn ... ok
test agents::a_resumed_root_is_told_which_child_was_mid_turn_when_the_process_ended ... ok
test agents::a_wake_that_finds_the_parent_busy_is_never_lost ... ok
test agents::an_unwoken_member_has_no_turn_and_its_brief_is_not_journalled ... ok
test agents::dismissing_a_child_keeps_print_stdout_valid_ndjson ... ok
test agents::dismissing_is_gated_and_a_refused_dismissal_deletes_nothing ... ok
test agents::stopping_an_idle_agent_is_an_error_result_that_says_so ... ok
test agents::the_child_s_reply_is_the_tool_call_s_result ... ok
test agents::the_mint_names_the_root_and_leaves_a_seat_its_own_name ... ok
test agents::the_same_roles_come_back_on_continue ... ok
test an_edit_is_asked_and_denied_off_a_tty_under_the_default_policy ... ok
test anthropic_without_credentials_fails_before_any_turn ... ok
test asking::a_permission_a_rule_could_silence_offers_the_session_rung ... ok
test asking::a_permission_no_rule_could_silence_offers_no_session_rung ... ok
test bash_runs_only_when_the_flags_allow_it ... ok
test batch::two_commands_in_one_step_both_come_back ... ok
test board::a_member_reads_a_room_s_board_claims_a_task_and_the_parent_sees_it ... ok
test board::a_session_s_own_list_is_untouched_by_the_board ... ok
test board::a_task_left_by_a_member_that_is_gone_reads_as_gone ... ok
test checkpoints::a_file_a_turn_created_is_removed_and_the_snapshots_outlive_nothing ... ok
test checkpoints::rewinding_two_turns_puts_the_file_back_to_before_the_first_of_them ... ok
test compaction_prefix::manual_summary_uses_the_old_baseline_and_next_request_refreshes_it ... ok
test compaction_prefix::rejected_summary_neither_executes_tools_nor_refreshes_the_baseline ... ok
test context::a_memory_the_model_writes_by_hand_is_what_memory_lists ... ok
test context::an_overflow_after_many_rounds_is_summarised_and_the_turn_goes_on ... ok
test context::an_overflow_is_retried_once_and_the_window_is_learned_on_disk ... mbx[progress]: 45s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test context::the_context_warning_is_said_once_near_the_line ... ok
test experience::a_commit_is_proposed_as_the_file_it_would_write ... ok
test experience::an_outcome_without_evidence_is_an_input_error ... ok
test experience::commit_query_outcome_and_revise_round_trip_on_disk ... ok
test experience::the_command_folds_the_library_into_a_table ... ok
test gateway::a_gateway_stopped_the_instant_it_is_up_still_gives_its_pidfile_back ... ok
test gateway::a_pasted_channel_secret_lands_in_auth_json_and_doctor_names_its_source ... ok
test gateway::a_schedule_fires_with_no_terminal_attached ... ok
test gateway::a_second_start_refuses_and_names_the_pid_that_already_has_it ... ok
test gateway::a_stale_pidfile_is_reported_dead_and_doctor_fix_clears_it ... ok
test gateway::a_start_with_no_channel_refuses_before_anything_moves ... ok
test gateway::a_supervisor_that_refuses_leaves_no_service_file_behind ... ok
test gateway::channels_add_asks_for_both_and_writes_each_where_it_belongs ... ok
test gateway::doctor_names_a_missing_credential_and_never_prints_one ... ok
test gateway::install_writes_a_secret_free_service_and_the_verbs_then_delegate ... ok
test gateway::start_status_stop_round_trips_on_a_real_detached_process ... ok
test gateway::stopping_gives_back_the_schedule_runner_claim_and_the_pidfile ... ok
test gateway::tracing_lines_land_in_the_gateway_log_at_info_and_at_warn ... ok
test hooks::a_pre_tool_use_hook_that_denies_stops_the_write_and_tells_the_model ... ok
test images::a_picture_over_the_bound_reaches_the_journal_bounded ... ok
test images::a_stream_json_user_line_carries_its_image_blocks ... ok
test images::a_url_that_is_not_a_picture_is_exit_1_and_says_so ... ok
test images::an_image_flag_puts_the_picture_beside_the_prompt_in_the_journal ... ok
test images::an_image_flag_under_stream_json_input_is_refused ... ok
test images::an_image_of_a_wider_format_reaches_the_journal_as_png ... ok
test images::an_image_that_does_not_read_is_exit_1_with_nothing_on_stdout ... ok
test images::an_image_url_is_fetched_and_journaled ... ok
test images::an_image_url_over_the_cap_is_exit_1_with_nothing_on_stdout ... ok
test images::an_image_url_that_is_not_there_is_exit_1_with_nothing_on_stdout ... ok
test instances::a_colliding_instance_name_stops_the_boot_and_is_named ... ok
test instances::a_pasted_key_reaches_the_wire_and_a_logout_takes_it_away ... ok
test instances::an_exported_key_does_not_feed_a_named_instance ... ok
test instances::an_instance_is_reachable_by_provider_and_by_model ... ok
test instances::two_codex_instances_hold_two_credentials_and_refresh_apart ... ok
test jobs::a_bare_foreground_sleep_is_refused_and_starts_nothing ... ok
test jobs::a_default_watch_still_says_it_once_and_counts_nothing ... ok
test jobs::a_finished_job_opens_a_turn_on_a_headless_run ... ok
test jobs::a_job_is_pulled_by_cursor_across_two_turns_and_then_killed ... ok
test jobs::a_one_shot_run_ends_at_its_turn_and_the_job_ends_with_it ... ok
test jobs::a_pinned_watch_wakes_once_and_counts_the_rest_onto_the_end ... ok
test jobs::a_running_command_is_promoted_mid_turn_and_the_call_returns_early ... ok
test jobs::a_tail_f_backgrounds_itself_and_a_plain_command_does_not ... ok
test jobs::an_id_no_job_has_is_an_error_result_the_model_can_correct ... ok
test jobs::an_ongoing_watch_on_a_slow_stream_wakes_on_every_line ... ok
test jobs::notify_all_with_no_condition_is_refused_and_starts_no_job ... ok
test json_output_is_one_frame_per_line_ending_in_turn_completed ... ok
test login::a_401_is_followed_by_one_refresh_and_the_turn_completes ... ok
test login::a_device_sign_in_stores_the_token_and_the_next_turn_uses_it ... mbx[progress]: 60s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test login::a_provider_without_a_sign_in_says_so ... ok
test login::an_unknown_provider_is_refused_by_name ... ok
test login::before_any_sign_in_a_codex_turn_is_refused_by_name ... ok
test login::device_and_paste_exclude_each_other ... ok
test mcp::a_child_process_cannot_be_signed_in_to ... ok
test mcp::a_name_nobody_configured_is_one_error_line_and_exit_1 ... ok
test mcp::a_pasted_code_signs_in_and_the_token_lands_in_auth_json ... ok
test mcp::a_row_the_plugin_would_refuse_is_refused_before_it_is_written ... ok
test mcp::a_run_configured_by_flag_lists_what_that_run_would_dial ... ok
test mcp::a_server_is_added_read_back_listed_and_removed ... ok
test mcp::a_stdio_server_joins_the_ones_that_are_already_there ... ok
test mcp::get_never_prints_a_header_or_an_environment_value ... ok
test mcp::signing_out_of_a_server_nobody_signed_in_to_is_a_no_op ... ok
test mcp::the_print_surface_refuses_a_sign_in_and_names_the_way_through ... ok
test mcp_config_must_exist_and_carry_servers ... ok
test mentions::a_question_left_unanswered_is_chased_when_the_next_process_opens_the_room ... ok
test mentions::the_card_stands_while_a_question_does_and_goes_when_it_is_answered ... ok
test mentions::the_room_table_names_who_owes_an_answer ... ok
test models::a_model_flag_is_not_remembered ... ok
test models::a_spawn_that_names_a_provider_and_a_model_still_lands ... ok
test models::list_models_names_every_provider_its_models_and_its_sign_in_state ... ok
test models::models_refuses_an_argument_it_does_not_know ... ok
test models::the_models_command_lists_the_catalogue_and_refreshes_on_demand ... ok
test models::thinking_ignores_false_capability_metadata_without_rewriting_it ... ok
test models::thinking_off_and_invalid_levels_preserve_model_overrides ... ok
test models::thinking_on_an_unknown_model_is_effective_and_remembered ... ok
test openai_runs_a_tool_round_and_feeds_the_result_back ... ok
test openai_streams_a_text_turn_through_the_same_loop ... ok
test openai_without_credentials_names_the_variable_before_any_turn ... ok
test peers::a_bounced_post_neither_opens_nor_answers_a_mention_debt ... ok
test peers::a_post_reaches_each_member_exactly_once ... ok
test peers::a_sibling_dm_reaches_the_teammate_and_leaves_the_parent_out_of_it ... ok
test peers::a_stale_post_bounces_with_what_it_missed_and_lands_on_the_retry ... mbx[progress]: 75s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test peers::a_standby_member_nothing_wakes_runs_no_turn_at_all ... ok
test peers::an_answer_from_the_parent_wakes_the_child_that_asked ... ok
test peers::one_kickoff_post_runs_a_relay_the_parent_never_dispatches ... ok
test peers::waiting_for_a_standby_agent_is_refused_before_one_is_started ... ok
test persona::a_projects_own_text_replaces_the_stance_entirely ... ok
test persona::a_typo_under_the_key_stops_the_run_and_names_the_field ... ok
test persona::switched_off_the_stance_is_not_in_the_prompt ... ok
test persona::the_page_this_plugin_wrote_is_listed_in_the_prompt ... ok
test persona::the_stance_reaches_the_model_in_the_system_prompt ... ok
test persona::the_stance_sits_between_the_kernels_blocks_and_the_projects_own ... ok
test plan_mode_denies_a_write_and_the_turn_goes_on ... ok
test plugins::a_missing_used_service_is_named_without_disabling_its_consumer ... ok
test plugins::a_needed_plugin_and_a_name_nobody_lists_are_refused_with_the_reason ... ok
test plugins::a_tool_plugin_switched_off_is_written_listed_and_gone_from_the_model_s_tools ... ok
test plugins::an_external_plugin_is_listed_switched_and_never_spawned ... ok
test plugins::an_external_plugins_tier_is_listed_from_user_settings_only ... ok
test prefix::context_snapshots_cannot_name_a_session_started_by_a_skill ... ok
test prefix::knowledge_stays_fixed_between_rounds_and_refreshes_when_resumed ... ok
test print_streams_prose_to_stdout_and_nothing_else ... ok
test provider_add::a_comment_in_the_settings_survives_the_command_that_writes_beside_it ... ok
test provider_add::a_name_already_taken_is_refused_and_nothing_is_written ... ok
test provider_add::a_protocol_that_is_neither_is_refused_by_name ... ok
test provider_add::a_settings_json_left_from_an_older_bingo_crosses_on_the_first_run ... ok
test provider_add::an_added_provider_is_written_down_and_the_next_run_uses_it ... ok
test provider_add::an_anthropic_instance_needs_neither_an_endpoint_nor_a_key ... ok
test rooms::a_member_listens_and_the_room_s_journal_says_what_it_now_hears ... mbx[progress]: 90s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test rooms::a_patient_holder_reads_the_room_at_the_head_of_its_next_turn ... ok
test rooms::a_post_that_calls_on_a_patient_holder_wakes_it_at_once ... ok
test rooms::a_post_that_calls_on_the_holder_wakes_it_and_is_owed_an_answer ... ok
test rooms::a_resumed_member_reads_only_what_its_cursor_left ... ok
test rooms::a_room_that_does_not_seat_the_holder_leaves_it_deaf ... ok
test rooms::a_root_asking_to_share_is_refused_in_words_and_opens_nothing ... ok
test rooms::a_rostered_live_holder_is_woken_by_a_post_and_reads_the_room ... ok
test rooms::a_seat_is_told_what_a_room_is_once_before_its_first_reading ... ok
test rooms::an_agent_opens_a_shared_room_and_its_peer_reads_the_post ... ok
test rooms::an_agent_that_opens_a_standing_name_is_refused_and_the_room_is_untouched ... ok
test rooms::rooms_read_the_real_team_owner_without_any_agent_roles ... ok
test rooms::the_permission_card_names_the_room_its_members_and_where_it_will_hang ... ok
test schedule::a_once_at_fires_and_then_disables_itself ... ok
test schedule::a_person_who_turned_wakes_off_is_offered_no_wake_tool ... ok
test schedule::a_schedule_fires_under_the_permission_mode_its_entry_names ... ok
test schedule::a_short_every_fires_a_real_turn_on_the_schedule_s_own_session ... ok
test schedule::a_wake_comes_back_as_a_turn_of_the_session_that_set_it ... mbx[progress]: 105s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test schedule::an_overdue_schedule_fires_once_however_long_it_was_overdue ... ok
test schedule::ownership::a_live_legacy_pid_is_not_replaced_or_treated_as_a_modern_lease ... ok
test schedule::ownership::a_second_process_stands_by_while_only_the_owner_fires ... ok
test schedule::ownership::an_already_started_standby_takes_over_after_child_kill ... ok
test schedule::ownership::an_already_started_standby_takes_over_after_normal_shutdown ... ok
test schedule::ownership::an_empty_legacy_file_is_preserved_during_its_possible_pid_write_window ... ok
test schedule::ownership::an_unlocked_modern_file_survives_shutdown_and_does_not_block_restart ... ok
test schedule::ownership::doctor_fix_preserves_held_and_released_modern_lease_files ... ok
test schedule::the_create_tool_writes_one_entry_a_person_can_read ... ok
test schedule::the_schedule_command_folds_to_stdout_under_print ... ok
test schedule::the_wake_command_answers_under_print_and_finds_nothing_standing ... ok
test schedule::the_wake_tool_sets_a_wake_on_the_session_and_writes_no_file ... ok
test sessions::a_resumed_session_carries_its_first_ask_and_what_was_said_in_it ... ok
test sessions::a_session_another_process_holds_cannot_be_continued ... mbx[progress]: 120s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test sessions::continue_and_resume_reopen_the_journal_the_last_run_wrote ... ok
test sessions::max_turns_stops_a_tool_loop_with_a_named_error ... ok
test sessions::resuming_an_unknown_session_is_not_found_before_any_turn ... ok
test shell::a_bang_line_journals_one_shell_item_and_spends_no_turn ... ok
test shell::a_bang_line_reaches_a_host_as_the_user_message_it_becomes ... ok
test shell::a_bang_with_nothing_after_it_is_refused ... ok
test skills::a_plugin_s_page_is_a_skill_the_model_can_read_by_name ... ok
test skills::a_project_skill_is_a_command_whose_body_becomes_the_prompt ... ok
test skills::an_unknown_slash_command_is_still_refused ... ok
test skills::the_prompt_lists_a_page_for_every_plugin_that_owns_a_noun ... ok
test skills::the_prompt_lists_the_pages_the_loaded_plugins_wrote ... ok
test stream_json::a_denied_permission_stops_the_tool_and_the_turn_goes_on ... ok
test stream_json::a_failed_turn_does_not_end_the_run ... ok
test stream_json::a_failed_turn_is_a_result_line_with_errors ... ok
test stream_json::a_junk_line_is_a_diagnostic_and_the_next_prompt_still_runs ... ok
test stream_json::a_permission_is_asked_of_the_host_and_an_allow_runs_the_tool ... ok
test stream_json::a_sub_sessions_lines_carry_the_call_that_spawned_it ... ok
test stream_json::an_interrupt_ends_the_running_turn ... ok
test stream_json::stdin_closing_with_nothing_to_do_is_a_clean_exit ... ok
test stream_json::stream_json_is_init_then_messages_then_result_and_nothing_else ... ok
test stream_json::the_host_protocol_needs_the_headless_surface_and_a_way_to_answer ... ok
test stream_json::two_user_lines_are_two_turns_with_one_result_each ... ok
test stream_json::without_the_prompt_tool_a_permission_is_still_refused ... ok
test tasks::the_journal_carries_the_list_from_one_run_to_the_next ... ok
test update::a_headless_run_that_sets_the_measure_is_not_told_the_key_is_unknown ... ok
test update::a_headless_run_that_turns_the_check_off_is_not_told_the_key_is_unknown ... ok
test update::an_api_that_answers_nothing_is_exit_one_with_nothing_on_stdout ... ok
test update::an_archive_that_is_not_what_the_list_says_is_refused ... ok
test update::check_reports_the_two_versions_and_nothing_else ... ok
test update::the_binary_becomes_the_release_it_was_told_about ... ok
test views::a_binary_without_a_script_has_no_fake_provider ... ok
test views::an_edit_carries_its_diff_view_in_the_json_frames ... ok
test views::an_edit_prints_its_diff_under_the_verdict_in_text_mode ... ok
test views::an_instant_command_prints_its_answer_and_leaves ... ok
test web_fetch_hands_the_model_the_page_as_markdown ... ok

test result: ok. 252 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out; finished in 116.15s

     Running tests/instances.rs (target/debug/deps/instances-cfe75de9a78cc4f8)

running 1 test
test the_catalogue_lists_each_instance_with_its_own_credential ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/login.rs (target/debug/deps/login-6b36d1f037157bc6)

running 1 test
test a_pasted_credential_is_stored_and_the_provider_is_usable_at_once ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s

     Running tests/plugin_kit.rs (target/debug/deps/plugin_kit-9f9c221ebdbf0ed9)

running 5 tests
test missing_entry_is_a_clean_validation_failure ... ok
test scenario_refuses_a_plugin_that_did_not_start ... ok
test typescript_acceptance_runs_outside_the_workspace ... ok
test wordcount_validates_with_the_existing_protocol ... ok
test wrong_expectation_names_the_step_and_exits_one ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.44s

     Running tests/plugin_rpc.rs (target/debug/deps/plugin_rpc-3c61823027b0b6e3)

running 7 tests
test a_host_with_no_plugins_installed_runs_as_it_always_did ... ok
test a_stub_completion_is_in_the_turns_terminal_bill ... ok
test an_explicit_plugin_directory_is_loaded_by_the_cli ... ok
test interrupting_a_remote_first_phase_cancels_its_chain ... mbx[progress]: 135s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test the_plugin_s_command_answers_with_its_view ... ok
test the_plugin_s_tool_counts_the_file_the_session_is_working_in ... ok
test the_plugin_s_tool_reaches_the_model_named_for_the_plugin_and_untrusted ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.95s

     Running tests/pty/main.rs (target/debug/deps/pty-ff2d7b7443e02066)

running 16 tests
test a_bang_line_runs_the_shell_and_leaves_its_output_in_the_transcript ... ok
test a_passthrough_tmux_says_is_off_is_never_asked_through ... ok
test a_picture_an_answer_named_in_markdown_is_read_in_and_sent ... ok
test a_picture_under_tmux_travels_in_the_passthrough_envelope ... mbx[progress]: 150s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test a_terminal_that_answers_only_da1_gets_the_chip ... ok
test a_terminal_that_answers_the_graphics_probe_is_sent_the_picture ... ok
test a_terminal_that_says_ok_and_draws_no_placeholder_gets_the_chip ... ok
test a_tmux_that_carries_nothing_through_gets_the_chip ... mbx[progress]: 165s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test an_answer_that_lands_after_the_probe_is_eaten_and_still_counted ... ok
test esc_twice_rewinds_the_turn_and_the_file_it_wrote ... ok
test every_frame_is_written_inside_a_synchronized_update ... ok
test leaving_prints_the_last_screenful_into_the_shells_own_screen ... mbx[progress]: 180s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test look::a_named_look_is_never_asked_what_ground_the_terminal_has ... ok
test look::a_terminal_whose_ground_turns_light_is_followed_within_one_focus ... ok
test look::a_theme_report_holds_crossterms_parser_and_swallows_what_follows ... ok
test no_print_on_exit_leaves_the_shell_as_it_was ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 54.04s

     Running tests/room_posts.rs (target/debug/deps/room_posts-b8c1c0ccee27f7b0)

running 14 tests
test tests::a_bounced_post_is_posted_again_by_one_word ... ok
test tests::a_call_that_says_both_or_neither_is_refused_as_input ... ok
test tests::a_message_to_a_room_is_a_post ... ok
test tests::a_message_wakes_the_child_and_says_who_wrote_it ... ok
test tests::a_name_nobody_has_is_an_error_result_the_model_can_correct ... ok
test tests::a_post_that_landed_since_bounces_the_repeat_and_the_next_one_lands ... ok
test tests::a_stale_post_bounces_and_lands_nothing ... ok
test tests::a_teammate_is_written_to_by_name ... ok
test tests::again_to_an_agent_is_refused_as_input ... ok
test tests::again_with_nothing_to_repeat_delivers_nothing ... ok
test tests::it_reads_only_and_names_the_agent_a_rule_may_match ... ok
test tests::nothing_lands_in_a_closed_room ... ok
test tests::the_card_says_where_it_went_and_when_it_will_be_read ... ok
test tests::the_description_says_a_direct_message_owes_nothing ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/rooms.rs (target/debug/deps/rooms-5cde94d03d0d3918)

running 2 tests
test a_post_in_a_room_wakes_its_member_and_is_not_fanned_back ... ok
test a_room_member_that_asks_a_person_is_answered_through_the_root ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.79s

     Running tests/rpc.rs (target/debug/deps/rpc-5d46bc2528937ae8)

running 15 tests
test a_background_agent_wakes_the_root_and_says_who_it_is ... ok
test a_foreground_agent_is_a_child_session_on_the_root_s_attachment ... ok
test a_method_before_initialize_is_refused ... ok
test a_permission_is_answered_over_the_wire_and_the_tool_runs ... ok
test a_retry_is_visible_on_the_wire ... ok
test a_session_written_by_a_print_run_reopens_by_id_with_its_items ... ok
test a_shell_line_and_a_permission_mode_dispatch_as_commands ... ok
test an_interrupt_reaches_a_running_turn ... mbx[progress]: 195s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test an_mcp_server_from_mcp_config_offers_its_tool_through_the_gate ... ok
test delete_removes_the_session_from_disk_and_shutdown_exits_zero ... ok
test one_interrupt_ends_the_turn_and_the_command_it_was_running ... ok
test open_submit_and_the_events_arrive_in_seq_order_with_the_clients_intent ... ok
test session_open_takes_a_thinking_level_and_the_config_view_says_it ... ok
test stdout_carries_json_rpc_lines_only_and_an_unknown_method_is_refused ... ok
test the_catalogue_lists_the_fake_provider_the_tools_and_the_model_facts ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.50s

     Running tests/thinking.rs (target/debug/deps/thinking-b8eb61c3a4b898d6)

running 2 tests
test a_child_spawned_at_a_level_asks_for_it_from_its_first_turn ... ok
test a_level_a_turn_sets_lands_on_the_next_turn_and_not_on_its_own ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/versioned_record.rs (target/debug/deps/versioned_record-efe61c67c2cc232e)

running 1 test
test unknown_records_resume_and_replay_over_rpc ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s

     Running tests/views.rs (target/debug/deps/views-a9b65d349ea362b1)

running 3 tests
test a_button_fires_its_command_and_the_table_changes ... ok
test a_second_run_reads_the_board_back_and_not_the_bar ... ok
test a_signal_is_on_the_live_stream_and_never_in_the_journal ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.26s

     Running unittests src/lib.rs (target/debug/deps/bingo_agents-9eb7da80d0c10b13)

running 155 tests
test command::tests::a_session_that_started_nothing_says_so ... ok
test command::tests::a_verb_without_a_name_and_a_word_that_is_no_verb_are_told_the_shape ... ok
test command::tests::dismiss_deletes_an_idle_child_and_stop_refuses_one ... ok
test command::tests::the_spec_runs_now_and_names_the_two_verbs ... ok
test command::tests::the_table_names_every_child_and_what_it_is_doing ... ok
test definition::tests::a_block_yaml_cannot_read_leaves_the_system_prompt_alone ... ok
test definition::tests::a_definition_that_names_itself_nothing_is_named_after_its_file ... ok
test definition::tests::a_fence_that_is_not_the_first_line_is_content ... ok
test definition::tests::a_file_without_frontmatter_is_all_system_prompt ... ok
test definition::tests::a_tool_list_written_as_a_block_keeps_its_entries ... ok
test definition::tests::an_unclosed_block_is_content ... ok
test definition::tests::every_field_this_plugin_reads ... ok
test definition::tests::thinking_off_arrives_as_the_word_and_not_a_boolean ... ok
test dismiss::tests::a_busy_child_is_refused_and_told_which_verb_ends_its_turn ... ok
test dismiss::tests::a_teammate_and_a_stranger_are_refused_with_the_reason ... ok
test dismiss::tests::an_idle_child_is_deleted_and_gone_from_the_tree ... ok
test dismiss::tests::it_is_destructive_and_not_read_only_and_names_the_agent_as_its_subject ... ok
test guide::tests::the_page_is_one_line_of_description_and_a_body_worth_reading ... ok
test guide::tests::the_page_names_every_column_a_roster_shows ... ok
test guide::tests::the_page_names_every_tool_a_child_never_has ... ok
test guide::tests::the_page_names_every_tool_and_command_this_plugin_registers ... ok
test guide::tests::the_page_names_the_addresses_a_message_may_carry ... ok
test guide::tests::this_plugin_claims_no_settings_for_the_page_to_name ... ok
test hook::tests::a_line_addressed_to_a_child_is_stripped_and_redirected ... ok
test hook::tests::a_line_is_this_session_s_own_when_the_tree_cannot_be_read ... ok
test hook::tests::a_line_with_nothing_to_deliver_is_an_ordinary_prompt ... ok
test hook::tests::a_name_nobody_has_is_an_ordinary_prompt ... ok
test hook::tests::it_asks_for_the_submit_point_only ... ok
test hook::tests::what_counts_as_an_address ... ok
test layers::tests::a_home_inside_the_working_tree_is_read_once ... ok
test layers::tests::a_nested_directory_speaks_before_the_repository_around_it ... ok
test layers::tests::the_project_s_definitions_come_before_the_person_s ... ok
test layers::tests::the_walk_ends_at_the_filesystem_root ... ok
test library::tests::a_file_that_is_not_markdown_is_not_a_definition ... ok
test library::tests::a_layer_lists_its_definitions_by_file_name ... ok
test library::tests::a_nearer_project_layer_overrides_a_farther_one ... ok
test library::tests::an_edited_definition_is_seen_on_the_next_look ... ok
test library::tests::an_empty_machine_has_no_definitions ... ok
test library::tests::the_project_s_definition_overrides_the_person_s_of_that_name ... ok
test list::tests::a_child_sees_the_teammates_beside_it_marked_as_such ... ok
test list::tests::a_roster_with_nobody_on_it_draws_nothing ... ok
test list::tests::a_row_carries_what_the_summary_carries_and_a_dash_for_what_it_lacks ... ok
test list::tests::a_session_alone_at_the_top_lists_only_what_it_started ... ok
test list::tests::a_session_that_started_nothing_is_told_how_to ... ok
test list::tests::every_child_of_this_session_with_its_state ... ok
test list::tests::the_listing_and_the_table_have_the_same_columns ... ok
test list::tests::the_tree_wears_each_state_as_a_badge_and_keeps_the_teammates_apart ... ok
test message::voice_tests::producer_stamps_its_voice ... ok
test models::tests::a_model_the_snapshot_does_not_carry_is_listed_without_facts ... ok
test models::tests::a_provider_carries_its_sign_in_state_and_its_models_facts ... ok
test models::tests::a_provider_the_catalogue_files_nothing_under_says_so ... ok
test models::tests::a_sign_in_state_this_build_cannot_read_is_not_read_as_ready ... ok
test models::tests::it_reads_and_takes_no_arguments ... ok
test names::tests::a_base_runs_out_of_names_rather_than_looping ... ok
test names::tests::a_caller_is_not_beside_itself_and_a_room_is_nobody_s_teammate ... ok
test names::tests::a_name_a_sibling_holds_gets_the_next_suffix ... ok
test names::tests::a_name_nobody_has_says_which_agents_and_rooms_are_here ... ok
test names::tests::a_name_nobody_has_tells_a_teammate_apart_from_a_child ... ok
test names::tests::a_name_resolves_among_the_caller_s_children_and_parent_upwards ... ok
test names::tests::a_name_that_would_break_the_key_or_the_address_is_refused ... ok
test names::tests::a_room_resolves_from_the_session_that_holds_it_and_from_a_member ... ok
test names::tests::a_teammate_resolves_and_an_own_child_of_that_name_shadows_it ... ok
test names::tests::case_variants_cannot_claim_an_existing_or_reserved_name ... ok
test names::tests::case_variants_resolve_children_siblings_rooms_and_parent ... ok
test names::tests::mine_is_an_own_child_and_a_teammate_is_refused_with_the_reason ... ok
test names::tests::the_first_name_of_a_base_is_the_base_itself ... ok
test names::tests::the_root_signs_as_parent_and_a_child_by_its_own_name ... ok
test note::tests::a_definition_s_body_follows_the_note ... ok
test note::tests::a_definition_with_no_body_leaves_the_note_alone ... ok
test note::tests::the_note_leaves_the_room_protocol_to_the_rooms_plugin ... ok
test note::tests::the_note_names_the_ways_back_and_the_one_that_is_gone ... ok
test note::tests::the_note_teaches_the_brief_a_seated_agent_opens_on ... ok
test plugin_tests::every_tool_is_read_only_trusted_and_alone_but_the_one_that_deletes ... ok
test plugin_tests::registering_reads_nothing_and_contributes_what_the_manifest_promises ... ok
test plugin_tests::the_manifest_says_what_it_provides_and_claims_no_settings ... ok
test report::tests::a_gone_child_is_reported_and_parent_end_cancels_followers ... ok
test report::tests::a_second_owner_delivers_a_finished_child_and_clears_the_record ... ok
test report::tests::a_second_owner_follows_a_running_child_to_its_end ... ok
test report::tests::an_obligation_has_one_persisted_shape ... ok
test spawn::tests::a_background_spawn_names_the_child_at_once ... ok
test spawn::tests::a_child_is_offered_every_tool_but_the_one_it_could_not_use ... ok
test spawn::tests::a_definition_decides_what_the_call_did_not ... ok
test spawn::tests::a_definition_nobody_wrote_says_what_could_have_been_named ... ok
test spawn::tests::a_definition_s_level_is_taken_and_a_bad_one_names_the_definition ... ok
test spawn::tests::a_foreground_spawn_reports_a_child_that_failed_as_an_error ... ok
test spawn::tests::a_foreground_spawn_returns_the_child_s_final_text ... ok
test spawn::tests::a_key_a_live_session_holds_is_the_same_kind_of_collision ... ok
test spawn::tests::a_name_a_sibling_holds_gets_the_next_one ... ok
test spawn::tests::a_spawn_opens_the_child_at_the_level_it_names ... ok
test spawn::tests::a_standby_agent_nobody_could_wait_for_is_refused_in_words ... ok
test spawn::tests::a_standby_member_is_minted_exactly_as_a_woken_one ... ok
test spawn::tests::a_standby_member_leaves_no_watcher_to_wake_its_parent ... ok
test spawn::tests::a_standby_spawn_names_the_child_and_says_it_has_read_nothing ... ok
test spawn::tests::a_word_that_is_not_a_level_is_refused_with_the_list ... ok
test spawn::tests::it_reads_and_a_rule_may_name_the_agent_it_starts ... ok
test spawn::tests::reopen_hands_the_prompt_to_the_standing_child_and_mints_nothing ... ok
test spawn::tests::reopen_without_a_standing_child_is_an_ordinary_spawn ... ok
test spawn::tests::standby_holds_the_brief_and_a_plain_spawn_wakes_on_it ... ok
test spawn::tests::the_child_carries_the_note_the_parent_link_and_a_key_of_its_own ... ok
test spawn::tests::the_description_says_the_turn_may_end_and_the_reply_wakes_it ... ok
test spawn::tests::the_description_says_what_an_unlabelled_line_is ... ok
test spawn::tests::the_description_teaches_the_room_pattern_over_per_member_dispatch ... ok
test stop::tests::a_busy_child_is_interrupted_and_the_receipt_says_how_its_turn_ended ... ok
test stop::tests::a_refused_interrupt_reads_as_already_idle_and_waits_for_nothing ... ok
test stop::tests::a_teammate_and_a_stranger_are_refused_with_the_reason ... ok
test stop::tests::a_turn_that_ended_on_its_own_first_is_said_so ... ok
test stop::tests::an_idle_child_is_refused_before_anything_is_sent ... ok
test stop::tests::it_is_the_plugin_s_kind_of_tool_and_names_the_agent_as_its_subject ... ok
test team::command::tests::a_project_with_no_team_file_is_told_where_one_would_be ... ok
test team::command::tests::the_spec_runs_now_and_takes_nothing ... ok
test team::command::tests::the_table_names_every_declared_role_and_what_is_seated ... ok
test team::file::tests::a_directory_with_no_team_file_anywhere_above_it_has_no_team ... ok
test team::file::tests::a_file_that_declares_no_roles_seats_nobody ... ok
test team::file::tests::a_file_that_is_not_json_says_where_it_is ... ok
test team::file::tests::a_norms_file_a_team_named_and_does_not_have_is_a_mistake ... ok
test team::file::tests::a_role_without_a_usable_name_is_refused_by_name ... ok
test team::file::tests::every_field_a_role_declares_and_nothing_of_another_plugin_s ... ok
test team::file::tests::the_nearest_team_file_wins ... ok
test team::file::tests::the_norms_beside_the_file_when_it_names_none ... ok
test team::file::tests::the_norms_come_before_the_system_prompt_and_alone_when_there_is_none ... ok
test team::file::tests::the_norms_file_a_team_names ... ok
test team::seat::tests::a_child_session_seats_nobody ... ok
test team::seat::tests::a_definition_gives_a_role_what_it_does_not_declare_itself ... ok
test team::seat::tests::a_level_nobody_can_read_leaves_the_seat_inheriting ... ok
test team::seat::tests::a_project_that_declares_no_team_seats_nobody ... ok
test team::seat::tests::a_role_is_told_the_norms_before_its_own_system_prompt ... ok
test team::seat::tests::a_role_the_root_already_holds_is_reopened_not_seated_twice ... ok
test team::seat::tests::a_root_seats_every_role_as_a_child_of_its_own ... ok
test team::seat::tests::a_team_that_cannot_be_read_seats_nobody_and_fails_no_session ... ok
test team::seat::tests::it_asks_for_the_session_point_only ... ok
test team::seat::tests::nothing_happens_when_a_session_ends ... ok
test team::service::native_path_tests::a_typed_owner_accepts_a_path_json_cannot_name ... ok
test team::service::tests::a_file_that_will_not_parse_says_which_one ... ok
test team::service::tests::a_key_nobody_wrote_and_a_file_nobody_wrote_answer_alike ... ok
test team::service::tests::a_method_this_service_does_not_speak_says_which_one_it_does ... ok
test team::service::tests::params_that_name_no_directory_are_refused_rather_than_guessed ... ok
test team::service::tests::the_service_answers_one_key_as_a_person_wrote_it ... ok
test thinking::tests::a_child_is_moved_by_the_name_the_spawn_gave_back ... ok
test thinking::tests::a_name_nobody_has_is_handed_back_with_the_children ... ok
test thinking::tests::a_session_sets_its_own_level_and_is_told_when_it_lands ... ok
test thinking::tests::a_word_that_is_not_one_is_handed_back_with_the_list ... ok
test thinking::tests::it_reads_only_names_the_agent_a_rule_may_match_and_offers_seven_words ... ok
test thinking::tests::off_is_a_word_like_the_others ... ok
test thinking::tests::the_description_says_when_the_level_lands ... ok
test watch::tests::a_background_agent_that_failed_tells_its_parent_so ... ok
test watch::tests::a_cancelled_wait_stops_waiting ... ok
test watch::tests::a_child_whose_stream_ends_reports_nothing ... ok
test watch::tests::a_finished_background_agent_wakes_its_parent_without_signing_the_text ... ok
test watch::tests::a_lagged_watcher_resyncs_and_still_wakes_the_parent ... ok
test watch::tests::a_reply_that_says_nothing_still_reads_as_finished ... ok
test watch::tests::a_session_no_turn_has_ever_run_in_has_no_reply_at_all ... ok
test watch::tests::a_turn_that_did_not_complete_is_never_read_as_an_answer ... ok
test watch::tests::an_idle_child_s_last_reply_is_what_it_said_last ... ok
test watch::tests::an_idle_child_whose_last_turn_failed_remembers_that ... ok
test watch::tests::the_reply_is_the_assistant_text_of_the_turn_that_ended ... ok

test result: ok. 155 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running unittests src/lib.rs (target/debug/deps/bingo_alarm-4e1cc4d2a9fea9c6)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/contracts.rs (target/debug/deps/contracts-4dde415ec4ced585)

running 1 test
test arm_cancel_replace_reconcile_and_drop_are_owned ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/bingo_auth_oauth-189c81f9133d01fe)

running 103 tests
test callback::tests::a_callback_without_a_state_is_read_but_matches_no_nonce ... ok
test callback::tests::a_callback_yields_its_code_and_state_percent_decoded ... ok
test callback::tests::a_missing_code_or_another_path_is_invalid ... ok
test callback::tests::a_pasted_redirect_reads_the_same_as_one_the_browser_delivered ... ok
test callback::tests::a_refusal_in_the_query_is_reported_in_the_servers_own_words ... ok
test callback::tests::an_issuer_the_server_named_is_read_back_decoded ... ok
test callback::tests::the_order_of_the_query_does_not_matter_and_extra_fields_are_ignored ... ok
test challenge::tests::a_401_comes_back_as_the_challenge_it_carried ... ok
test challenge::tests::a_challenge_names_the_metadata_the_scope_and_the_error ... ok
test challenge::tests::a_challenge_that_names_nothing_is_empty_rather_than_a_failure ... ok
test challenge::tests::a_comma_inside_a_quoted_value_does_not_end_the_parameter ... ok
test challenge::tests::a_parameter_name_in_any_case_is_read_and_an_unquoted_value_too ... ok
test challenge::tests::a_server_nothing_is_listening_on_is_a_transport_failure ... ok
test challenge::tests::a_server_that_answers_asks_for_no_sign_in ... ok
test challenge::tests::only_a_401_is_a_sign_in_and_the_other_refusals_are_not ... ok
test discover::tests::a_challenge_that_names_the_document_puts_it_first ... ok
test discover::tests::a_document_that_calls_itself_something_else_is_refused ... ok
test discover::tests::a_resource_leads_to_its_server_and_the_audience_it_names ... ok
test discover::tests::a_resource_with_no_metadata_anywhere_says_what_it_tried ... ok
test discover::tests::a_server_at_the_root_has_one_well_known_url_and_not_two ... ok
test discover::tests::a_server_without_s256_is_refused_in_words ... ok
test discover::tests::a_url_splits_into_its_origin_and_its_path ... ok
test discover::tests::the_audience_is_the_url_unless_the_resource_names_one_that_covers_it ... ok
test discover::tests::the_resource_ladder_inserts_the_path_after_the_well_known_segment ... ok
test discover::tests::the_scope_the_challenge_named_wins_over_the_advertised_list ... ok
test discover::tests::the_server_ladder_is_the_specifications_order ... ok
test error::tests::a_permanent_body_classifies_as_expired_whatever_the_status ... ok
test error::tests::the_three_retired_refresh_tokens_are_permanent_and_nothing_else_is ... ok
test http::tests::deadline_bounds_a_codex_token_refresh ... ok
test http::tests::deadline_bounds_mcp_auth_discovery ... ok
test issuer::tests::a_discovered_issuer_names_its_resource_and_omits_a_scope_nobody_gave ... ok
test issuer::tests::an_absolute_endpoint_is_the_endpoint_and_the_base_is_not_prefixed ... ok
test issuer::tests::every_path_hangs_off_the_base_without_a_doubled_slash ... ok
test issuer::tests::the_authorize_url_is_the_one_the_issuer_expects ... ok
test jwt::tests::a_token_that_is_not_a_jwt_yields_no_account ... ok
test jwt::tests::a_top_level_claim_is_still_read ... ok
test jwt::tests::an_email_claim_is_read_and_a_missing_one_is_none ... ok
test jwt::tests::the_account_id_comes_from_the_namespaced_claim ... ok
test jwt::tests::the_first_organization_is_the_last_resort ... ok
test jwt::tests::the_namespaced_claim_wins_over_a_top_level_one ... ok
test key::tests::a_browser_or_a_device_says_paste_is_the_way ... ok
test key::tests::a_neighbours_entry_is_not_this_providers_key ... ok
test key::tests::a_pasted_key_lands_in_the_store_and_a_logout_takes_it_out ... ok
test key::tests::a_token_set_under_a_key_providers_name_is_not_a_key ... ok
test key::tests::an_empty_paste_and_a_refused_dialog_store_nothing ... ok
test key::tests::the_hint_names_every_place_this_provider_reads ... ok
test key::tests::the_store_is_read_before_what_was_configured ... ok
test mcp::tests::a_configured_client_id_is_used_and_nothing_is_registered ... ok
test mcp::tests::a_forged_state_or_a_foreign_issuer_is_refused_before_the_code_is_redeemed ... ok
test mcp::tests::a_pasted_redirect_registers_signs_in_and_stores_the_one_entry ... ok
test mcp::tests::a_refresh_token_the_issuer_retired_leaves_the_entry_gone_and_expired ... ok
test mcp::tests::a_server_that_registers_nobody_and_was_given_no_client_id_says_which_key ... ok
test mcp::tests::a_stale_token_is_renewed_once_however_many_callers_ask ... ok
test mcp::tests::a_token_with_no_lifetime_is_used_until_the_server_refuses_it ... ok
test mcp::tests::an_mcp_server_has_no_device_flow_and_says_so ... ok
test mcp::tests::nothing_of_the_credential_reaches_a_debug_line ... ok
test mcp::tests::signing_out_revokes_and_forgets ... ok
test mcp::tests::the_token_request_carries_the_resource_and_the_verifier ... ok
test percent::tests::a_malformed_escape_is_left_as_written ... ok
test percent::tests::a_redirect_uri_survives_a_round_trip ... ok
test percent::tests::a_scope_encodes_its_spaces ... ok
test pkce::tests::a_state_nonce_is_22_characters_and_never_repeats ... ok
test pkce::tests::a_verifier_is_43_url_safe_characters_and_never_repeats ... ok
test pkce::tests::the_challenge_matches_the_rfc_test_vector ... ok
test redirect::tests::a_redirect_without_a_code_is_refused_the_same_way ... ok
test redirect::tests::a_registration_names_both_spellings_of_the_one_socket ... ok
test redirect::tests::a_wrong_state_is_refused_and_the_browser_is_told ... ok
test redirect::tests::the_named_port_is_taken_when_it_is_free_and_given_up_when_it_is_not ... ok
test redirect::tests::the_redirect_uri_names_the_port_that_was_bound ... ok
test redirect::tests::the_right_state_yields_the_code_and_a_page_that_says_so ... ok
test register::tests::a_reply_yields_the_id_and_the_secret_only_when_there_is_one ... ok
test register::tests::a_server_that_refuses_says_so_with_its_status ... ok
test register::tests::a_server_that_registers_us_answers_with_the_name_it_gave ... ok
test register::tests::the_request_is_the_native_public_client_rfc_7591_describes ... ok
test source::tests::a_cancelled_device_login_stops_polling ... ok
test source::tests::a_credential_with_no_refresh_token_expires_rather_than_being_sent ... ok
test source::tests::a_forced_refresh_renews_a_token_that_still_looked_fresh ... ok
test source::tests::a_pasted_credential_is_stored_as_a_key_and_never_refreshed ... ok
test source::tests::a_refresh_that_omits_the_refresh_token_keeps_the_stored_one ... ok
test source::tests::a_retired_refresh_token_clears_the_entry_and_reads_as_expired ... ok
test source::tests::an_empty_paste_is_refused_and_a_cancelled_one_is_cancelled ... ok
test source::tests::an_empty_store_reads_as_signed_out_and_a_login_elsewhere_is_seen ... ok
test source::tests::eight_concurrent_callers_make_one_refresh ... ok
test source::tests::signing_out_revokes_best_effort_and_empties_the_entry ... ok
test source::tests::the_browser_flow_checks_the_state_and_exchanges_with_its_verifier ... ok
test source::tests::the_device_flow_polls_until_it_is_granted_and_exchanges_the_code ... ok
test store::tests::a_corrupt_file_is_an_error_rather_than_a_silent_emptying ... ok
test store::tests::a_missing_file_reads_as_an_empty_store ... ok
test store::tests::an_account_id_nobody_knows_is_left_out_of_the_file ... ok
test store::tests::an_entry_round_trips_and_its_neighbours_are_untouched ... ok
test store::tests::an_mcp_entry_is_written_under_its_server_in_the_shape_the_adr_names ... ok
test store::tests::the_file_is_private_after_the_first_write_and_after_a_rewrite ... ok
test store::tests::the_serialised_shape_is_the_one_the_adr_names ... ok
test store::tests::what_the_server_never_gave_is_absent_rather_than_empty ... ok
test tokens::tests::a_lifetime_becomes_an_instant_and_the_reply_fills_every_field ... ok
test tokens::tests::a_receipt_reads_the_email_out_of_the_id_token ... ok
test tokens::tests::a_refresh_reply_that_omits_a_field_keeps_the_previous_one ... ok
test tokens::tests::a_rotated_refresh_token_replaces_the_previous_one ... ok
test tokens::tests::an_account_absent_from_the_reply_comes_from_the_id_token_then_the_access_token ... ok
test tokens::tests::an_entry_round_trips_and_an_api_key_is_not_a_token_set ... ok
test tokens::tests::an_entry_with_no_expiry_or_refresh_token_reads_back_as_unknown ... ok
test tokens::tests::debug_never_prints_a_secret ... ok
test tokens::tests::freshness_leads_the_expiry_by_five_minutes_and_no_expiry_is_never_fresh ... ok

test result: ok. 103 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.79s

     Running unittests src/lib.rs (target/debug/deps/bingo_channel_adapter-1c10ad6e952061ea)

running 33 tests
test conversation::tests::a_thread_extends_its_chats_path_and_a_chat_alone_is_the_path ... ok
test limits::tests::a_clip_never_exceeds_the_limit_in_its_own_unit ... ok
test limits::tests::a_long_label_is_cut_with_an_ellipsis ... ok
test limits::tests::a_message_that_fits_is_not_copied ... ok
test limits::tests::a_plain_dialect_drops_the_fences_and_keeps_the_code ... ok
test limits::tests::each_encoding_counts_the_same_string_differently ... ok
test loopback::tests::a_click_carries_the_question_and_the_key_the_button_held ... ok
test loopback::tests::a_group_is_addressed_only_on_a_mention ... ok
test loopback::tests::a_line_that_is_not_an_event_is_ignored_rather_than_guessed_at ... ok
test loopback::tests::a_mechanism_that_is_off_hands_over_nothing ... ok
test loopback::tests::a_sign_comes_off_only_with_the_mark_that_put_it_up ... ok
test loopback::tests::every_call_is_recorded_in_order_with_the_ids_it_minted ... ok
test loopback::tests::the_spoken_wire_is_the_one_the_black_box_tests_read ... ok
test platform::tests::a_platform_refusal_keeps_the_platforms_words ... ok
test platform::tests::an_unknown_flag_keeps_the_existing_refusal ... ok
test platform::tests::an_unknown_settings_slice_names_the_missing_platform ... ok
test question::tests::a_button_label_is_cut_to_the_platforms_length ... ok
test question::tests::a_chosen_option_is_named_by_the_label_the_chat_showed ... ok
test question::tests::a_form_is_one_ladder_per_question_and_the_last_settles_them_all ... ok
test question::tests::a_form_the_kernel_will_not_take_a_form_for_is_not_asked ... ok
test question::tests::a_login_has_no_rung_in_a_chat ... ok
test question::tests::a_lone_question_is_settled_by_its_own_reply ... ok
test question::tests::a_numbered_list_says_how_to_answer_it ... ok
test question::tests::a_permission_offers_the_three_rungs_and_numbers_them ... ok
test question::tests::a_session_rule_nobody_offered_is_not_shown_as_if_it_would_be_installed ... ok
test question::tests::a_withdrawn_question_says_why ... ok
test question::tests::an_outcome_names_the_decision_and_where_it_was_made ... ok
test question::tests::free_text_is_accepted_where_the_spec_allows_it ... ok
test question::tests::more_choices_than_the_platform_draws_drops_the_whole_question_to_the_lower_rung ... ok
test question::tests::the_same_key_answers_on_either_rung ... ok
test secret::tests::a_secret_is_the_store_s_when_the_environment_has_none ... ok
test secret::tests::only_named_platforms_are_configured_and_only_those_with_secrets_sign ... ok
test secret::tests::the_store_is_keyed_so_a_channel_can_never_collide_with_a_provider ... ok

test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running unittests src/lib.rs (target/debug/deps/bingo_channel_feishu-4fd35b87cafca214)

running 120 tests
test feishu::api::tests::a_call_carries_the_bearer_and_hands_back_the_whole_body ... ok
test feishu::api::tests::a_refusal_keeps_its_code_so_the_caller_can_decide ... ok
test feishu::api::tests::a_refusal_that_is_not_a_rate_limit_is_not_shrugged_at ... ok
test feishu::api::tests::the_endpoint_is_asked_for_without_a_bearer ... ok
test feishu::attachments::tests::a_file_over_the_cap_is_named_and_not_kept ... ok
test feishu::attachments::tests::a_hostile_name_cannot_leave_the_message_it_arrived_in ... ok
test feishu::attachments::tests::a_name_is_cut_to_fit_and_keeps_what_says_what_it_is ... ok
test feishu::attachments::tests::a_refused_fetch_costs_the_attachment_and_not_the_words ... ok
test feishu::attachments::tests::a_small_text_file_is_read_out_and_a_bigger_one_is_only_pointed_at ... ok
test feishu::attachments::tests::a_type_the_table_refuses_arrives_as_png ... ok
test feishu::attachments::tests::a_voice_note_that_is_not_served_as_audio_is_served_as_a_file ... ok
test feishu::attachments::tests::bytes_no_decoder_reads_are_dropped_whatever_the_header_says ... ok
test feishu::attachments::tests::one_of_each_kind_lands_where_the_words_say_it_did ... ok
test feishu::attachments::tests::pictures_are_fetched_in_order_and_a_refused_one_is_dropped ... ok
test feishu::attachments::tests::what_a_fortnight_has_passed_over_is_swept_as_the_next_file_lands ... ok
test feishu::bootstrap::tests::a_forbidden_or_limit_bound_handshake_is_fatal_and_the_rest_are_worth_retrying ... ok
test feishu::bootstrap::tests::a_pong_hot_updates_only_what_it_carries ... ok
test feishu::bootstrap::tests::a_refusal_carries_the_peers_own_words ... ok
test feishu::bootstrap::tests::an_answer_without_a_url_is_a_transport_error_not_a_panic ... ok
test feishu::bootstrap::tests::the_endpoint_answer_is_a_url_and_the_peers_intervals ... ok
test feishu::bootstrap::tests::the_first_reconnect_is_jittered_and_the_rest_are_flat ... ok
test feishu::bootstrap::tests::the_request_is_the_pascal_case_one_the_endpoint_accepts ... ok
test feishu::card::tests::a_question_that_will_not_fit_in_buttons_is_a_card_with_none ... ok
test feishu::card::tests::a_streaming_card_is_created_with_its_element_already_named ... ok
test feishu::card::tests::the_card_bodies_are_pinned ... ok
test feishu::chunks::tests::a_message_of_one_part_is_whole_already ... ok
test feishu::chunks::tests::a_part_nobody_finished_is_dropped_after_five_seconds ... ok
test feishu::chunks::tests::a_part_past_the_end_of_its_message_is_dropped_rather_than_resizing_it ... ok
test feishu::chunks::tests::parts_are_joined_in_seq_order_however_they_arrive ... ok
test feishu::chunks::tests::two_messages_in_flight_do_not_mix ... ok
test feishu::content::tests::a_card_is_what_it_calls_itself_what_it_says_and_what_can_be_pressed ... ok
test feishu::content::tests::a_card_that_is_a_whole_dashboard_is_read_down_to_its_top ... ok
test feishu::content::tests::a_card_with_nothing_readable_in_it_still_says_that_a_card_arrived ... ok
test feishu::content::tests::a_chat_with_ten_mentions_reads_every_one_of_them ... ok
test feishu::content::tests::a_code_block_full_of_fences_is_fenced_wider ... ok
test feishu::content::tests::a_file_an_audio_and_a_media_message_are_each_one_resource ... ok
test feishu::content::tests::a_merged_forward_is_its_title_until_the_bundle_is_fetched ... ok
test feishu::content::tests::a_picture_is_a_message_with_no_words_and_one_key_to_fetch ... ok
test feishu::content::tests::a_post_carries_every_run_a_post_can_hold ... ok
test feishu::content::tests::a_post_keyed_by_locale_is_still_a_post ... ok
test feishu::content::tests::a_shared_chat_names_the_chat_by_whichever_half_it_carries ... ok
test feishu::content::tests::a_sticker_and_a_type_this_build_has_never_heard_of_both_arrive ... ok
test feishu::content::tests::a_text_message_is_its_words_with_the_mentions_read_as_a_person_read_them ... ok
test feishu::content::tests::content_that_is_not_json_is_still_what_somebody_typed ... ok
test feishu::content::tests::everyone_is_named_even_though_the_mentions_never_list_them ... ok
test feishu::event::tests::a_button_carries_back_the_question_the_choice_and_where_it_was_asked ... ok
test feishu::event::tests::a_callback_from_someone_elses_card_is_not_an_answer ... ok
test feishu::event::tests::a_direct_message_is_always_addressed_to_the_bot ... ok
test feishu::event::tests::a_group_is_addressed_only_when_the_mentions_name_this_bot ... ok
test feishu::event::tests::a_merged_forward_names_the_message_whose_bundle_is_still_to_be_read ... ok
test feishu::event::tests::a_picture_is_a_message_with_no_words_and_one_key_to_fetch ... ok
test feishu::event::tests::a_redelivered_event_is_handled_once_and_the_ring_stays_bounded ... ok
test feishu::event::tests::a_topic_thread_is_a_group_with_a_thread_of_its_own ... ok
test feishu::event::tests::a_voice_note_is_a_message_with_one_resource_to_fetch ... ok
test feishu::event::tests::an_event_this_build_has_no_use_for_is_still_an_event ... ok
test feishu::event::tests::an_ordinary_message_forwards_nothing ... ok
test feishu::footer::tests::a_completed_turn_shows_its_model_effort_usage_and_context ... ok
test feishu::footer::tests::a_fence_closes_only_with_its_own_marker_and_enough_characters ... ok
test feishu::footer::tests::a_live_turn_does_not_use_the_previous_turns_usage ... ok
test feishu::footer::tests::a_long_answer_keeps_the_footer_inside_feishus_text_limit ... ok
test feishu::footer::tests::a_partial_answer_is_closed_even_without_clipping ... ok
test feishu::footer::tests::clipping_through_fence_delimiters_never_encloses_the_footer ... ok
test feishu::footer::tests::complete_fences_are_unchanged_and_clipped_fences_are_closed ... ok
test feishu::footer::tests::footer_labels_are_english_whatever_the_answer_language_or_code ... ok
test feishu::footer::tests::the_footer_stays_outside_a_valid_fenced_answer_at_the_limit ... ok
test feishu::frame::tests::a_data_frame_encodes_to_the_documented_bytes ... ok
test feishu::frame::tests::a_field_this_build_does_not_know_is_stepped_over ... ok
test feishu::frame::tests::a_header_is_read_by_name_and_as_a_number_when_it_is_one ... ok
test feishu::frame::tests::a_truncated_frame_is_an_error_rather_than_a_guess ... ok
test feishu::frame::tests::an_ack_echoes_the_frame_with_the_latency_and_the_payload_the_peer_wants ... ok
test feishu::frame::tests::every_frame_round_trips_through_the_wire ... ok
test feishu::frame::tests::setting_a_header_twice_leaves_one_of_it ... ok
test feishu::merged::tests::a_bundle_bigger_than_anybody_meant_to_forward_is_read_down_to_its_top ... ok
test feishu::merged::tests::a_bundle_is_read_by_the_same_normaliser_as_a_message_of_its_own ... ok
test feishu::merged::tests::a_bundle_no_scope_allows_leaves_the_title_to_speak_for_it ... ok
test feishu::merged::tests::a_bundle_of_nothing_adds_nothing ... ok
test feishu::posted::tests::both_kinds_round_trip_and_nothing_else_parses ... ok
test feishu::send::tests::two_messages_in_one_chat_are_spaced_and_two_chats_are_not ... ok
test feishu::tests::a_caption_follows_the_file_as_its_own_message ... ok
test feishu::tests::a_card_cannot_be_reacted_to_because_it_is_not_a_message ... ok
test feishu::tests::a_card_cannot_be_settled_and_a_message_cannot_be_streamed_into ... ok
test feishu::tests::a_card_that_has_streamed_a_while_is_renewed_before_it_closes ... ok
test feishu::tests::a_card_that_timed_out_is_opened_again_too ... ok
test feishu::tests::a_card_the_platform_closed_is_opened_again_rather_than_abandoned ... ok
test feishu::tests::a_channel_with_no_credential_refuses_before_it_dials ... ok
test feishu::tests::a_failed_turn_leaves_a_cross_where_the_sign_was ... ok
test feishu::tests::a_file_goes_up_by_its_type_and_the_message_carries_the_key ... ok
test feishu::tests::a_file_hangs_under_the_message_a_reply_would ... ok
test feishu::tests::a_picture_goes_to_the_image_endpoint_whatever_it_is_called ... ok
test feishu::tests::a_plain_message_goes_to_the_chat_as_text ... ok
test feishu::tests::a_question_is_its_own_card_and_settling_it_edits_that_message ... ok
test feishu::tests::a_rate_limited_frame_is_dropped_and_the_stream_carries_on ... ok
test feishu::tests::a_reply_hangs_under_the_message_that_started_it ... ok
test feishu::tests::a_streamed_answer_is_a_card_entity_sent_by_id_and_written_in_full ... ok
test feishu::tests::a_working_message_gets_a_typing_reaction_and_loses_it_at_the_end ... ok
test feishu::tests::an_upload_that_keeps_no_key_is_a_refusal ... ok
test feishu::tests::finishing_recovers_a_closed_stream ... mbx[progress]: 210s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test feishu::tests::finishing_recovers_a_streaming_timeout ... ok
test feishu::tests::finishing_renews_an_old_card_before_its_last_write ... ok
test feishu::tests::terminal_controls_can_be_extended_from_feishu_settings ... ok
test feishu::tests::terminal_controls_have_feishu_command_defaults ... ok
test feishu::tests::the_bootstrap_body_is_the_pascal_case_one ... ok
test feishu::tests::the_bot_asks_who_it_is_before_it_listens ... ok
test feishu::tests::the_capabilities_are_the_ones_this_platform_really_has ... ok
test feishu::token::tests::a_fresh_token_is_used_without_asking_the_issuer_again ... ok
test feishu::token::tests::a_token_inside_the_renewal_margin_is_renewed ... ok
test feishu::token::tests::eight_callers_at_once_make_one_request ... ok
test feishu::token::tests::the_debug_shape_never_carries_the_secret ... ok
test feishu::upload::tests::a_picture_is_recognised_by_its_first_bytes_in_every_format_the_endpoint_takes ... ok
test feishu::upload::tests::every_route_is_the_one_feishus_own_table_names ... ok
test feishu::upload::tests::the_body_is_rfc_7578_to_the_byte ... ok
test feishu::upload::tests::the_bytes_go_through_untouched_and_a_hostile_name_cannot_forge_a_header ... ok
test feishu::ws::tests::a_cancelled_listener_stops_without_dialling_again ... ok
test feishu::ws::tests::a_forbidden_handshake_stops_the_ladder_rather_than_hammering_it ... ok
test feishu::ws::tests::a_killed_socket_is_dialled_again_and_the_events_keep_arriving ... ok
test feishu::ws::tests::a_ping_is_answered_with_a_pong_and_a_card_frame_is_dropped ... ok
test feishu::ws::tests::a_pong_hot_updates_the_intervals_the_loop_keeps ... ok
test feishu::ws::tests::an_attachment_arrives_as_a_path_under_the_words ... ok
test feishu::ws::tests::an_event_is_acked_once_and_a_redelivery_is_acked_but_not_repeated ... ok
test feishu::ws::tests::the_connection_limit_is_fatal_however_many_times_it_is_tried ... ok

test result: ok. 120 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.99s

     Running unittests src/lib.rs (target/debug/deps/bingo_channels-1f2c4de3ab63f706)

running 104 tests
test access::tests::a_chats_own_rule_replaces_the_one_for_groups_whole ... ok
test access::tests::a_rule_that_wants_no_mention_engages_a_group_on_every_message ... ok
test access::tests::an_admin_passes_every_policy_but_off ... ok
test access::tests::every_policy_admits_the_same_people_wherever_it_is_applied ... ok
test access::tests::the_default_is_open_with_a_group_engaging_on_a_mention ... ok
test access::tests::the_settings_spelling_is_lowercase_words_and_every_field_defaults ... ok
test deliver::tests::a_cancelled_question_takes_its_buttons_with_it ... ok
test deliver::tests::a_command_result_is_posted_for_this_chats_own_write ... ok
test deliver::tests::a_command_view_is_folded_into_the_words_a_chat_can_carry ... ok
test deliver::tests::a_failed_turn_finalizes_what_there_was_and_then_says_why ... ok
test deliver::tests::a_large_memory_table_fits_a_byte_budget ... ok
test deliver::tests::a_login_has_no_rung_so_the_chat_says_where_to_answer_it ... ok
test deliver::tests::a_message_and_a_view_are_two_blocks ... ok
test deliver::tests::a_prompts_own_ack_says_nothing_because_the_turn_says_it ... ok
test deliver::tests::a_question_finalizes_the_stream_and_carries_both_rungs ... ok
test deliver::tests::a_question_this_chat_never_showed_settles_nothing ... ok
test deliver::tests::a_queued_command_keeps_ownership_until_its_terminal_ack ... ok
test deliver::tests::a_resolution_anywhere_settles_the_question_this_chat_showed ... ok
test deliver::tests::a_resolution_in_this_chat_does_not_claim_to_have_happened_elsewhere ... ok
test deliver::tests::a_shell_line_the_person_ran_lands_beside_the_answer ... ok
test deliver::tests::a_turn_with_nothing_to_say_still_says_it_ended ... ok
test deliver::tests::a_view_table_is_a_table_to_a_chat_that_draws_markdown ... ok
test deliver::tests::a_warning_is_a_status_beside_the_answer_and_a_note_is_not ... ok
test deliver::tests::a_write_this_chat_never_made_is_not_reported_here ... ok
test deliver::tests::an_interrupted_turn_ends_without_having_failed ... ok
test deliver::tests::command_results_obey_the_platform_length_and_dialect ... ok
test deliver::tests::enough_new_characters_open_the_gate_without_a_boundary ... ok
test deliver::tests::finishing_a_stream_drains_pending_words_once ... ok
test deliver::tests::only_the_newest_snapshot_is_pending_and_the_older_ones_are_forgotten ... ok
test deliver::tests::resetting_a_delivery_forgets_its_previous_session ... ok
test deliver::tests::several_assistant_items_in_one_turn_read_as_one_answer ... ok
test deliver::tests::text_after_a_question_opens_a_new_message_and_repeats_nothing ... ok
test deliver::tests::the_first_words_open_a_message_and_the_gate_holds_the_rest ... ok
test deliver::tests::the_pending_snapshot_never_overwrites_the_final_text ... ok
test deliver::tests::the_platforms_dialect_and_length_are_applied_once_here ... ok
test deliver::tests::the_timer_is_only_armed_while_something_is_held ... ok
test deliver::tests::the_timer_shows_what_the_other_two_gates_held ... ok
test directory::tests::a_session_that_sat_down_is_found_again_and_one_that_left_is_not ... ok
test directory::tests::the_directory_serves_only_while_the_guard_lives ... ok
test directory::tests::the_parent_is_the_seats_and_a_session_with_no_seat_keeps_none ... ok
test gate::tests::a_sentence_boundary_opens_the_gate_at_once ... ok
test gate::tests::an_ascii_full_stop_is_not_a_boundary ... ok
test gate::tests::enough_new_characters_open_it_without_a_boundary ... ok
test gate::tests::the_timer_opens_it_when_nothing_else_does ... ok
test guide::tests::the_page_is_one_line_of_description_and_a_body_worth_reading ... ok
test guide::tests::the_page_names_every_adapter_this_crate_has ... ok
test guide::tests::the_page_names_every_settings_key_and_word_this_plugin_claims ... ok
test guide::tests::the_page_names_the_local_platform_switch ... ok
test guide::tests::the_page_names_the_top_level_key_and_the_tool_this_plugin_registers ... ok
test guide::tests::the_page_says_how_a_chat_is_run_from_a_shell ... ok
test guide::tests::the_settings_example_is_toml_and_no_json_is_left ... ok
test host::tests::a_command_card_fallback_keeps_the_platform_budget_and_dialect ... ok
test host::tests::a_form_is_asked_one_message_at_a_time_and_answered_once ... ok
test host::tests::a_group_that_did_not_address_the_bot_opens_nothing ... ok
test host::tests::a_message_opens_a_session_keyed_by_its_chat_and_carries_who_spoke ... ok
test host::tests::a_new_session_retires_open_questions_before_switching ... ok
test host::tests::a_picture_is_submitted_beside_its_words ... ok
test host::tests::a_platform_that_cannot_stream_says_it_is_typing_instead ... ok
test host::tests::a_platform_that_streams_needs_no_typing_affordance ... ok
test host::tests::a_principal_the_policy_refuses_opens_nothing_and_hears_nothing ... ok
test host::tests::a_question_becomes_buttons_and_a_click_answers_it ... ok
test host::tests::a_refused_finish_still_asks_the_question_and_says_the_answer_whole ... ok
test host::tests::a_resolution_at_another_surface_edits_the_card_this_chat_showed ... ok
test host::tests::a_second_message_continues_the_same_session ... ok
test host::tests::a_second_surface_on_one_credential_refuses_loudly ... ok
test host::tests::a_sign_goes_up_on_the_message_that_spoke_and_comes_off_at_the_end ... ok
test host::tests::a_sign_that_would_not_go_up_costs_no_part_of_the_answer ... ok
test host::tests::a_thread_is_a_session_of_its_own ... ok
test host::tests::a_turn_that_failed_takes_the_sign_off_as_a_failure ... ok
test host::tests::an_answer_streams_into_one_message_and_is_finished_there ... ok
test host::tests::controls::a_new_session_notice_replies_in_the_originating_thread ... ok
test host::tests::controls::a_new_session_preserves_the_answer_without_editing ... ok
test host::tests::controls::a_new_session_preserves_the_previous_streamed_answer ... ok
test host::tests::refused_buttons_ask_in_words_rather_than_losing_the_question ... ok
test host::tests::stopping_keeps_the_streamed_answer_and_posts_a_notice_separately ... ok
test host::tests::with_nothing_to_edit_the_outcome_is_said_in_a_message_of_its_own ... ok
test host::tests::without_an_edit_the_answer_arrives_whole_and_once ... ok
test host::tests::without_buttons_the_numbered_rung_is_drawn_and_a_reply_answers_it ... ok
test lock::tests::a_claim_is_given_back_when_it_is_dropped ... ok
test lock::tests::a_credential_with_a_path_in_it_cannot_escape_the_directory ... ok
test lock::tests::a_different_credential_is_a_different_claim ... ok
test lock::tests::legacy_files_are_refused_with_a_remedy_and_never_changed ... ok
test lock::tests::the_second_claim_on_one_credential_is_refused_loudly ... ok
test plugin_tests::a_surface_with_no_adapter_refuses_rather_than_going_quiet ... ok
test plugin_tests::registration_refuses_unknown_and_platform_rejected_slices ... ok
test plugin_tests::the_plugin_contributes_its_page_with_no_adapter_configured ... ok
test plugin_tests::the_plugin_contributes_the_file_tool_through_a_source ... ok
test plugin_tests::the_plugin_registers_one_concurrent_surface ... ok
test runner::tests::a_deleted_child_leaves_the_chat_fold ... ok
test runner::voice_tests::an_incoming_message_is_said_by_its_sender ... ok
test settings::tests::a_flag_becomes_the_layer_the_plugin_would_have_read_from_a_file ... ok
test settings::tests::a_mechanism_can_be_turned_off_from_the_settings ... ok
test settings::tests::a_named_adapter_is_built_with_its_defaults ... ok
test settings::tests::a_policy_is_read_per_adapter_and_an_unnamed_channel_has_none ... ok
test settings::tests::an_adapter_that_says_nothing_about_access_runs_todays_rule ... ok
test settings::tests::nothing_configured_is_no_adapters_and_the_default_gate ... ok
test settings::tests::the_gate_is_settable_because_a_test_should_not_have_to_wait ... ok
test tool::tests::a_chat_with_no_way_to_carry_a_file_says_so ... ok
test tool::tests::a_file_hangs_under_whatever_the_answer_would ... ok
test tool::tests::a_path_that_does_not_read_is_named ... ok
test tool::tests::a_session_that_is_not_a_chat_is_refused_in_words ... ok
test tool::tests::the_file_goes_to_the_chat_this_session_is_in ... ok
test tool::tests::the_source_answers_nothing_until_the_surface_is_running ... ok
test tool::tests::the_subject_is_the_resolved_path_so_a_rule_can_name_it ... ok

test result: ok. 104 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.60s

     Running unittests src/lib.rs (target/debug/deps/bingo_checkpoints-c813d0e2a8472d05)

running 42 tests
test command::tests::a_bare_rewind_lists_the_turns_newest_first_with_what_each_touched ... ok
test command::tests::a_restore_that_cannot_be_read_leaves_the_journal_alone ... ok
test command::tests::a_turn_that_touched_no_file_says_so_rather_than_nothing ... ok
test command::tests::a_turn_this_session_never_had_moves_nothing ... ok
test command::tests::going_back_puts_the_files_back_and_then_the_conversation ... ok
test command::tests::going_back_to_the_last_turn_leaves_the_ones_before_it_alone ... ok
test command::tests::the_reply_says_what_moved_and_what_did_not ... ok
test command::tests::the_spec_is_the_one_the_pickers_chord_looks_for ... ok
test guide::tests::the_page_is_one_line_of_description_and_a_body_worth_reading ... ok
test guide::tests::the_page_names_every_command_this_plugin_registers ... ok
test guide::tests::the_page_names_every_tool_whose_writes_are_kept ... ok
test guide::tests::the_page_says_how_big_a_file_may_be_and_still_be_kept ... ok
test guide::tests::this_plugin_claims_no_settings_for_the_page_to_name ... ok
test hook::tests::a_call_outside_a_turn_is_not_a_turns_snapshot ... ok
test hook::tests::a_call_whose_field_is_missing_or_is_not_a_string_names_no_file ... ok
test hook::tests::a_file_the_turn_creates_is_kept_as_absent ... ok
test hook::tests::a_relative_path_hangs_off_the_sessions_own_directory ... ok
test hook::tests::a_tool_the_table_does_not_name_writes_nothing_this_can_keep ... ok
test hook::tests::a_write_and_an_edit_of_one_file_in_one_turn_keep_it_once ... ok
test plugin_tests::a_host_that_cannot_say_collects_nothing ... ok
test plugin_tests::a_host_that_lists_no_session_has_no_checkpoint_left_to_keep ... ok
test plugin_tests::a_plugin_that_never_registered_starts_without_a_store ... ok
test plugin_tests::registering_contributes_a_hook_a_command_and_a_page_and_creates_nothing ... ok
test plugin_tests::registering_twice_is_refused_rather_than_silently_forgotten ... ok
test plugin_tests::starting_collects_the_checkpoints_of_a_session_that_is_no_longer_there ... ok
test plugin_tests::the_manifest_says_what_it_provides_and_claims_no_settings ... ok
test restore::tests::a_plan_reads_the_bytes_of_the_turn_that_kept_them ... ok
test restore::tests::a_snapshot_whose_bytes_are_gone_refuses_the_whole_plan ... ok
test restore::tests::no_turns_is_an_empty_plan_and_not_an_error ... ok
test restore::tests::the_oldest_snapshot_of_a_file_is_the_one_that_wins ... ok
test store::tests::a_directory_where_a_file_was_asked_for_is_skipped_not_copied ... ok
test store::tests::a_file_over_the_cap_is_recorded_and_not_copied ... ok
test store::tests::a_file_that_is_not_there_yet_is_recorded_as_absent ... ok
test store::tests::a_line_nothing_can_read_is_skipped_and_the_rest_stand ... ok
test store::tests::a_path_that_is_not_utf_8_is_not_kept_rather_than_kept_wrongly ... ok
test store::tests::a_session_that_is_gone_takes_its_checkpoints_with_it ... ok
test store::tests::an_index_line_keeps_a_path_with_spaces_in_it ... ok
test store::tests::forgetting_a_session_that_kept_nothing_is_not_a_failure ... ok
test store::tests::the_first_snapshot_of_a_turn_wins_and_the_second_is_not_taken ... ok
test turns::tests::a_turn_nobody_opened_by_typing_is_named_by_its_own_id ... ok
test turns::tests::going_back_to_a_turn_undoes_it_and_every_later_one ... ok
test turns::tests::the_turns_are_the_transcripts_own_boundaries_oldest_first ... ok

test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running unittests src/lib.rs (target/debug/deps/bingo_claim-31fdc1d0df6d94da)

running 8 tests
test tests::an_io_failure_is_not_contention_or_legacy ... ok
test tests::an_unlocked_modern_file_is_taken_without_rewriting_it ... ok
test tests::holder_says_another_runner_only_while_the_os_lock_is_held ... ok
test tests::initialization_publishes_only_one_complete_marker_under_contention ... ok
test tests::legacy_and_unrecognized_files_are_never_claimed_or_modified ... ok
test tests::probing_absent_files_creates_nothing_and_reports_no_owner ... ok
test tests::releasing_preserves_the_published_marker_and_inode ... ok
test tests::the_second_claim_is_refused_until_the_first_drops ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/bare.rs (target/debug/deps/bare-0f84204f880ed439)

running 1 test
test a_bare_lock_excludes_until_drop_and_preserves_the_file ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/bingo_collab-3cd17a456db4a996)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/contracts.rs (target/debug/deps/contracts-9da087b0e5105d84)

running 3 tests
test journal_record_shapes_stay_compatible ... ok
test names_keep_their_spelling_and_share_one_rule ... ok
test room_readers_preserve_the_columns_and_targets_from_fixtures ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/bingo_context-c45f400eea2e3368)

running 99 tests
test baseline::lifecycle::a_replayed_baseline_needs_no_private_contributor_state ... ok
test baseline::lifecycle::an_empty_instruction_capture_is_retained_when_a_file_appears ... ok
test baseline::lifecycle::baseline_retention_does_not_hide_memory_changes_from_disk_readers ... ok
test baseline::lifecycle::compacted_and_rewound_frames_refresh_only_the_next_assembly ... ok
test baseline::lifecycle::disk_changes_remain_hidden_when_compaction_does_not_advance_history ... ok
test baseline::lifecycle::session_start_invalidates_both_keys_without_opening_the_session ... ok
test baseline::lifecycle::session_start_publishes_where_the_memories_are ... ok
test baseline::tests::an_unavailable_journal_does_not_render_unrecorded_knowledge ... ok
test baseline::tests::disk_changes_do_not_rewrite_an_existing_instruction_baseline ... ok
test baseline::tests::persisted_blocks_round_trip_through_the_extension_fold ... ok
test baseline::tests::retained_blocks_never_poll_the_renderer_even_in_a_later_turn ... ok
test compact::tests::a_journal_with_nothing_old_enough_is_an_invalid_request ... ok
test compact::tests::a_provider_error_comes_back_with_its_own_code ... ok
test compact::tests::a_shortened_retry_discloses_omission_in_its_instruction_and_summary ... ok
test compact::tests::a_summary_replaces_the_old_turns_and_bills_what_it_cost ... ok
test compact::tests::an_empty_summary_falls_to_the_rung_that_needs_no_model ... ok
test compact::tests::an_overflow_under_a_tripped_breaker_asks_no_model ... ok
test compact::tests::an_overflow_under_two_failures_still_asks ... ok
test compact::tests::fresh_overflow_after_two_summary_overflows_drops_without_a_third_call ... ok
test compact::tests::fresh_overflow_without_headroom_drops_without_asking_but_other_reasons_fail ... ok
test compact::tests::manual_instructions_reach_the_system_prompt ... ok
test compact::tests::repeated_overflow_is_bounded_and_only_the_retry_loses_history ... ok
test files::tests::a_block_says_how_many_lines_it_left_out ... ok
test files::tests::a_block_that_left_nothing_out_says_nothing ... ok
test files::tests::a_byte_cap_cutting_mid_file_keeps_whole_newest_lines ... ok
test files::tests::a_file_under_both_caps_is_unchanged ... ok
test files::tests::four_hundred_lines_contribute_the_last_three_hundred ... ok
test guide::tests::the_page_is_one_line_of_description_and_a_body_worth_reading ... ok
test guide::tests::the_page_names_both_files_a_directory_may_speak_through ... ok
test guide::tests::the_page_names_every_command_this_plugin_registers ... ok
test guide::tests::the_page_names_every_type_a_memory_may_be ... ok
test guide::tests::the_page_says_what_an_index_is_called_and_what_it_costs ... ok
test guide::tests::this_plugin_claims_no_settings_for_the_page_to_name ... ok
test instructions::tests::a_directory_with_both_files_speaks_through_agents_md ... ok
test instructions::tests::a_directory_with_only_claude_md_still_speaks ... ok
test instructions::tests::a_file_that_is_not_there_contributes_nothing ... ok
test instructions::tests::a_long_file_contributes_its_newest_lines_and_says_so ... ok
test instructions::tests::every_directory_from_the_root_down_speaks_in_order ... ok
test instructions::tests::it_is_the_first_thing_in_the_system_prompt ... ok
test memory::command::tests::an_empty_memory_says_where_memories_go ... ok
test memory::command::tests::it_lists_both_scopes ... ok
test memory::command::tests::the_spec_runs_now_and_takes_nothing ... ok
test memory::dir::tests::a_directory_outside_git_is_keyed_by_its_path ... ok
test memory::dir::tests::a_name_keeps_only_what_a_file_name_may_hold ... ok
test memory::dir::tests::a_repository_is_keyed_by_the_commit_it_began_with ... ok
test memory::dir::tests::a_scope_holds_its_index_and_one_file_per_memory ... ok
test memory::dir::tests::the_two_scopes_are_two_directories_under_the_data_directory ... ok
test memory::file::tests::a_file_without_frontmatter_is_not_a_memory ... ok
test memory::file::tests::a_file_written_with_windows_line_endings_still_parses ... ok
test memory::file::tests::a_memory_printed_is_the_memory_parsed ... ok
test memory::file::tests::a_memory_whose_fact_is_its_description_has_no_body ... ok
test memory::file::tests::a_name_that_is_not_the_file_name_is_refused ... ok
test memory::file::tests::a_quoted_value_is_the_value_inside_the_quotes ... ok
test memory::file::tests::a_type_nobody_defined_is_refused_with_the_four_that_exist ... ok
test memory::file::tests::every_frontmatter_line_is_needed ... ok
test memory::file::tests::every_kind_round_trips ... ok
test memory::file::tests::the_fixture_is_the_memory_it_describes ... ok
test memory::row::tests::a_file_tool_on_a_memory_is_the_memory_verb_and_the_slug ... ok
test memory::row::tests::a_relative_path_is_read_from_where_the_session_works ... ok
test memory::row::tests::anything_else_is_what_it_was ... ok
test memory::row::tests::the_hook_places_only_memory_calls_on_the_gated_item ... ok
test memory::store::tests::a_directory_that_is_not_there_holds_nothing ... ok
test memory::store::tests::a_saved_memory_is_a_file_and_a_line ... ok
test memory::store::tests::a_second_memory_is_a_second_line_not_a_second_index ... ok
test memory::store::tests::nothing_a_directory_holds_but_memories_is_read_as_one ... ok
test memory::teach::tests::it_is_short_enough_to_carry_every_turn ... ok
test memory::teach::tests::it_says_the_format_and_the_rules_and_no_path ... ok
test memory::teach::tests::the_teaching_is_the_same_words_for_every_turn ... ok
test memory::tests::a_long_index_contributes_its_newest_lines_and_says_so ... ok
test memory::tests::an_empty_memory_still_says_where_the_directories_are ... ok
test memory::tests::indexes_are_retained_until_the_history_generation_changes ... ok
test memory::tests::it_speaks_after_the_instructions ... ok
test memory::tests::the_prompt_carries_the_index_and_never_a_body ... ok
test prompt::tests::a_tiny_window_never_sends_an_oversized_request ... ok
test prompt::tests::overflow_never_leaves_an_orphaned_tool_result ... ok
test prompt::tests::overflow_shrinks_only_complete_exchanges_and_preserves_purpose ... ok
test prompt::tests::the_parent_prefix_is_unchanged_and_only_the_instruction_is_appended ... ok
test root::tests::a_directory_in_no_repository_is_its_own_root ... ok
test root::tests::a_directory_outside_the_root_walks_nothing ... ok
test root::tests::a_subdirectory_answers_with_the_checkout_above_it ... ok
test root::tests::a_worktree_and_its_checkout_began_with_the_same_commit ... ok
test root::tests::a_worktree_answers_with_the_checkout_it_was_made_from ... ok
test root::tests::the_chain_runs_from_the_root_down_to_the_directory ... ok
test root::tests::two_repositories_began_differently_and_an_empty_one_has_not_begun ... ok
test split::tests::a_generous_budget_still_keeps_only_the_last_twelve ... ok
test split::tests::a_heavy_tail_cuts_further_than_the_count_would ... ok
test split::tests::a_journal_that_is_one_long_round_cuts_before_it ... ok
test split::tests::a_short_journal_has_nothing_to_cut ... ok
test split::tests::the_boundary_never_shares_a_round_with_a_call_before_it ... ok
test split::tests::the_cut_moves_off_a_tool_round_instead_of_splitting_it ... ok
test stream::tests::cancellation_after_observed_usage_rejects_the_summary_but_keeps_its_cost ... ok
test stream::tests::summary_rejects_a_missing_finish_and_pre_cancelled_request ... ok
test stream::tests::summary_rejects_tools_and_length_but_retains_observed_usage ... ok
test tail::tests::a_single_element_over_budget_leaves_nothing ... ok
test tail::tests::an_empty_slice_starts_at_zero ... ok
test tail::tests::everything_fits_under_a_generous_budget ... ok
test tail::tests::the_budget_keeps_the_newest_and_drops_the_oldest ... ok
test tests::the_manifest_says_what_it_provides_and_claims_no_settings ... ok
test tests::the_plugin_registers_a_compactor_two_contributors_the_command_and_two_hooks ... ok

test result: ok. 99 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.63s

     Running unittests src/lib.rs (target/debug/deps/bingo_core-db5c8f5bb1a811c8)

running 483 tests
test accumulator::tests::a_cut_the_endpoint_made_is_a_compaction_item_with_nothing_replaced ... ok
test accumulator::tests::a_text_block_starts_streams_and_completes ... ok
test accumulator::tests::a_tool_call_becomes_a_pending_item_with_parsed_input ... ok
test accumulator::tests::an_interrupted_stream_keeps_text_and_drops_unfinished_tool_input ... ok
test accumulator::tests::invalid_tool_input_is_kept_as_a_string_and_flagged ... ok
test accumulator::tests::reasoning_metadata_lands_on_the_completed_item ... ok
test accumulator::tests::the_last_reading_of_a_round_is_the_rounds_reading ... ok
test chain::json::contracts::descriptions_and_received_text_are_rewritable ... ok
test chain::json::contracts::pinned_changes_and_tighten_answers_are_skipped_once ... ok
test chain::json::tests::completion_wraps_the_text_from_beneath ... ok
test chain::observe::tests::additive_spend_is_not_a_context_measurement ... ok
test chain::tests::a_failed_or_expired_link_never_runs_its_continuation_twice ... ok
test chain::tests::a_full_link_answers_without_the_kernel_and_receipts_its_name ... ok
test chain::tests::a_rewrite_allow_over_nexts_ask_is_skipped ... ok
test chain::tests::a_tighten_link_cannot_answer_tool_call ... ok
test chain::tests::an_external_links_deadline_excludes_its_continuation ... ok
test chain::tests::draining_a_continuation_is_not_plugin_phase_time ... ok
test chain::tests::dynamic_links_are_read_again_and_receive_only_the_user_grant ... ok
test chain::tier::tests::deny_is_stricter_than_ask_which_is_stricter_than_allow ... ok
test chain::tier::tests::each_tier_has_exactly_its_declared_rights ... ok
test chain::tool::item_tests::after_hooks_receive_each_outcomes_own_item ... ok
test commands::login::tests::the_provider_is_required_and_the_method_optional ... ok
test commands::model::tests::a_leading_provider_is_taken_only_when_it_is_one ... ok
test commands::models::tests::a_build_with_no_models_says_so ... ok
test commands::models::tests::a_provider_says_how_many_ids_it_offers_and_who_named_them ... ok
test commands::models::tests::a_refresh_answers_with_a_count_a_provider_and_a_failure_by_name ... ok
test commands::models::tests::an_age_is_coarse_and_never_negative ... ok
test commands::plugins::tests::a_bare_line_is_the_listing_and_a_verb_is_a_switch ... ok
test commands::plugins::tests::a_row_carries_the_five_columns_the_headers_promise ... ok
test commands::plugins::tests::a_word_it_does_not_know_and_a_missing_name_are_refused ... ok
test commands::plugins::tests::the_spec_says_both_verbs_and_the_catalogue_the_name_comes_from ... ok
test commands::rename::tests::a_name_is_one_trimmed_line_and_no_argument_is_a_question ... ok
test commands::rename::tests::a_session_with_no_name_is_told_where_one_comes_from ... ok
test commands::status::tests::the_context_row_reads_as_a_fraction ... ok
test commands::status::tests::the_mode_is_the_policy_s_or_default ... ok
test commands::think::tests::the_usage_line_lists_every_word_a_level_may_be_said_in ... ok
test context::budget::extreme_pixels::anchoring_a_saturated_pixel_estimate_stays_saturated ... ok
test context::budget::tests::a_held_window_has_no_lines_in_it ... ok
test context::budget::tests::a_tiny_window_never_goes_negative ... ok
test context::budget::tests::the_anchor_adds_only_what_grew_since_the_server_counted ... ok
test context::budget::tests::the_lines_are_ordered_within_the_effective_window ... ok
test context::budget::tests::the_lines_follow_the_effective_window ... ok
test context::budget::tests::used_never_drops_below_the_servers_count ... ok
test context::elide::images::tests::a_conversation_without_images_is_left_untouched ... ok
test context::elide::images::tests::a_pathless_picture_gets_the_note_without_whereabouts ... ok
test context::elide::images::tests::a_picture_leaves_on_the_answer_after_the_third_and_not_before ... ok
test context::elide::images::tests::inside_a_tool_result_the_id_the_flag_and_the_other_parts_survive ... ok
test context::elide::images::tests::only_answered_pictures_move_and_the_projection_is_idempotent ... ok
test context::elide::images::tests::the_note_names_the_type_the_size_and_where_to_read_it_again ... ok
test context::elide::images::weight_contract::six_megabyte_pictures_keep_only_the_newest_four ... ok
test context::elide::tests::nothing_to_elide_is_no_clone ... ok
test context::elide::tests::only_old_large_results_are_elided_and_nothing_else_moves ... ok
test context::elide::tests::the_projection_keeps_ids_and_the_tail_and_is_idempotent ... ok
test context::tests::a_call_the_model_never_made_never_reaches_it ... ok
test context::tests::a_compaction_with_no_summary_says_nothing_to_the_model ... ok
test context::tests::a_contributors_piece_sharing_the_turn_does_the_same ... ok
test context::tests::a_nudge_sharing_the_turn_makes_the_persons_line_say_it_is_theirs ... ok
test context::tests::a_peer_sharing_the_turn_makes_the_persons_line_say_it_is_theirs ... ok
test context::tests::a_replayed_journal_folds_exactly_like_the_live_session ... ok
test context::tests::a_shell_line_reaches_the_model_as_the_line_and_its_output ... ok
test context::tests::a_tool_round_folds_into_assistant_then_user_with_results_first ... ok
test context::tests::a_turn_the_person_never_spoke_in_is_marked_nowhere ... ok
test context::tests::a_user_item_from_a_named_principal_says_who_spoke ... ok
test context::tests::a_user_item_from_a_room_says_where_it_came_from ... ok
test context::tests::actions_with_results_reach_the_model_as_notes ... ok
test context::tests::appending_another_source_never_relabels_the_existing_fold ... ok
test context::tests::compaction_replaces_the_head_and_keeps_what_it_says ... ok
test context::tests::empty_assistant_text_never_becomes_a_message ... ok
test context::tests::every_persons_line_keeps_its_own_attribution ... ok
test context::tests::every_projection_is_legal_for_the_api ... ok
test context::tests::every_tool_use_gets_a_result_even_without_output ... ok
test context::tests::reasoning_with_only_replay_data_still_goes_back_to_the_provider ... ok
test context::tests::retry_withdraws_partial_text_from_the_journal_view ... ok
test context::tests::rewind_drops_items ... ok
test context::tests::the_estimate_counts_cjk_per_character_and_images_flat ... ok
test context::tests::the_kernel_and_its_hooks_speak_unsigned_and_are_not_the_person ... ok
test context::tests::transcript_model_matches_frame_oracle ... ok
test context::tests::two_compactions_do_not_resurrect_the_first_summary ... ok
test context::tests::version_one_frames_fold_to_the_same_messages_forever ... ok
test executor::tests::a_call_that_cannot_run_together_splits_the_batch_around_it ... ok
test executor::tests::a_denied_call_fails_without_running_and_a_missing_tool_fails_too ... ok
test executor::tests::a_denied_call_splits_the_batch_around_it_too ... ok
test executor::tests::a_tool_that_ignores_its_token_still_ends_on_the_interrupt ... ok
test executor::tests::an_interrupt_before_a_batch_runs_none_of_it ... ok
test executor::tests::an_interrupt_drops_the_call_in_flight_and_skips_the_rest ... ok
test executor::tests::an_interrupt_keeps_what_a_parallel_batch_already_finished ... ok
test executor::tests::results_are_clipped_at_the_global_cap ... ok
test executor::tests::results_come_back_in_input_order_whichever_finishes_first ... ok
test executor::tests::two_safe_allowed_calls_are_in_flight_at_the_same_moment ... ok
test executor::tests::unsafe_calls_run_one_at_a_time ... ok
test gate::hook_contract::before_ask_ends_hooks_but_the_policy_can_still_deny ... ok
test gate::hook_contract::before_block_ends_the_run ... ok
test gate::hook_contract::before_deny_ends_the_run ... ok
test gate::hook_contract::before_redirect_continues ... ok
test gate::hook_contract::hooks_match_exact_names_and_prefixes ... ok
test gate::summary_tests::a_summary_names_the_subjects_and_falls_back_to_the_input ... ok
test host::auth::tests::the_refusal_names_every_provider_and_what_it_wants ... ok
test host::catalog::tests::a_catalogued_model_carries_the_provider_and_the_four_facts ... ok
test host::catalog::tests::a_model_the_catalogue_does_not_know_carries_the_provider_alone ... ok
test host::catalog::tests::a_tool_entry_carries_its_schema_and_its_traits ... ok
test host::catalog::tests::an_endpoint_that_has_answered_says_which_ids_exist_there ... ok
test host::catalog::tests::the_facts_are_the_catalogue_s_own ... ok
test host::catalog::tests::the_plugins_catalogue_lists_the_sources_after_the_registry ... ok
test host::registry::tests::a_late_source_of_every_kind_is_kept_where_the_turn_reads_it ... ok
test host::registry::tests::a_loss_cascades_to_whoever_required_the_lost_capability ... ok
test host::registry::tests::a_missing_used_service_leaves_the_plugin_standing ... ok
test host::registry::tests::a_plugin_switched_off_is_disabled_with_its_reason_and_registers_nothing ... ok
test host::registry::tests::a_requirement_met_later_in_the_order_still_stands ... ok
test host::registry::tests::a_store_ignores_its_switch_and_the_notice_names_it ... ok
test host::registry::tests::a_switch_cascades_to_whoever_required_what_it_provided ... ok
test host::tests::a_declared_window_is_the_ruler_the_turn_measures_with ... ok
test host::tests::a_plugin_receives_only_the_settings_it_claimed ... ok
test host::tests::a_second_policy_is_a_conflict ... ok
test host::tests::a_session_can_release_its_route_for_a_fresh_session ... ok
test host::tests::a_start_hook_may_read_the_session_tree_and_the_first_turn_waits_for_it ... ok
test host::tests::a_switched_off_plugin_is_neither_registered_nor_started_nor_stopped ... ok
test host::tests::ask::a_question_between_turns_reaches_the_person_and_their_answer_comes_back ... ok
test host::tests::ask::a_question_that_marks_no_option_is_never_answered_for_the_person ... ok
test host::tests::ask::a_question_the_session_closes_under_is_refused_rather_than_left_waiting ... ok
test host::tests::ask::a_question_under_a_running_turn_is_refused_when_the_turn_is_interrupted ... ok
test host::tests::ask::a_refusing_stance_answers_with_the_fail_closed_option_at_once ... ok
test host::tests::ask::a_session_this_host_does_not_run_is_never_answered_for ... ok
test host::tests::ask::a_sign_in_is_put_to_the_person_whatever_the_stance ... ok
test host::tests::ask::an_allowing_stance_answers_the_question_and_opens_nothing ... ok
test host::tests::catalogue::a_denied_setting_changes_neither_the_session_nor_the_file ... ok
test host::tests::catalogue::a_passive_observer_cannot_hold_a_command_acknowledgement ... ok
test host::tests::catalogue::lifecycle_facts_are_ordered_and_completion_text_follows_the_answer ... ok
test host::tests::checkpoint::checkpoint_host_resume_reads_only_the_tail_and_clean_close_writes ... ok
test host::tests::commands::compact_is_a_turn_of_its_own_carrying_the_instructions ... ok
test host::tests::commands::disable_writes_the_user_layer_and_the_next_start_opens_on_it ... ok
test host::tests::commands::model_and_think_change_the_next_turn_and_are_announced ... ok
test host::tests::commands::model_is_remembered_in_the_user_settings ... ok
test host::tests::commands::plugins_and_modules_draw_one_table_with_every_reason_in_it ... ok
test host::tests::commands::rename_names_the_session_and_announces_it ... ok
test host::tests::commands::status_answers_with_the_session_s_facts ... ok
test host::tests::commands::the_catalogue_lists_the_builtins_and_the_models ... ok
test host::tests::commands::the_catalogue_reads_the_sources_too ... ok
test host::tests::commands::think_reaches_an_unknown_model_at_every_level_and_off_omits_it ... ok
test host::tests::commands::thinking_is_remembered_in_the_user_settings ... ok
test host::tests::complete::complete_inside_a_turn_is_in_its_terminal_usage ... ok
test host::tests::complete::complete_outside_a_turn_charges_the_session_and_resolves_its_model ... ok
test host::tests::login::a_flow_that_finishes_on_its_own_closes_the_dialog_it_opened ... ok
test host::tests::login::a_pasted_credential_signs_in_and_the_receipt_is_recorded ... ok
test host::tests::login::cancelling_the_dialog_fails_the_login_and_an_instant_command_does_not ... ok
test host::tests::login::login_names_a_registered_provider_and_logout_answers_a_receipt ... ok
test host::tests::models::a_changed_list_is_announced_and_an_unchanged_one_is_not ... ok
test host::tests::models::a_list_one_process_cached_is_what_the_next_one_answers_with ... ok
test host::tests::models::a_named_instance_reads_its_family_s_shelf_then_the_id_s_own ... ok
test host::tests::models::a_provider_that_cannot_sign_in_is_not_asked ... ok
test host::tests::models::an_endpoint_that_cannot_be_asked_keeps_the_list_it_gave ... ok
test host::tests::models::an_endpoint_that_holds_the_context_names_the_window_the_session_uses ... ok
test host::tests::models::the_list_arrives_on_its_own_after_the_host_is_up ... ok
test host::tests::models::what_the_endpoint_answers_replaces_the_shelf_it_was_filed_under ... ok
test host::tests::open_create_runs_a_turn_and_the_session_is_findable_afterwards ... ok
test host::tests::opening_without_a_provider_or_model_says_so ... ok
test host::tests::plugins_load_in_order_and_unmet_requirements_disable_not_crash ... ok
test host::tests::policy::the_policys_view_is_published_at_open_and_after_a_verdict ... ok
test host::tests::providers::a_session_opens_on_a_provider_that_did_not_exist_at_boot ... ok
test host::tests::providers::the_catalogue_lists_a_late_provider_s_models_where_it_lists_every_other ... ok
test host::tests::providers::the_default_provider_is_one_that_can_answer ... ok
test host::tests::providers::with_nobody_signed_in_the_refusal_lists_every_provider ... ok
test host::tests::queue::a_held_line_waits_out_the_barrier_and_the_next_turn_takes_it ... ok
test host::tests::queue::a_queued_line_comes_back_out_to_the_surface_that_sent_it ... ok
test host::tests::resume::a_child_a_start_hook_reopens_is_followed_from_its_head ... ok
test host::tests::resume::a_delivery_reaches_a_stored_session_that_is_not_live ... ok
test host::tests::resume::a_log_session_needs_no_provider_and_answers_nothing ... ok
test host::tests::resume::a_renamed_session_comes_back_under_its_name ... ok
test host::tests::resume::a_replayed_child_that_wakes_repeats_no_frame ... ok
test host::tests::resume::a_resumed_session_comes_back_on_the_model_it_was_moved_to ... ok
test host::tests::resume::a_resumed_session_is_told_which_child_was_mid_turn ... ok
test host::tests::resume::a_resumed_session_keeps_its_system_prompt_and_tool_set ... ok
test host::tests::resume::a_resumed_session_thinks_as_hard_as_it_was_left ... ok
test host::tests::resume::a_resumed_session_with_no_lost_child_says_nothing ... ok
test host::tests::resume::a_resumed_tree_replays_the_descendants_only_the_store_has ... ok
test host::tests::resume::a_stored_session_reopens_on_another_host_with_its_history ... ok
test host::tests::resume::latest_in_a_directory_comes_from_the_store_when_nothing_is_live ... ok
test host::tests::resume::latest_prefers_a_root_over_the_newer_child_under_it ... ok
test host::tests::resume::record::a_journal_with_unknown_records_resumes_with_known_history ... ok
test host::tests::rewind::a_rewind_appends_the_item_and_takes_the_turn_out_of_every_fold ... ok
test host::tests::rewind::a_rewind_under_a_running_turn_is_refused ... ok
test host::tests::rewind::a_turn_this_session_never_had_is_nothing_to_go_back_to ... ok
test host::tests::routes::a_released_route_allows_both_sessions_to_resume_on_another_host ... ok
test host::tests::routes::a_released_route_reaches_the_actor_and_journal ... ok
test host::tests::routes::a_renamed_route_reaches_the_actor_and_journal ... ok
test host::tests::services::a_service_is_a_typed_value_under_a_key_and_crosses_only_if_it_opened_a_face ... ok
test host::tests::services::a_service_opened_late_answers_by_key_and_by_wire ... ok
test host::tests::services::an_undeclared_service_is_refused_with_its_plugin_and_key ... ok
test host::tests::spec_owner::concurrent_model_and_thinking_changes_keep_both ... ok
test host::tests::spec_owner::every_spec_field_survives_the_journal_round_trip ... ok
test host::tests::spec_owner::message_counts_agree_after_retry_compaction_close_and_resume ... ok
test host::tests::spec_owner::resume_prefers_the_fold_level_and_defaults_only_when_absent ... ok
test host::tests::sub_sessions_are_sessions_with_a_parent_and_a_depth_limit ... ok
test host::tests::tree::a_child_inherits_the_model_and_effort_its_parent_stands_on ... ok
test host::tests::tree::a_child_opened_after_the_attachment_streams_from_its_head_and_is_answered_through_the_root ... ok
test host::tests::tree::a_child_opens_at_the_level_its_spec_names_and_inherits_when_it_names_none ... ok
test host::tests::tree::a_child_that_already_exists_is_followed_from_its_head_too ... ok
test host::tests::tree::a_lagging_child_is_healed_and_the_client_never_sees_a_marker ... ok
test host::tests::tree::a_lagging_root_is_healed_and_its_stream_does_not_end ... ok
test host::tests::tree::bounded_tree_follows_only_new_descendant_frames ... ok
test host::tests::tree::deleting_the_root_deletes_its_children_first ... ok
test host::tests::views::a_signal_is_live_and_never_journaled ... ok
test journal::tests::list_filters_by_cwd_and_orders_newest_first ... ok
test journal::tests::replay_returns_frames_after_the_cursor ... ok
test models::catalog::tests::a_dated_snapshot_reads_as_the_longest_family_at_a_separator ... ok
test models::catalog::tests::a_provider_lists_its_models_in_id_order ... ok
test models::catalog::tests::a_proxied_vendor_model_is_found_under_any_provider_by_exact_id_only ... ok
test models::catalog::tests::an_exact_id_under_its_provider_wins ... ok
test models::catalog::tests::an_instance_reads_its_family_s_shelf_then_its_own_name_s ... ok
test models::catalog::tests::the_embedded_snapshot_parses_and_knows_both_first_party_providers ... ok
test models::declared::tests::a_misspelled_field_is_refused_rather_than_ignored ... ok
test models::declared::tests::a_partial_declaration_leaves_the_other_fields_open ... ok
test models::declared::tests::the_key_joins_provider_and_model ... ok
test models::learned::tests::a_lesson_only_lowers_the_window_and_stays_within_bounds ... ok
test models::learned::tests::a_lesson_outlives_the_process_through_its_file ... ok
test models::learned::tests::a_message_without_a_number_or_with_an_absurd_one_teaches_nothing ... ok
test models::learned::tests::a_rejected_size_alone_stands_in_at_eighty_five_percent ... ok
test models::learned::tests::the_stated_maximum_shapes_of_both_providers_are_read ... ok
test models::resolve::tests::a_learned_window_clamps_the_catalogue_and_a_declaration_outranks_both ... ok
test models::resolve::tests::a_lesson_never_raises_the_window_above_the_catalogue ... ok
test models::resolve::tests::a_window_the_endpoint_names_is_the_window ... ok
test models::resolve::tests::an_unknown_model_has_conservative_output_and_no_reasoning_metadata ... ok
test models::resolve::tests::images_need_both_the_model_and_the_endpoint ... ok
test models::resolve::tests::the_catalogue_speaks_when_nobody_else_does_and_the_default_budget_bounds_it ... ok
test models::resolve::tests::the_declared_word_beats_every_other_owner ... ok
test models::resolve::tests::the_endpoint_can_veto_images_but_never_grant_them ... ok
test models::resolve::tests::the_input_side_always_keeps_half_the_window ... ok
test models::resolve::tests::the_window_an_endpoint_names_beats_the_declaration_and_the_catalogue ... ok
test models::served::tests::a_list_is_stale_a_day_later_and_a_provider_never_asked_is_stale_too ... ok
test models::served::tests::a_list_outlives_the_process_through_its_file ... ok
test models::served::tests::only_a_changed_list_is_worth_announcing_and_an_empty_one_is_no_answer ... ok
test models::served::tests::the_configured_id_is_first_once_and_named_by_whoever_knows_it ... ok
test models::served::tests::the_endpoint_s_list_replaces_the_snapshot_s ... ok
test models::served::tests::the_file_holds_one_stamped_list_per_provider ... ok
test models::vision::tests::a_conversation_without_images_is_left_untouched ... ok
test models::vision::tests::a_picture_with_a_path_leaves_its_path_behind ... ok
test models::vision::tests::images_at_the_top_and_inside_tool_results_become_the_note ... ok
test plugins::tests::a_key_of_the_wrong_shape_is_refused_and_the_file_is_left_alone ... ok
test plugins::tests::a_store_and_a_surface_may_not_be_switched_off_and_the_rest_may ... ok
test plugins::tests::a_switch_joins_the_user_layer_without_disturbing_it ... ok
test prompt::tests::identity_is_cacheable_and_the_env_block_is_not ... ok
test prompt::tests::the_env_block_names_the_working_directory ... ok
test prompt::tests::the_identity_is_the_same_words_for_every_session ... ok
test rewind::tests::a_turn_this_transcript_never_had_is_nothing_to_go_back_to ... ok
test rewind::tests::the_cut_is_the_turns_first_item_and_everything_after_it ... ok
test rewind::tests::the_first_turn_takes_the_whole_transcript ... ok
test session::commands::tests::a_bang_line_is_the_shell_command_with_the_rest_verbatim ... ok
test session::commands::tests::a_run_is_labelled_by_the_line_that_would_have_typed_it ... ok
test session::commands::tests::a_shell_line_with_no_shell_says_that_is_what_is_missing ... ok
test session::commands::tests::a_slash_line_is_a_name_and_its_arguments ... ok
test session::commands::tests::an_action_is_its_name_and_its_arguments_as_text_or_json ... ok
test session::commands::tests::an_invocation_keeps_the_submitter_and_takes_the_command_surface ... ok
test session::commands::tests::only_the_persons_own_line_may_run_a_shell_command ... ok
test session::gate::hook_contract::ask_at_submit_continues ... ok
test session::gate::hook_contract::submit_block_ends_the_run ... ok
test session::gate::hook_contract::submit_deny_ends_the_run ... ok
test session::gate::hook_contract::submit_redirect_ends_the_run ... ok
test session::gate::hook_contract::submit_rewrites_in_order ... ok
test session::queue::tests::a_barrier_absorbs_prose_only_up_to_the_first_command ... ok
test session::queue::tests::a_barrier_stops_at_a_held_line_as_it_stops_at_a_command ... ok
test session::queue::tests::a_queued_line_is_found_by_its_intent_and_taken_out_once ... ok
test session::queue::tests::an_image_only_ask_previews_as_an_image ... ok
test session::queue::tests::prose_is_one_unit_and_a_command_is_another ... ok
test session::queue::tests::the_turn_that_opens_next_takes_the_held_lines ... ok
test session::queue::tests::the_wire_view_marks_a_held_line_as_not_steerable ... ok
test session::queue::tests::the_wire_view_marks_commands_as_not_steerable ... ok
test session::tests::a_busy_session_queues_and_the_queue_opens_the_next_turn ... ok
test session::tests::a_journal_cut_inside_a_turn_resumes_with_that_turn_lost ... ok
test session::tests::a_journal_that_ends_closed_resumes_open ... ok
test session::tests::a_journal_without_its_head_cannot_be_resumed ... ok
test session::tests::a_lagging_subscriber_is_told_and_can_resync ... ok
test session::tests::a_panicking_turn_is_reported_as_lost_not_hung ... ok
test session::tests::a_permission_is_answered_once_and_late_answers_are_rejected ... ok
test session::tests::a_start_hook_finishes_before_the_first_turn_opens ... ok
test session::tests::an_unknown_command_an_unknown_action_and_empty_text_are_rejected ... ok
test session::tests::bounded::a_retry_dropped_cursor_is_stale_without_changing_generation ... ok
test session::tests::bounded::bounded_attachment_keeps_the_atomic_cut_and_an_explicit_older_cursor ... ok
test session::tests::bounded::live_only_tail_subscription_never_replays_old_journal_frames ... ok
test session::tests::busy::a_resumed_session_publishes_no_busy_it_inherited ... ok
test session::tests::busy::a_summary_says_busy_while_the_turn_runs_and_idle_once_it_is_over ... ok
test session::tests::checkpoint::checkpoint_ack_releases_only_unneeded_items_and_covered_frames ... ok
test session::tests::checkpoint::checkpoint_catch_up_includes_the_cut_once_then_live_frames ... ok
test session::tests::checkpoint::checkpoint_failure_retains_the_actor_window ... ok
test session::tests::checkpoint::checkpoint_history_pages_join_the_client_window_without_duplicates ... ok
test session::tests::checkpoint::checkpoint_io_does_not_block_actor_or_discard_newer_frames ... ok
test session::tests::checkpoint::checkpoint_oversized_history_item_is_pinned_from_storage ... ok
test session::tests::checkpoint::checkpoint_storage_page_rechecks_generation_on_delivery ... ok
test session::tests::checkpoint::checkpoint_unbounded_attachment_restores_full_history_at_its_cut ... ok
test session::tests::closing_cancels_the_turn_and_ends_the_journal ... ok
test session::tests::commands::a_command_from_a_source_dispatches_like_a_registered_one ... ok
test session::tests::commands::a_command_that_is_not_instant_waits_and_holds_the_prose_behind_it ... ok
test session::tests::commands::a_compaction_turn_compacts_and_closes_and_is_refused_while_a_turn_runs ... ok
test session::tests::commands::a_failing_command_is_rejected_with_its_error ... ok
test session::tests::commands::a_prompt_outcome_does_not_name_the_session ... ok
test session::tests::commands::a_prompt_outcome_opens_a_turn_with_the_commands_own_intent ... ok
test session::tests::commands::a_record_outcome_is_an_item_and_an_action_dispatches_by_name ... ok
test session::tests::commands::a_shell_line_from_anyone_but_the_person_is_refused ... ok
test session::tests::commands::a_shell_line_with_no_shell_registered_says_so ... ok
test session::tests::commands::an_idle_session_opens_the_question_it_is_handed ... ok
test session::tests::commands::an_instant_command_runs_during_a_turn_and_is_applied ... ok
test session::tests::commands::session_and_journal_hooks_observe_without_delaying_anything ... ok
test session::tests::commands::turn_end_hooks_run_after_the_completion_is_published ... ok
test session::tests::compacting_a_held_session_is_refused_in_the_providers_name ... ok
test session::tests::completion::a_recovered_turn_and_the_turn_after_it_are_each_completed_once ... ok
test session::tests::completion::a_turn_a_close_cancels_is_completed_once ... ok
test session::tests::completion::a_turn_that_runs_to_its_end_is_completed_once ... ok
test session::tests::completion::a_turn_whose_provider_panics_is_completed_once ... ok
test session::tests::completion::a_turn_whose_tool_panics_is_completed_once ... ok
test session::tests::completion::an_interrupted_turn_is_completed_once ... ok
test session::tests::history_pages_backwards_from_the_newest_item ... ok
test session::tests::images::an_image_with_no_words_journals_as_one_image_part ... ok
test session::tests::images::an_oversize_image_is_invalid_input ... ok
test session::tests::images::an_unknown_media_type_is_invalid_input ... ok
test session::tests::images::text_and_two_images_journal_in_order ... ok
test session::tests::interrupting_an_idle_session_is_rejected ... ok
test session::tests::invoke::a_bridged_after_tool_block_completes_at_the_round_boundary ... ok
test session::tests::invoke::a_call_after_the_stream_ended_is_refused_while_the_turn_completes ... ok
test session::tests::invoke::a_call_is_served_while_the_turn_s_stream_is_still_open ... ok
test session::tests::invoke::a_call_still_running_when_its_turn_completes_is_not_called_an_interrupt ... ok
test session::tests::invoke::a_call_the_gate_denies_reports_the_denial_to_the_caller ... ok
test session::tests::invoke::a_call_the_turn_was_not_offered_is_refused_and_never_runs ... ok
test session::tests::invoke::a_call_with_no_turn_in_flight_is_refused_with_a_reason ... ok
test session::tests::invoke::a_full_link_answers_a_call_without_running_the_tool ... ok
test session::tests::invoke::an_interrupt_drops_a_bridged_call_and_the_caller_is_told ... ok
test session::tests::invoke::an_invoked_call_is_seen_by_after_tool ... ok
test session::tests::log::a_delivery_of_either_kind_is_recorded_the_same_way ... ok
test session::tests::log::a_resumed_session_restates_its_extensions_for_the_observers ... ok
test session::tests::log::a_submit_is_recorded_at_once_and_opens_no_turn ... ok
test session::tests::log::an_extension_is_durable_folded_and_the_latest_payload_is_the_state ... ok
test session::tests::log::nothing_compacts_and_nothing_is_there_to_interrupt ... ok
test session::tests::naming::a_name_that_landed_after_the_first_ask_wins_on_resume ... ok
test session::tests::naming::a_session_somebody_named_is_never_renamed ... ok
test session::tests::naming::an_unnamed_journal_is_named_by_the_head_that_reopens_it ... ok
test session::tests::naming::an_unnamed_session_takes_its_first_ask_and_is_not_renamed_again ... ok
test session::tests::owned::a_hung_submit_keeps_queries_and_other_sessions_responsive ... ok
test session::tests::owned::a_turn_ending_during_submit_does_not_let_prose_overtake_a_command ... ok
test session::tests::owned::close_joins_a_hung_command_within_the_grace ... ok
test session::tests::owned::close_rejects_the_gate_head_and_every_waiting_input ... ok
test session::tests::owned::gate_fifo_keeps_prose_command_prose_in_arrival_order ... ok
test session::tests::owned::interrupt_drops_a_compactor_that_ignores_its_token ... ok
test session::tests::owned::interrupt_drops_a_hung_before_tool_hook ... ok
test session::tests::owned::interrupt_drops_a_hung_policy ... ok
test session::tests::owned::interrupt_drops_a_hung_tool_check_link ... ok
test session::tests::owned::interrupt_releases_a_holding_command_and_its_queue ... ok
test session::tests::peers::a_delivery_to_a_busy_session_is_queued_whatever_its_kind ... ok
test session::tests::peers::a_held_briefing_is_read_first_by_the_turn_a_later_wake_opens ... ok
test session::tests::peers::a_held_delivery_waits_in_the_queue_and_the_next_submit_carries_it_first ... ok
test session::tests::peers::a_peer_may_not_deliver_an_action ... ok
test session::tests::peers::a_redirect_hook_sends_the_line_elsewhere_and_acks_where_it_went ... ok
test session::tests::peers::a_redirect_to_a_session_that_is_gone_is_rejected ... ok
test session::tests::peers::a_wake_delivery_to_an_idle_session_opens_a_peer_turn_that_says_who_spoke ... ok
test session::tests::retry::retry_withdrawal_survives_resume ... ok
test session::tests::retry::retry_withdrawal_survives_the_next_turn ... ok
test session::tests::storage::storage_failure_never_publishes_or_observes_the_failed_frame ... ok
test session::tests::storage::storage_failure_stops_a_batch_before_spawning_a_turn ... ok
test session::tests::storage::storage_failure_stops_opening_at_the_first_failed_append ... ok
test session::tests::storage::storage_failure_stops_the_actor_mailbox ... ok
test session::tests::submit_starts_a_turn_and_streams_it_to_the_end ... ok
test session::title::tests::a_cjk_sentence_ends_at_its_own_stop ... ok
test session::title::tests::a_commands_own_prompt_names_nothing ... ok
test session::title::tests::a_long_ask_is_cut_at_a_character_boundary ... ok
test session::title::tests::a_name_is_the_first_sentence_of_the_first_line ... ok
test session::title::tests::a_stop_inside_a_word_is_not_a_stop ... ok
test session::title::tests::nothing_is_named_by_nothing ... ok
test session::title::tests::the_first_ask_is_the_first_prose_a_person_wrote ... ok
test settings::edit::tests::a_comment_above_an_untouched_key_survives ... ok
test settings::edit::tests::a_key_the_new_document_does_not_name_is_removed ... ok
test settings::edit::tests::a_leaf_that_did_not_change_is_not_a_change ... ok
test settings::edit::tests::a_nested_leaf_change_leaves_its_siblings_byte_identical ... ok
test settings::edit::tests::a_new_nested_object_lands_as_a_standard_table ... ok
test settings::edit::tests::a_null_names_the_dotted_key_it_was_written_at ... ok
test settings::edit::tests::a_null_stops_every_change_and_not_just_its_own ... ok
test settings::edit::tests::a_removed_keys_comment_goes_with_it ... ok
test settings::edit::tests::an_existing_inline_table_that_changes_stays_inline ... ok
test settings::edit::tests::an_object_is_recursed_into_and_an_array_is_one_leaf ... ok
test settings::format::tests::a_datetime_reads_as_a_value_rather_than_a_panic ... ok
test settings::format::tests::a_file_that_is_not_toml_says_where_it_stopped ... ok
test settings::format::tests::a_layer_keeps_its_name_when_it_loses_its_extension ... ok
test settings::format::tests::a_toml_root_is_always_an_object_even_when_the_file_is_empty ... ok
test settings::format::tests::the_extension_decides_and_nothing_else_does ... ok
test settings::format::tests::the_same_settings_in_either_language_parse_to_the_same_value ... ok
test settings::merge::tests::a_higher_layer_flips_one_plugin_back_on_and_leaves_its_neighbours ... ok
test settings::merge::tests::a_kernel_key_of_the_wrong_type_names_the_layer_that_set_it ... ok
test settings::merge::tests::a_key_claimed_twice_or_a_kernel_key_claimed_is_a_conflict ... ok
test settings::merge::tests::a_null_over_a_switch_gives_the_plugin_back ... ok
test settings::merge::tests::a_switch_of_the_wrong_type_names_the_layer_that_set_it ... ok
test settings::merge::tests::accumulate_concatenates_in_layer_order_without_repeats ... ok
test settings::merge::tests::accumulate_rejects_a_non_list_with_its_source ... ok
test settings::merge::tests::by_name_replaces_entries_in_place ... ok
test settings::merge::tests::higher_layers_replace_and_objects_merge_field_by_field ... ok
test settings::merge::tests::model_declarations_merge_by_field_across_layers_and_refuse_a_typo ... ok
test settings::merge::tests::null_clears_the_layers_below_it ... ok
test settings::merge::tests::plugin_trust_comes_only_from_an_explicit_user_layer ... ok
test settings::merge::tests::the_switches_are_a_kernel_key_and_never_an_unknown_one ... ok
test settings::merge::tests::unclaimed_keys_are_reported_with_their_source ... ok
test settings::migrate::tests::a_bak_already_there_is_never_overwritten ... ok
test settings::migrate::tests::a_jsonc_layer_crosses_and_the_bak_keeps_what_did_not ... ok
test settings::migrate::tests::a_layer_that_has_crossed_is_left_alone_on_every_run_after ... ok
test settings::migrate::tests::a_null_refuses_by_name_and_moves_nothing ... ok
test settings::migrate::tests::every_layer_crosses_on_one_call ... ok
test settings::migrate::tests::nothing_to_move_is_nothing_said ... ok
test settings::tests::a_file_with_comments_is_read_but_never_rewritten ... ok
test settings::tests::a_key_no_one_knows_under_pictures_is_said_out_loud ... ok
test settings::tests::a_layer_directory_with_no_toml_reads_the_json_that_stands_there ... ok
test settings::tests::a_life_that_is_not_a_number_of_days_is_refused ... ok
test settings::tests::a_non_object_layer_is_an_error_and_an_empty_file_is_skipped ... ok
test settings::tests::a_null_handed_to_remember_takes_the_key_out_of_the_layer ... ok
test settings::tests::a_null_over_a_life_gives_the_default_back ... ok
test settings::tests::a_toml_layer_that_will_not_parse_names_the_file ... ok
test settings::tests::a_write_into_a_json_layer_that_spells_a_null_is_refused_by_name ... ok
test settings::tests::a_write_into_a_layer_that_is_still_json_migrates_it_first ... ok
test settings::tests::an_explicit_settings_file_is_read_where_it_is_and_never_migrated ... ok
test settings::tests::load_reads_the_layers_that_exist_in_priority_order ... ok
test settings::tests::never_caching_is_a_number_like_any_other ... ok
test settings::tests::remember_sets_its_keys_and_leaves_every_neighbour_where_it_was ... ok
test settings::tests::the_cache_life_comes_from_the_highest_layer_that_names_it ... ok
test turn::late::tests::a_call_a_hook_did_not_claim_never_reaches_it ... ok
test turn::late::tests::a_point_asks_only_the_hooks_that_claim_it ... ok
test turn::late::tests::a_session_with_no_hook_anywhere_is_empty ... ok
test turn::late::tests::a_source_s_contributors_join_the_registered_ones_in_order ... ok
test turn::late::tests::a_source_s_hooks_join_the_registered_ones_in_order ... ok
test turn::late::tests::a_source_s_providers_join_the_registered_ones_and_never_shadow_one ... ok
test turn::late::tests::a_source_s_strategy_is_used_where_the_slot_is_free ... ok
test turn::late::tests::a_turn_with_no_provider_anywhere_gathers_none ... ok
test turn::late::tests::a_turn_with_no_strategy_anywhere_resolves_to_none ... ok
test turn::late::tests::the_registered_strategy_keeps_the_slot_against_a_source_s ... ok
test turn::ruler::tests::a_held_ruler_never_warns ... ok
test turn::ruler::tests::a_held_ruler_reads_the_endpoints_own_count_and_adds_nothing ... ok
test turn::ruler::tests::a_kernel_ruler_is_as_it_was ... ok
test turn::tests::a_denied_permission_fails_the_call_and_records_a_receipt ... ok
test turn::tests::a_hook_that_asks_opens_a_permission_with_its_reason_and_allow_runs_the_tool ... ok
test turn::tests::a_length_stop_injects_a_continue_prompt_up_to_three_times ... ok
test turn::tests::a_model_without_vision_gets_a_note_where_the_image_was ... ok
test turn::tests::a_non_retryable_error_fails_the_turn_with_its_code ... ok
test turn::tests::a_picture_steered_in_at_the_barrier_is_kept ... ok
test turn::tests::a_retryable_error_withdraws_the_attempt_and_tries_again ... ok
test turn::tests::a_source_contributor_speaks_when_the_turn_starts_with_its_own_origin ... ok
test turn::tests::a_source_s_hook_composes_where_a_second_registered_hook_would ... ok
test turn::tests::a_source_strategy_compacts_when_nothing_in_process_holds_the_slot ... ok
test turn::tests::a_source_tool_is_gathered_when_the_turn_starts_and_a_duplicate_is_dropped ... ok
test turn::tests::a_stream_that_ends_without_a_finish_is_retried_like_a_dropped_connection ... ok
test turn::tests::a_text_only_turn_streams_one_assistant_item_and_completes ... ok
test turn::tests::a_tool_round_gates_executes_and_feeds_the_result_back ... ok
test turn::tests::an_absorbed_input_lands_between_one_round_s_tool_item_and_the_next ... ok
test turn::tests::an_empty_response_is_retried_once_then_accepted ... ok
test turn::tests::an_interrupt_mid_stream_keeps_the_text_and_records_the_marker ... ok
test turn::tests::an_overflow_teaches_the_window_the_server_named ... ok
test turn::tests::attribution::a_peer_at_the_barrier_preserves_the_previous_request_prefix ... ok
test turn::tests::attribution::a_turn_that_mixes_tells_the_model_which_line_is_the_persons ... ok
test turn::tests::backoff_doubles_from_half_a_second_and_honours_the_server ... ok
test turn::tests::budget::a_summary_that_shrinks_nothing_is_discarded_billed_and_counted ... ok
test turn::tests::budget::accepted_threshold_cuts_are_bounded_when_system_and_reissued_context_stay_large ... ok
test turn::tests::budget::an_overflow_passes_the_failures_on_and_retries_once ... ok
test turn::tests::budget::failed_and_cancelled_summary_usage_is_billed_once_without_cutting ... ok
test turn::tests::budget::manual_threshold_and_overflow_receive_the_assembled_parent_request ... ok
test turn::tests::budget::normal_long_turns_preserve_old_results_and_the_request_prefix ... ok
test turn::tests::budget::overflow_recovery_elides_only_the_wire_and_retries_once ... ok
test turn::tests::budget::the_person_is_warned_once_near_the_line ... ok
test turn::tests::budget::the_ruler_never_reads_below_what_the_server_counted ... ok
test turn::tests::budget::three_useless_summaries_trip_the_breaker_and_one_good_one_resets_it ... ok
test turn::tests::chain::a_full_link_answers_compaction_without_the_strategy ... ok
test turn::tests::chain::a_full_link_answers_turn_step_without_the_provider ... ok
test turn::tests::chain::a_tighten_link_at_turn_step_observes_only ... ok
test turn::tests::chain::the_ruler_measures_the_rewritten_composition ... ok
test turn::tests::compaction_hooks_bracket_the_cut ... ok
test turn::tests::held::a_cut_the_endpoint_made_is_journalled_as_a_compaction_row ... ok
test turn::tests::held::a_held_turn_asks_for_no_summary_where_the_kernels_own_would ... ok
test turn::tests::held::a_reading_is_the_rounds_context_whatever_the_bill_said ... ok
test turn::tests::held::an_overflow_on_a_held_session_fails_the_turn_without_a_ladder ... ok
test turn::tests::hook_contract::after_every_matched_hook_sees_every_result_and_block_ends_the_round ... ok
test turn::tests::hook_contract::stop_first_block_adds_its_origin_and_another_round ... ok
test turn::tests::pictures::a_picture_a_tool_returned_leaves_the_wire_after_three_answers ... ok
test turn::tests::pictures::a_picture_at_the_top_of_what_the_person_said_ages_the_same_way ... ok
test turn::tests::queued_input_is_absorbed_at_the_barrier ... ok
test turn::tests::quiet::quiet_is_stamped_when_opened_and_kept_through_completion ... ok
test turn::tests::racing::esc_ends_a_turn_a_contributor_is_still_thinking_for ... ok
test turn::tests::racing::esc_ends_a_turn_a_start_hook_is_still_holding ... ok
test turn::tests::racing::esc_ends_a_turn_still_gathering_its_tools ... ok
test turn::tests::racing::esc_ends_a_turn_waiting_on_an_exact_count ... ok
test turn::tests::racing::esc_ends_a_turn_whose_request_is_still_being_established ... ok
test turn::tests::snapshots::compaction_replay_reissues_a_removed_snapshot_then_keeps_the_request_prefix ... ok
test turn::tests::the_request_names_the_session_the_turn_runs_for ... ok
test turn::tests::the_round_budget_stops_a_runaway_loop ... ok
test turn::tests::turn_start_shares_ten_thousand_bodies_without_a_journal ... ok

test result: ok. 483 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.10s

     Running unittests src/lib.rs (target/debug/deps/bingo_demo_ui-206de6096038ba96)

running 16 tests
test board::tests::a_fresh_board_has_three_rows_and_none_of_them_has_started ... ok
test board::tests::a_tick_moves_the_first_row_that_is_not_done_and_then_the_next ... ok
test board::tests::a_view_that_is_not_a_board_is_not_read_as_one ... ok
test board::tests::every_tick_of_a_finished_board_leaves_it_as_it_is ... ok
test board::tests::the_board_shows_one_element_the_sdk_has_no_word_for ... ok
test board::tests::the_view_is_a_panel_of_a_table_and_its_buttons ... ok
test board::tests::the_view_is_the_payload_and_the_board_reads_back_out_of_it ... ok
test command::tests::a_reset_starts_over_and_takes_the_progress_bar_away ... ok
test command::tests::every_command_runs_now_and_takes_nothing ... ok
test command::tests::showing_the_board_publishes_a_fresh_one_and_tick_moves_it_on ... ok
test plugin_tests::registering_reads_nothing_and_contributes_what_the_manifest_promises ... ok
test plugin_tests::the_manifest_says_what_it_provides_and_claims_one_setting ... ok
test plugin_tests::the_setting_is_off_unless_it_says_otherwise ... ok
test progress::tests::the_bar_is_published_once_a_step_and_never_journaled ... ok
test progress::tests::the_label_is_the_tools_own_name_when_none_is_given ... ok
test progress::tests::the_tool_reads_nothing_and_is_alone_in_its_kind ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/lib.rs (target/debug/deps/bingo_digest-a03972de1a7e0615)

running 3 tests
test tests::a_million_letters_hash_as_the_standard_says ... ok
test tests::the_padding_is_right_on_both_sides_of_a_block ... ok
test tests::the_standards_own_vectors ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running unittests src/lib.rs (target/debug/deps/bingo_experience-447659c7417ac6a5)

running 82 tests
test bm25::tests::a_document_is_recalled_by_its_own_text ... ok
test bm25::tests::a_weighted_field_outranks_the_same_word_in_a_plain_one ... ok
test bm25::tests::every_token_is_a_stem_or_a_bigram ... ok
test bm25::tests::ranking_finds_the_right_document_across_scripts ... ok
test bm25::tests::the_floor_drops_the_riders_and_no_floor_keeps_them ... ok
test bm25::tests::the_floor_keeps_a_prefix_of_the_unfloored_ranking ... ok
test bm25::tests::tokenize_stems_ascii_and_bigrams_cjk ... ok
test bm25::tests::ubiquitous_tokens_still_match_in_tiny_corpora ... ok
test command::tests::a_file_that_could_not_be_read_is_said_out_loud ... ok
test command::tests::an_empty_library_is_one_line ... ok
test command::tests::every_entry_is_a_row_in_the_order_the_prompt_shows_them ... ok
test command::tests::the_spec_runs_now_and_takes_nothing ... ok
test contributor::index::tests::an_empty_library_says_there_are_no_active_playbooks ... ok
test contributor::index::tests::changed_and_retired_entries_append_and_visible_journal_suppresses_repeats ... ok
test contributor::index::tests::it_refreshes_at_the_start_of_each_round ... ok
test contributor::index::tests::past_ten_the_rest_is_one_line_naming_the_tool ... ok
test contributor::index::tests::the_most_useful_entries_come_first ... ok
test contributor::recall::tests::a_retired_entry_is_never_recalled ... ok
test contributor::recall::tests::an_empty_store_and_an_empty_question_both_contribute_nothing ... ok
test contributor::recall::tests::at_most_three_lines_reach_the_prompt ... ok
test contributor::recall::tests::it_answers_a_message_once_however_many_rounds_the_turn_runs ... ok
test contributor::recall::tests::it_is_a_round_start_piece ... ok
test contributor::recall::tests::nothing_relevant_is_nothing_at_all ... ok
test contributor::recall::tests::the_entries_that_fit_the_question_land_as_a_user_item ... ok
test entry::tests::an_age_is_hours_then_days ... ok
test entry::tests::the_counts_are_read_off_the_history ... ok
test entry::tests::the_same_playbook_is_the_trigger_the_summary_and_the_steps ... ok
test frontmatter::tests::a_file_that_does_not_parse_says_why ... ok
test frontmatter::tests::a_hand_written_file_is_read_as_it_was_meant ... ok
test frontmatter::tests::an_unknown_key_is_ignored_and_the_body_keeps_its_fences ... ok
test frontmatter::tests::the_adversarial_entry_survives_the_round_trip ... ok
test frontmatter::tests::the_counts_are_nowhere_in_the_file ... ok
test guide::tests::the_page_is_one_line_of_description_and_a_body_worth_reading ... ok
test guide::tests::the_page_names_every_status_an_entry_may_have_and_every_outcome ... ok
test guide::tests::the_page_names_every_tool_and_command_this_plugin_registers ... ok
test guide::tests::the_page_names_the_setting_that_turns_the_library_off ... ok
test guide::tests::the_settings_example_is_toml_and_no_json_is_left ... ok
test plugin_tests::a_project_that_turned_playbooks_off_is_offered_none ... ok
test plugin_tests::registering_reads_nothing_and_contributes_what_the_manifest_promises ... ok
test plugin_tests::the_manifest_says_what_it_provides_and_claims_one_setting ... ok
test plugin_tests::the_settings_slice_says_whether_this_project_keeps_playbooks ... ok
test project::tests::a_directory_outside_a_repository_is_its_own_key ... ok
test project::tests::a_key_is_one_directory_name ... ok
test project::tests::every_spelling_of_one_remote_is_one_key ... ok
test rank::tests::nothing_matches_nothing ... ok
test rank::tests::the_trigger_outweighs_the_notes ... ok
test render::tests::a_row_has_a_column_per_header ... ok
test render::tests::a_summary_with_more_than_one_line_is_cut_in_a_line_and_whole_in_the_block ... ok
test render::tests::one_line_is_the_id_the_summary_and_the_counts ... ok
test render::tests::the_whole_playbook_is_the_when_the_steps_and_the_check ... ok
test render::tests::the_worth_of_an_entry_is_what_it_has_helped_with_and_whether_it_is_live ... ok
test store::tests::a_file_nobody_can_parse_costs_one_entry_and_is_named ... ok
test store::tests::a_minted_id_is_free_in_this_project ... ok
test store::tests::a_store_that_does_not_exist_is_an_empty_shelf ... ok
test store::tests::an_entry_round_trips_through_the_directory ... ok
test store::tests::saving_the_same_id_replaces_the_file_and_forgetting_removes_it ... ok
test store::tests::the_shelf_is_in_id_order_whatever_the_directory_says ... ok
test store::tests::two_directories_are_two_stores ... ok
test tools::commit::tests::a_card_for_a_call_that_cannot_run_is_no_card ... ok
test tools::commit::tests::a_playbook_becomes_one_file_named_by_its_minted_id ... ok
test tools::commit::tests::a_playbook_without_a_step_is_an_input_error ... ok
test tools::commit::tests::an_id_nobody_has_is_an_answer_and_writes_nothing ... ok
test tools::commit::tests::revising_by_id_keeps_the_history_and_the_day_it_was_written ... ok
test tools::commit::tests::the_card_shows_the_file_it_would_write ... ok
test tools::commit::tests::the_same_playbook_twice_revises_instead_of_forking ... ok
test tools::commit::tests::the_spec_is_an_approved_write ... ok
test tools::forget::tests::an_ambiguous_prefix_deletes_nothing ... ok
test tools::forget::tests::one_entry_goes_and_the_others_stay ... ok
test tools::forget::tests::the_card_shows_the_file_that_would_go ... ok
test tools::forget::tests::the_spec_is_destructive ... ok
test tools::outcome::tests::a_record_is_appended_and_counted_but_never_written_as_a_count ... ok
test tools::outcome::tests::an_id_nobody_has_is_an_answer ... ok
test tools::outcome::tests::evidence_is_required ... ok
test tools::outcome::tests::recording_an_outcome_never_changes_the_status ... ok
test tools::outcome::tests::the_spec_is_a_write_a_person_sees ... ok
test tools::query::tests::a_limit_cuts_the_list_and_nothing_at_all_says_what_to_do ... ok
test tools::query::tests::a_retired_entry_is_still_an_answer_and_says_so ... ok
test tools::query::tests::a_weak_match_is_still_returned_because_someone_asked ... ok
test tools::query::tests::the_best_entry_comes_first_and_carries_its_steps ... ok
test tools::query::tests::the_spec_reads_and_nothing_else ... ok
test tools::tests::an_ambiguous_prefix_lists_what_it_could_have_meant ... ok
test tools::tests::an_unknown_prefix_names_the_tool_that_searches ... ok

test result: ok. 82 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.15s

     Running tests/shelf.rs (target/debug/deps/shelf-a59210559999f2d8)

running 1 test
test reading_hand_edited_experience_never_rewrites_it ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running unittests src/lib.rs (target/debug/deps/bingo_gateway-40c929c6017a4f8d)

running 50 tests
test doctor::tests::a_configured_channel_with_no_secret_is_named_and_no_secret_is_printed ... ok
test doctor::tests::a_live_lock_is_ok_and_a_dead_one_is_bad_and_fixable ... ok
test doctor::tests::a_running_gateway_older_than_this_binary_is_a_warning_not_a_failure ... ok
test doctor::tests::a_secret_in_the_store_is_reported_by_its_location_and_never_its_value ... ok
test doctor::tests::a_settings_json_standing_where_no_toml_does_is_read_and_named ... ok
test doctor::tests::every_lock_under_the_data_dir_is_found_whatever_plugin_left_it ... ok
test doctor::tests::fix_removes_the_dead_locks_it_just_reported_and_only_those ... ok
test doctor::tests::fix_with_nothing_wrong_removes_nothing_and_says_so ... ok
test doctor::tests::modern_channel_claims_are_probed_and_never_removed ... ok
test doctor::tests::preflight_passes_a_channel_that_signs_with_nothing ... ok
test doctor::tests::preflight_refuses_a_channel_that_cannot_sign_with_the_doctor_s_own_line ... ok
test doctor::tests::preflight_refuses_a_gateway_with_no_channel_and_says_how_to_add_one ... ok
test doctor::tests::settings_that_will_not_parse_are_the_first_thing_reported ... ok
test doctor::tests::the_log_row_says_where_it_is_and_that_it_can_be_written ... ok
test log::tests::a_log_that_was_never_written_opens_without_a_roll ... ok
test log::tests::a_log_under_the_cap_is_appended_to_and_an_oversized_one_is_rolled_aside ... ok
test log::tests::a_tail_is_the_last_lines_and_never_more_than_there_are ... ok
test log::tests::a_tail_of_a_log_with_no_final_newline_keeps_the_last_line ... ok
test paths::tests::both_files_sit_under_the_data_dir_s_own_gateway_directory ... ok
test pidfile::tests::a_file_that_is_not_a_record_is_an_error_naming_the_path ... ok
test pidfile::tests::a_record_survives_the_round_trip_through_the_file_it_is_written_as ... ok
test pidfile::tests::the_record_this_process_writes_names_this_process_and_this_binary ... ok
test probe::tests::a_stranger_is_not_a_bingo_and_a_missing_answer_is_not_either ... ok
test probe::tests::the_fake_refuses_to_signal_what_is_not_running ... ok
test probe::tests::the_real_table_knows_this_process_is_here_and_a_reaped_one_is_not ... ok
test probe::tests::the_real_table_names_the_program_a_live_pid_is_running ... ok
test run::tests::a_live_pid_that_is_not_a_bingo_is_still_never_stepped_on ... ok
test run::tests::a_record_whose_process_is_alive_is_refused_and_left_alone ... ok
test run::tests::a_record_whose_process_is_gone_is_taken_over ... ok
test service::tests::launchd_is_told_in_the_words_launchctl_answers_to ... ok
test service::tests::systemd_is_told_in_the_words_systemctl_answers_to ... ok
test service::tests::the_mode_is_the_file_and_nothing_else ... ok
test service::tests::the_uid_is_a_number_this_machine_agrees_with ... ok
test start::tests::giving_up_names_the_log_and_shows_the_end_of_it ... ok
test start::tests::the_forwarded_argv_carries_the_directory_and_the_settings_and_nothing_else ... ok
test state::tests::no_record_is_stopped_a_live_one_is_running_a_dead_one_is_stale ... ok
test status::tests::a_gateway_older_than_the_binary_here_is_told_to_be_restarted ... ok
test status::tests::logs_says_what_is_there_or_that_nothing_is ... ok
test status::tests::status_names_both_files_even_when_nothing_runs ... ok
test status::tests::the_headline_says_running_stale_or_stopped ... ok
test status::tests::uptime_says_the_two_units_that_matter_and_stops ... ok
test stop::tests::a_live_pid_that_is_not_a_bingo_is_never_signalled ... ok
test stop::tests::a_stale_record_is_cleared_and_the_person_is_pointed_at_doctor ... ok
test stop::tests::stopping_what_never_started_says_so_and_is_not_a_failure ... ok
test unit::tests::a_path_with_xml_in_it_cannot_break_the_plist_open ... ok
test unit::tests::each_supervisor_keeps_its_file_where_it_looks_for_one ... ok
test unit::tests::neither_file_carries_a_secret_or_any_environment_at_all ... ok
test unit::tests::the_plist_runs_this_binary_and_keeps_it_alive_only_when_it_failed ... ok
test unit::tests::the_supervisor_here_is_the_one_this_platform_has ... ok
test unit::tests::the_unit_runs_this_binary_and_restarts_only_on_failure ... ok

test result: ok. 50 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/claim.rs (target/debug/deps/claim-121d63ac7112f494)

running 8 tests
test a_claimed_record_naming_a_live_non_gateway_does_not_block_run ... ok
test a_free_claim_is_stale_and_stop_never_signals_its_live_pid ... ok
test a_killed_gateway_can_restart_without_removing_files ... ok
test a_legacy_record_naming_a_live_child_still_refuses ... ok
test a_second_gateway_cannot_overwrite_a_held_claim ... ok
test child_process ... ok
test doctor_never_removes_the_gateway_claim_file ... ok
test new_records_declare_claim_ownership ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

     Running unittests src/lib.rs (target/debug/deps/bingo_hooks_shell-e842f4c7946db4c5)

running 104 tests
test config::tests::a_claude_code_hooks_block_parses_as_written ... ok
test config::tests::a_hook_type_this_plugin_cannot_run_is_refused ... ok
test config::tests::a_misspelt_field_is_refused ... ok
test config::tests::a_rule_without_a_matcher_is_a_rule ... ok
test config::tests::an_event_this_plugin_does_not_serve_is_refused ... ok
test config::tests::no_event_is_claimed_twice_and_every_one_is_claimed ... ok
test config::tests::the_schema_is_generated_from_the_claimed_types ... ok
test dispatch::tests::a_block_says_why_in_the_hook_s_own_words ... ok
test dispatch::tests::a_hook_gets_a_minute_unless_it_asked_for_something_else ... ok
test dispatch::tests::session_end_is_quick_by_default_and_capped_when_it_is_not ... ok
test env_file::tests::a_later_line_wins_over_an_earlier_one ... ok
test env_file::tests::a_line_that_is_not_an_assignment_is_skipped_not_guessed_at ... ok
test env_file::tests::an_empty_value_is_an_assignment ... ok
test env_file::tests::comments_and_blank_lines_are_skipped ... ok
test env_file::tests::plain_and_exported_assignments_are_both_read ... ok
test env_file::tests::quotes_are_stripped_and_a_value_may_hold_anything ... ok
test env_file::tests::the_path_names_the_session ... ok
test events::tests::a_multipart_result_reaches_the_hook_whole ... ok
test events::tests::notification_shape ... ok
test events::tests::permission_request_shape ... ok
test events::tests::post_tool_use_failure_shape ... ok
test events::tests::post_tool_use_shape ... ok
test events::tests::pre_compact_shape ... ok
test events::tests::pre_tool_use_shape ... ok
test events::tests::session_end_shape ... ok
test events::tests::session_start_shape ... ok
test events::tests::session_start_without_a_model_omits_it ... ok
test events::tests::stop_shape ... ok
test events::tests::user_prompt_submit_shape ... ok
test guide::tests::the_page_is_one_line_of_description_and_a_body_worth_reading ... ok
test guide::tests::the_page_names_every_event_this_plugin_serves ... ok
test guide::tests::the_page_names_the_settings_key_this_plugin_claims ... ok
test guide::tests::the_page_registers_nothing_else_to_describe ... ok
test guide::tests::the_settings_example_is_toml_and_no_json_is_left ... ok
test hook::tests::a_deny_refuses_the_call_with_its_reason ... ok
test hook::tests::a_failed_result_reaches_the_hook_as_tool_error ... ok
test hook::tests::a_frame_no_hook_asked_for_runs_nothing ... ok
test hook::tests::a_hook_past_its_deadline_is_killed_and_decides_nothing ... ok
test hook::tests::a_hook_that_fails_any_other_way_decides_nothing ... ok
test hook::tests::a_large_input_and_a_hook_that_never_reads_it_still_finishes ... ok
test hook::tests::a_later_hook_is_told_what_an_earlier_one_rewrote ... ok
test hook::tests::a_matcher_selects_the_tools_it_names ... ok
test hook::tests::a_notice_reaches_a_notification_hook_matched_on_its_code ... ok
test hook::tests::a_prompt_hook_appends_its_context_to_the_input ... ok
test hook::tests::a_prompt_hook_can_reject_the_input ... ok
test hook::tests::a_result_that_failed_is_a_different_event_from_one_that_did_not ... ok
test hook::tests::a_second_session_start_hook_sees_the_first_one_s_exports ... ok
test hook::tests::a_session_end_hook_does_not_hold_the_teardown ... ok
test hook::tests::a_session_that_ended_forgets_what_it_exported ... ok
test hook::tests::a_stop_hook_can_ask_for_one_more_turn ... ok
test hook::tests::a_stop_hook_may_also_block_in_json ... ok
test hook::tests::a_stop_hook_that_is_content_lets_the_turn_end ... ok
test hook::tests::an_action_carries_no_prompt_and_runs_no_hook ... ok
test hook::tests::an_allow_still_goes_to_the_gate ... ok
test hook::tests::an_ask_puts_the_call_to_a_person ... ok
test hook::tests::an_event_nobody_configured_runs_nothing ... ok
test hook::tests::an_opened_permission_reaches_a_permission_request_hook ... ok
test hook::tests::every_post_tool_hook_runs_even_after_one_objects ... ok
test hook::tests::exit_two_denies_with_what_the_hook_wrote_on_stderr ... ok
test hook::tests::exit_two_without_a_word_still_names_the_event_that_blocked ... ok
test hook::tests::only_the_start_of_a_compaction_is_a_pre_compact ... ok
test hook::tests::output_that_is_not_json_decides_nothing ... ok
test hook::tests::the_call_reaches_the_hook_as_the_documented_shape ... ok
test hook::tests::the_first_hook_to_object_settles_the_call ... ok
test hook::tests::the_matcher_claims_every_point_this_plugin_serves ... ok
test hook::tests::the_prompt_reaches_the_hook_under_its_documented_name ... ok
test hook::tests::updated_input_accumulates_over_the_hooks ... ok
test hook::tests::what_session_start_exports_reaches_the_next_hook ... ok
test matcher::tests::a_pattern_is_compiled_once_and_reused ... ok
test matcher::tests::an_anchored_pattern_stays_anchored_when_it_alternates ... ok
test matcher::tests::the_matcher_table ... ok
test program::tests::a_rule_with_no_commands_is_not_a_rule ... ok
test program::tests::an_event_nobody_configured_selects_nothing ... ok
test program::tests::the_rules_that_match_contribute_their_commands_in_order ... ok
test run::tests::a_command_that_cannot_start_is_an_error_not_a_panic ... ok
test run::tests::a_hook_that_never_reads_a_large_stdin_still_finishes ... ok
test run::tests::a_hook_that_writes_more_than_the_cap_still_ends ... ok
test run::tests::a_missing_shell_is_a_hook_error_with_the_remedy ... ok
test run::tests::the_deadline_kills_the_whole_group ... ok
test run::tests::the_environment_reaches_the_hook ... ok
test run::tests::the_exit_code_and_stderr_come_back ... ok
test run::tests::the_hook_reads_its_event_on_stdin ... ok
test run::tests::the_hook_runs_where_the_session_is ... ok
test session::tests::a_session_nobody_opened_has_no_environment ... ok
test session::tests::closing_forgets_the_environment_and_the_file ... ok
test session::tests::one_session_never_sees_another_s_exports ... ok
test session::tests::opening_a_session_starts_from_nothing ... ok
test session::tests::two_hooks_appending_both_survive ... ok
test session::tests::what_a_hook_exported_reaches_every_later_hook ... ok
test tests::a_process_with_no_hooks_configured_still_registers ... ok
test tests::a_settings_mistake_stops_the_boot_instead_of_being_ignored ... ok
test tests::the_manifest_claims_only_the_hooks_key ... ok
test tests::the_plugin_registers_one_hook ... ok
test verdict::tests::a_decision_this_plugin_does_not_know_is_no_decision ... ok
test verdict::tests::a_hook_that_says_nothing_decides_nothing ... ok
test verdict::tests::additional_context_is_read_from_hook_specific_output ... ok
test verdict::tests::continue_false_halts_with_its_reason ... ok
test verdict::tests::hook_specific_output_wins_where_the_two_disagree ... ok
test verdict::tests::json_that_does_not_parse_is_not_a_verdict ... ok
test verdict::tests::plain_text_on_stdout_is_not_a_verdict ... ok
test verdict::tests::the_documented_dialect_is_read ... ok
test verdict::tests::the_older_dialect_is_read_too ... ok
test verdict::tests::updated_input_merges_field_by_field ... ok
test verdict::tests::updated_input_that_is_not_an_object_replaces ... ok

test result: ok. 104 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.24s

     Running unittests src/lib.rs (target/debug/deps/bingo_http-48e177f10f4d25b3)

running 14 tests
test metered::tests::a_clock_that_keeps_moving_is_never_quiet ... ok
test metered::tests::a_clock_that_stops_is_quiet_one_idle_later ... ok
test metered::tests::a_stamp_pushes_the_silence_out_by_a_whole_idle ... ok
test metered::tests::an_empty_body_ends_at_once ... ok
test metered::tests::every_frame_stamps_the_clock_once ... ok
test metered::tests::the_frames_are_the_body_cut_at_sixty_four_kibibytes ... ok
test metered::tests::the_size_hint_is_exact_so_the_request_is_never_chunked ... ok
test sse::tests::a_body_that_ends_without_a_blank_line_still_yields_its_tail ... ok
test sse::tests::a_boundary_split_across_chunks_is_still_found ... ok
test sse::tests::a_comment_only_block_yields_nothing ... ok
test sse::tests::a_frame_split_across_chunks_is_reassembled ... ok
test sse::tests::an_unframed_body_is_a_protocol_error_instead_of_unbounded_growth ... ok
test sse::tests::crlf_and_repeated_data_lines_join_with_a_newline ... ok
test sse::tests::one_frame_is_an_event_and_its_data ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s

     Running tests/contract.rs (target/debug/deps/contract-654b3e183ce7fad0)

running 1 test
test the_bounds_preserve_the_transport_contract ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s

     Running tests/stream.rs (target/debug/deps/stream-be6d069add1a8046)

running 5 tests
test a_body_that_never_frames_is_an_error ... ok
test cancellation_stops_a_pending_stream ... ok
test chunked_equals_whole_and_keeps_events_before_an_error ... ok
test quiet_times_out_and_ends ... ok
test transport_failure_ends ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s

     Running tests/transport.rs (target/debug/deps/transport-80e8d785e5b889c7)

running 4 tests
test a_peer_that_stops_reading_ends_the_request ... ok
test a_server_that_accepts_and_says_nothing_times_out ... ok
test a_slow_upload_is_not_a_silence ... ok
test the_request_head_carries_its_length_and_is_never_chunked ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.90s

     Running unittests src/lib.rs (target/debug/deps/bingo_jsonrpc-7d2f09db6493997f)

running 10 tests
test contracts::a_notification_has_no_id ... ok
test contracts::a_request_round_trips ... ok
test contracts::a_result_and_an_error_are_never_both_present ... ok
test contracts::a_string_id_stays_a_string ... ok
test contracts::absent_params_parse_as_null ... ok
test contracts::another_version_is_not_a_message ... ok
test peer_contract::a_dropped_request_removes_its_entry ... ok
test peer_contract::an_overlong_line_ends_the_peer_without_a_newline ... ok
test peer_contract::closure_fails_every_pending_request ... ok
test peer_contract::out_of_order_responses_find_their_request ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s

     Running unittests src/lib.rs (target/debug/deps/bingo_loopback-e63d25b57fd3b685)

running 42 tests
test answer::tests::a_cancel_is_a_cancel_whatever_else_it_carries ... ok
test answer::tests::a_submitted_value_comes_back_as_the_json_it_was ... ok
test answer::tests::an_envelope_this_does_not_know_is_refused_by_name ... ok
test answer::tests::any_json_is_a_value ... ok
test browser::tests::only_the_exact_client_mode_delegates_presentation ... ok
test browser::tests::the_opt_out_is_honoured_and_a_command_is_built_for_this_platform ... ok
test browser::tests::the_opt_out_wins_over_every_browser_mode ... ok
test page::tests::a_document_the_caller_wrote_is_served_as_it_wrote_it ... ok
test page::tests::a_fragment_gets_a_doctype_a_charset_and_the_title_it_was_given ... ok
test page::tests::a_title_is_text_and_cannot_open_a_tag ... ok
test page::tests::everything_else_is_missing ... ok
test page::tests::the_page_and_its_answer_are_the_only_two_paths ... ok
test page::tests::the_url_is_the_bound_port_and_the_token ... ok
test request::tests::a_get_names_its_method_and_target_and_carries_no_body ... ok
test request::tests::a_head_without_a_method_and_a_target_is_refused ... ok
test request::tests::a_length_that_is_not_a_number_is_refused_rather_than_guessed ... ok
test request::tests::a_lower_cased_method_is_read_as_the_method_it_is ... ok
test request::tests::a_post_reads_its_length_whatever_the_header_is_cased_like ... ok
test request::tests::a_query_is_part_of_the_target_and_not_part_of_the_path ... ok
test response::tests::a_missing_path_says_nothing_about_why ... ok
test response::tests::a_page_carries_its_length_its_type_and_the_close ... ok
test response::tests::the_length_counts_bytes ... ok
test script::tests::a_page_with_a_body_carries_the_script_just_before_it_closes ... ok
test script::tests::a_page_without_a_body_carries_it_at_the_end ... ok
test script::tests::the_last_body_end_is_the_one_used_and_the_case_does_not_matter ... ok
test script::tests::the_script_names_both_halves_of_the_round_trip ... ok
test serve::tests::a_reload_is_served_the_same_page_again ... ok
test serve::tests::an_answer_over_the_cap_is_refused_and_the_page_keeps_waiting ... ok
test serve::tests::an_answer_that_is_not_the_envelope_is_refused_and_the_page_keeps_waiting ... ok
test serve::tests::an_answer_without_the_token_is_refused_and_the_page_keeps_waiting ... ok
test serve::tests::the_page_is_served_at_its_token_with_the_script_in_it ... ok
test server::tests::a_body_over_the_cap_is_refused_before_it_is_read ... ok
test server::tests::a_bound_port_is_a_real_one_and_a_range_lands_inside_itself ... ok
test server::tests::a_connection_that_says_nothing_asks_for_nothing ... ok
test server::tests::a_get_arrives_as_its_method_and_target_and_is_answered ... ok
test server::tests::a_head_over_the_cap_is_refused ... ok
test server::tests::a_head_that_never_ends_is_refused_when_the_client_goes ... ok
test server::tests::a_post_arrives_with_the_body_its_length_declared ... ok
test server::tests::a_range_with_nothing_free_says_which_ports_were_tried ... ok
test token::tests::a_token_is_43_url_safe_characters_and_never_repeats ... ok
test token::tests::a_token_matches_itself_and_nothing_else ... ok
test token::tests::a_token_never_prints_itself ... ok

test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/lib.rs (target/debug/deps/bingo_mcp-354ed32da33586a8)

running 90 tests
test auth::tests::a_401_in_the_chain_is_read_by_type_and_anything_else_is_not ... ok
test auth::tests::a_child_process_signs_in_to_nothing_here ... ok
test auth::tests::a_server_a_person_already_authorized_takes_no_sign_in ... ok
test auth::tests::an_http_server_with_no_authorization_of_its_own_can_be_signed_in_to ... ok
test auth::tests::only_the_http_servers_that_can_be_signed_in_to_get_a_handle ... ok
test auth::tests::the_bearer_joins_the_dial_and_not_the_configured_row ... ok
test client::tests::a_guard_holds_the_door_for_one_call_and_no_longer ... ok
test client::tests::a_question_with_nothing_of_ours_in_flight_reaches_nobody ... ok
test client::tests::the_handshake_declares_both_modes_of_elicitation ... ok
test command::tests::a_row_says_what_a_server_is_doing_and_how_much_it_gave ... ok
test command::tests::a_sign_in_nobody_answered_names_the_headless_way_through ... ok
test command::tests::a_verb_nobody_defined_is_refused_with_the_ones_that_exist ... ok
test command::tests::a_verb_that_signs_in_refuses_a_server_that_signs_in_to_nothing ... ok
test command::tests::a_verb_without_a_server_or_with_two_is_refused ... ok
test command::tests::every_verb_takes_one_server ... ok
test command::tests::nothing_asks_for_the_table ... ok
test command::tests::the_auth_column_says_where_the_sign_in_stands ... ok
test command::tests::the_spec_holds_the_queue_and_names_every_verb ... ok
test command::tests::the_word_after_a_verb_is_a_configured_server ... ok
test config::tests::a_field_of_the_other_transport_is_refused_rather_than_ignored ... ok
test config::tests::a_misspelled_field_is_a_config_error ... ok
test config::tests::a_row_carries_what_dialling_the_server_takes ... ok
test config::tests::a_row_written_back_out_reads_as_the_server_it_came_from ... ok
test config::tests::a_server_prints_the_names_of_its_secrets_and_not_their_values ... ok
test config::tests::a_server_that_names_no_client_id_forwards_no_oauth_key ... ok
test config::tests::a_stdio_entry_without_a_command_says_so ... ok
test config::tests::a_transport_nobody_implements_is_a_config_error ... ok
test config::tests::an_empty_slice_configures_no_servers ... ok
test config::tests::an_entry_without_a_type_is_a_child_process ... ok
test config::tests::an_http_entry_carries_its_url_and_headers ... ok
test config::tests::an_http_entry_may_name_the_client_id_it_signs_in_as ... ok
test config::tests::an_http_entry_without_a_url_says_so ... ok
test config::tests::deadline_setting_is_positive_and_round_trips_on_both_transports ... ok
test config::tests::deadline_settings_schema_is_pinned ... ok
test config::tests::the_slice_reads_both_claimed_keys ... ok
test dial::tests::a_command_that_does_not_exist_fails_without_waiting ... ok
test dial::tests::a_failed_http_dial_never_reports_a_credential_from_the_url ... ok
test dial::tests::a_header_this_crate_cannot_use_is_named_without_its_value ... ok
test dial::tests::a_log_the_process_cannot_open_sends_stderr_to_the_void_not_the_screen ... ok
test dial::tests::a_readable_header_reaches_the_transport ... ok
test dial::tests::a_server_logs_its_stderr_under_the_data_directory ... ok
test dial::tests::a_url_in_a_failure_keeps_its_endpoint_and_drops_its_query ... ok
test dial::tests::a_url_without_a_query_is_left_as_it_is ... ok
test dial::tests::catalogue_order_preserves_duplicate_definitions_and_precedence ... ok
test dial::tests::dropping_a_server_closes_a_grandchilds_stdout ... ok
test elicitation::tests::a_number_is_checked_before_it_is_sent_back ... ok
test elicitation::tests::a_question_the_person_left_is_left_out_and_a_required_one_declines ... ok
test elicitation::tests::a_shape_the_spec_does_not_allow_declines_the_whole_request ... ok
test elicitation::tests::a_url_request_shows_the_whole_address_and_asks_before_anything_opens ... ok
test elicitation::tests::an_enum_is_a_choice_and_a_boolean_is_two_options ... ok
test elicitation::tests::every_property_is_a_question_wearing_its_own_words ... ok
test elicitation::tests::leaving_the_card_cancels_and_anything_else_declines ... ok
test elicitation::tests::one_string_property_is_one_question_answered_in_words ... ok
test elicitation::tests::the_card_names_the_server_that_is_asking ... ok
test elicitation::tests::what_the_person_answered_is_the_content_the_server_receives ... ok
test elicitation::tests::words_beside_the_ticks_are_what_reaches_the_server ... ok
test guide::tests::the_page_is_one_line_of_description_and_a_body_worth_reading ... ok
test guide::tests::the_page_names_every_command_and_settings_key_this_plugin_registers ... ok
test guide::tests::the_page_names_every_verb_of_the_command ... ok
test guide::tests::the_page_spells_a_tool_name_the_way_this_crate_mints_one ... ok
test guide::tests::the_settings_examples_are_toml_and_no_json_is_left ... ok
test manager::tests::a_child_process_has_no_sign_in_of_its_own ... ok
test manager::tests::a_configured_server_is_on_its_way_before_anything_is_dialled ... ok
test manager::tests::a_disabled_server_starts_switched_off_and_is_never_dialled ... ok
test manager::tests::a_handshake_of_the_current_dial_is_filed ... ok
test manager::tests::a_handshake_that_lands_after_a_disable_is_dropped ... ok
test manager::tests::a_mid_session_401_on_a_server_with_no_sign_in_starts_nothing ... ok
test manager::tests::a_server_that_answered_401_is_filed_as_needing_authentication ... ok
test manager::tests::a_switched_off_server_is_not_reconnected ... ok
test manager::tests::only_a_configured_name_is_known ... ok
test manager::tests::shutting_down_leaves_no_server_offering_tools ... ok
test manager::tests::switching_a_server_off_and_on_says_what_changed ... ok
test rows::tests::a_method_this_service_does_not_speak_says_which_one_it_does ... ok
test rows::tests::a_server_that_is_switched_off_is_not_answered ... ok
test rows::tests::the_service_answers_the_rows_as_a_person_wrote_them ... ok
test tests::a_plugin_that_registered_nothing_starts_and_stops_quietly ... ok
test tests::a_server_this_crate_cannot_read_stops_the_plugin_rather_than_being_dropped ... ok
test tests::the_claimed_schema_describes_both_keys ... ok
test tests::the_manifest_says_what_it_provides_and_what_it_claims ... ok
test tests::the_plugin_registers_a_tool_source_a_command_and_the_rows ... ok
test tool::tests::a_name_is_its_two_parts_and_nothing_else ... ok
test tool::tests::a_name_that_already_holds_the_separator_is_still_copied ... ok
test tool::tests::a_picture_wider_than_the_box_comes_back_inside_it ... ok
test tool::tests::a_tool_is_named_for_its_server_and_itself ... ok
test tool::tests::arguments_are_the_object_the_model_sent ... ok
test tool::tests::bytes_a_server_called_a_picture_and_are_not_are_refused_in_words ... ok
test tool::tests::is_error_crosses_the_boundary ... ok
test tool::tests::text_and_images_pass_through_and_anything_else_arrives_as_json ... ok
test tool::tests::the_catalogue_learns_which_server_a_tool_came_from ... ok
test tool::tests::the_dialect_marker_leaves_the_schema_and_the_rest_stays ... ok

test result: ok. 90 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.14s

     Running tests/server.rs (target/debug/deps/server-51519ad934a3bfaa)

running 19 tests
test a_call_the_turn_cancelled_never_waits_for_the_server ... ok
test a_call_with_nobody_to_ask_declines ... ok
test a_child_is_spawned_in_the_directory_and_environment_it_was_given ... ok
test a_connected_server_s_tools_carry_its_name_and_its_schema ... ok
test a_disabled_server_is_enabled_before_it_is_dialled_again ... ok
test a_server_nobody_configured_is_refused ... ok
test a_server_s_stderr_lands_in_its_log_and_never_on_the_screen ... ok
test a_server_that_answers_with_is_error_says_so ... ok
test a_server_that_hangs_holds_up_neither_the_others_nor_the_source ... ok
test a_server_that_never_answers_fails_at_the_deadline ... ok
test a_servers_question_becomes_a_form_card_and_the_answer_reaches_it ... ok
test an_mcp_tool_never_earns_a_trait_the_gate_would_trust ... ok
test calling_a_tool_returns_what_the_server_answered ... ok
test deadline_bounds_an_mcp_call ... ok
test deadline_uses_the_server_override ... ok
test disabling_a_server_takes_its_tools_out_of_the_source ... ok
test leaving_the_card_cancels_and_an_unanswered_requirement_declines ... ok
test reconnecting_dials_the_server_again ... ok
test the_table_says_what_every_server_is_doing ... ok

test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.43s

     Running tests/signing_in.rs (target/debug/deps/signing_in-d519c73ff580149e)

running 8 tests
test a_call_refused_mid_session_renews_and_dials_again ... mbx[progress]: 225s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test a_connected_server_lists_what_it_offers ... ok
test a_failed_reconnect_drops_the_previously_connected_catalogue ... ok
test a_header_a_person_wrote_themselves_fails_rather_than_needing_a_sign_in ... ok
test a_renewal_that_fails_leaves_the_server_needing_authentication ... ok
test a_server_that_wants_a_bearer_nobody_has_needs_authentication ... ok
test a_stored_sign_in_is_the_bearer_the_dial_carries ... ok
test catalogue_order_survives_reconnect_and_tools_remain_callable ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s

     Running unittests src/lib.rs (target/debug/deps/bingo_permissions-dd2c509a44b86d7a)

running 103 tests
test command::tests::a_mode_chosen_in_one_session_decides_that_session_only ... ok
test command::tests::a_mode_nobody_defined_is_refused_and_changes_nothing ... ok
test command::tests::bypassing_the_gate_is_a_mode_like_any_other ... ok
test command::tests::no_argument_names_this_session_s_mode_and_the_five_to_choose_from ... ok
test command::tests::the_session_s_mode_leaves_the_configured_one_alone ... ok
test command::tests::the_spec_runs_now_and_takes_a_mode ... ok
test decide::tests::a_compound_command_gets_no_session_scope ... ok
test decide::tests::a_deny_rule_beats_the_mode_that_would_have_allowed ... ok
test decide::tests::a_deny_rule_matches_any_sub_command ... ok
test decide::tests::a_path_rule_normalises_before_it_denies ... ok
test decide::tests::a_plain_non_read_only_tool_across_every_mode ... ok
test decide::tests::a_prompt_that_outranks_allow_rules_offers_no_scope ... ok
test decide::tests::a_read_of_a_sensitive_path_is_still_a_read ... ok
test decide::tests::a_sensitive_relative_path_resolves_from_the_session_cwd ... ok
test decide::tests::a_session_scope_stops_the_gate_asking_again ... ok
test decide::tests::a_skill_rule_matches_exactly_or_by_prefix ... ok
test decide::tests::a_tools_own_confirmation_asks_in_every_mode ... ok
test decide::tests::a_trusted_read_only_tool_is_allowed_in_every_mode ... ok
test decide::tests::a_url_tool_asks_until_a_domain_rule_names_its_host ... ok
test decide::tests::a_write_asks_by_default_and_is_accepted_by_accept_edits ... ok
test decide::tests::a_write_into_a_sensitive_path_asks_in_every_mode ... ok
test decide::tests::accept_edits_does_not_reach_a_sensitive_path ... ok
test decide::tests::accept_edits_needs_a_path_to_judge ... ok
test decide::tests::accept_edits_stops_at_the_edge_of_the_working_directories ... ok
test decide::tests::an_allow_rule_lets_bash_run_in_default_mode ... ok
test decide::tests::an_allow_rule_needs_every_sub_command ... ok
test decide::tests::an_ask_rule_outranks_an_allow_rule ... ok
test decide::tests::an_ask_rule_prompt_offers_no_session_scope ... ok
test decide::tests::an_ask_rule_survives_bypass_and_a_deny_rule_still_wins ... ok
test decide::tests::an_mcp_tools_own_read_only_claim_never_reaches_the_gate ... ok
test decide::tests::dont_ask_denies_even_the_prompts_no_mode_can_silence ... ok
test decide::tests::plan_denies_everything_that_is_not_read_only ... ok
test decide::tests::the_colon_star_suffix_is_a_prefix_as_one_piece ... ok
test guide::tests::the_page_is_one_line_of_description_and_a_body_worth_reading ... ok
test guide::tests::the_page_names_every_command_this_plugin_registers ... ok
test guide::tests::the_page_names_every_mode_this_plugin_has ... ok
test guide::tests::the_page_names_every_settings_key_this_plugin_claims ... ok
test mode::tests::a_mode_reads_back_from_its_own_name ... ok
test mode::tests::an_unknown_mode_is_an_error_that_names_the_choices ... ok
test mode::tests::every_mode_says_what_it_does ... ok
test mode::tests::json_and_the_command_line_spell_a_mode_the_same_way ... ok
test mode::tests::only_the_mode_that_runs_everything_answers_a_question_for_the_person ... ok
test mode::tests::the_default_mode_is_default ... ok
test path::tests::a_relative_path_resolves_against_the_session_cwd ... ok
test path::tests::a_sensitive_directory_is_recognised_anywhere_in_the_path ... ok
test path::tests::a_tilde_expands_to_the_home_directory ... ok
test path::tests::a_trailing_separator_survives ... ok
test path::tests::containment_stops_at_a_component_boundary ... ok
test path::tests::normalising_twice_changes_nothing ... ok
test path::tests::parent_components_collapse_without_touching_the_disk ... ok
test path::tests::without_a_home_a_tilde_is_just_a_name ... ok
test rule::tests::a_bare_tool_rule_covers_every_call_of_that_tool ... ok
test rule::tests::a_bash_rule_is_a_prefix_of_the_command ... ok
test rule::tests::a_domain_rule_matches_the_host_and_only_the_host ... ok
test rule::tests::a_name_rule_is_exact_unless_it_says_prefix ... ok
test rule::tests::a_path_rule_normalises_before_it_matches ... ok
test rule::tests::a_path_rule_written_as_a_glob_matches_as_one ... ok
test rule::tests::a_quoted_separator_is_not_a_sub_command ... ok
test rule::tests::a_rule_that_names_something_never_covers_a_call_that_names_nothing ... ok
test rule::tests::a_star_says_no_more_than_the_bare_tool_name ... ok
test rule::tests::a_url_rule_without_domain_is_a_prefix ... ok
test rule::tests::a_word_boundary_narrows_allow_without_narrowing_deny ... ok
test rule::tests::allow_needs_every_sub_command_and_deny_takes_any ... ok
test rule::tests::an_mcp_server_rule_covers_its_tools_and_a_tool_rule_covers_one ... ok
test rule::tests::an_unreadable_line_is_not_a_rule ... ok
test rule::tests::an_untrusted_parse_is_covered_by_no_allow_rule ... ok
test rule::tests::the_first_matching_rule_of_a_table_is_the_one_reported ... ok
test scope::tests::a_call_that_names_nothing_is_scoped_to_the_tool ... ok
test scope::tests::a_call_that_names_two_things_has_no_narrowest_rule ... ok
test scope::tests::a_compound_or_unreadable_command_has_no_scope ... ok
test scope::tests::a_file_at_the_root_has_no_directory_worth_scoping_to ... ok
test scope::tests::a_file_tool_is_scoped_to_the_directory_it_touches ... ok
test scope::tests::a_simple_command_is_scoped_to_its_first_word ... ok
test scope::tests::a_url_is_scoped_to_its_host_and_a_name_to_itself ... ok
test session::tests::a_line_the_grammar_cannot_read_installs_nothing ... ok
test session::tests::a_mode_and_an_accepted_rule_live_side_by_side ... ok
test session::tests::a_session_runs_in_the_last_mode_it_was_given ... ok
test session::tests::an_accepted_rule_is_there_for_the_next_call ... ok
test session::tests::one_sessions_answer_does_not_speak_for_another ... ok
test session::tests::the_same_answer_twice_is_still_one_rule ... ok
test split::tests::a_redirection_stays_with_its_command ... ok
test split::tests::a_simple_command_is_one_part ... ok
test split::tests::a_substituted_command_keeps_the_command_that_contains_it ... ok
test split::tests::an_unterminated_quote_is_never_trusted ... ok
test split::tests::deep_nesting_does_not_overflow_the_stack ... ok
test split::tests::every_sequencing_operator_splits ... ok
test split::tests::nothing_to_run_is_no_part_at_all ... ok
test split::tests::quoted_separators_are_not_operators ... ok
test split::tests::subshells_and_substitutions_yield_their_inner_commands ... ok
test tests::a_denial_installs_nothing ... ok
test tests::a_misspelled_key_is_a_startup_failure_not_a_silence ... ok
test tests::an_accepted_scope_silences_the_next_call_of_that_session_only ... ok
test tests::an_empty_slice_is_the_default_mode_and_no_rules ... ok
test tests::an_unreadable_rule_is_a_startup_failure ... ok
test tests::describe_names_the_mode_and_the_rules_a_session_accepted ... ok
test tests::slots_place_the_mode_and_each_action ... ok
test tests::the_manifest_says_what_it_provides ... ok
test tests::the_plugin_registers_one_policy_and_the_command_that_reads_it ... ok
test tests::the_slice_carries_the_mode_the_rules_and_the_extra_directories ... ok
test tests::the_stance_is_the_mode_this_session_chose_and_no_other_session_s ... ok
test url::tests::a_backslash_ends_the_authority ... ok
test url::tests::a_host_this_parser_cannot_read_is_no_host_at_all ... ok
test url::tests::a_plain_url_yields_its_host ... ok

test result: ok. 103 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.81s

     Running tests/properties.rs (target/debug/deps/properties-1904e3ae28affc0e)

running 6 tests
test a_deny_rule_beats_the_same_line_in_the_allow_table ... ok
test a_write_into_a_sensitive_path_asks_under_bypass ... ok
test an_unreadable_command_is_never_allowed ... ok
test dont_ask_never_asks ... ok
test normalising_a_path_twice_changes_nothing ... ok
test plan_never_allows_a_tool_that_is_not_read_only ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s

     Running unittests src/lib.rs (target/debug/deps/bingo_persona-51e1789fec4d5ac7)

running 18 tests
test guide::tests::every_settings_example_is_toml_and_none_is_json ... ok
test guide::tests::the_page_is_one_line_of_description_and_a_body_worth_reading ... ok
test guide::tests::the_page_names_every_field_under_the_key ... ok
test guide::tests::the_page_names_the_settings_key_this_plugin_claims ... ok
test guide::tests::the_page_quotes_the_line_the_stance_is_pinned_by ... ok
test guide::tests::the_page_shows_the_key_the_silence_and_the_off_switch ... ok
test judgement::tests::a_persons_own_words_are_the_whole_block ... ok
test judgement::tests::an_empty_text_is_no_block_at_all ... ok
test judgement::tests::it_speaks_after_the_kernel_and_before_the_project ... ok
test judgement::tests::the_one_block_is_cacheable ... ok
test judgement::tests::the_stance_says_when_to_speak_who_decides_and_what_is_too_small_to_raise ... ok
test judgement::tests::the_stance_stays_under_a_thousand_characters ... ok
test tests::a_typo_under_the_key_stops_the_host_and_names_the_field ... ok
test tests::a_written_setting_replaces_the_whole_stance ... ok
test tests::an_empty_setting_leaves_the_plugin_on_and_silent ... ok
test tests::an_unwritten_setting_leaves_the_shipped_stance ... ok
test tests::registering_contributes_the_block_and_the_page_the_manifest_promises ... ok
test tests::the_manifest_says_what_it_provides_and_claims_one_setting ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/bingo_pictures-bd9466ffeefa94c5)

running 68 tests
test accepted::tests::a_handed_over_picture_nothing_reads_is_refused ... ok
test accepted::tests::a_handed_over_picture_of_a_wider_type_becomes_png ... ok
test accepted::tests::a_handed_over_picture_over_the_budget_is_bounded_and_keeps_its_file ... ok
test accepted::tests::a_handed_over_picture_the_table_takes_passes_through_untouched ... ok
test accepted::tests::a_picture_over_the_budget_is_sniffed_into_the_bound ... ok
test accepted::tests::a_type_the_table_takes_is_sniffed_and_kept_as_it_came ... ok
test accepted::tests::a_wider_type_is_decoded_and_sent_as_png ... ok
test accepted::tests::bytes_no_decoder_recognises_are_not_a_picture ... ok
test bounded::tests::a_bounded_picture_fed_back_is_untouched ... ok
test bounded::tests::a_budget_no_quality_reaches_shrinks_the_picture_and_tries_again ... ok
test bounded::tests::a_picture_inside_the_box_and_under_the_budget_is_the_bytes_it_came_as ... ok
test bounded::tests::a_picture_larger_than_the_box_is_fitted_to_it_with_its_shape_kept ... ok
test bounded::tests::a_picture_no_png_fits_comes_back_jpeg_under_the_budget ... ok
test bounded::tests::a_picture_with_alpha_comes_back_flattened_on_white ... ok
test bounded::tests::a_type_the_table_refuses_is_re_encoded_even_inside_the_box ... ok
test bounded::tests::bytes_no_decoder_reads_are_not_bounded_into_a_picture ... ok
test bounded::tests::the_ladder_gives_up_when_even_the_last_shrink_is_over_budget ... ok
test cache::tests::a_cache_of_no_days_is_no_cache_at_all ... ok
test cache::tests::a_directory_that_will_not_be_made_is_no_ones_error ... ok
test cache::tests::a_life_no_clock_could_measure_is_kept_rather_than_wrapped ... ok
test cache::tests::an_address_names_one_entry_and_no_other_address_names_it ... ok
test cache::tests::an_entry_is_fresh_until_its_time_is_up ... ok
test cache::tests::an_entry_past_its_time_is_a_miss_and_is_gone ... ok
test cache::tests::keeping_an_address_again_replaces_what_was_there ... ok
test cache::tests::what_is_kept_is_read_back_and_nothing_is_left_beside_it ... ok
test file::tests::a_kept_picture_is_named_by_its_bytes_and_read_back_whole ... ok
test file::tests::a_write_makes_its_directory_and_leaves_nothing_on_failure ... ok
test load::tests::a_bmp_on_disk_becomes_a_png ... ok
test load::tests::a_fetch_that_failed_leaves_no_entry_behind ... ok
test load::tests::a_file_on_this_machine_is_never_copied_into_the_cache ... ok
test load::tests::a_file_over_the_budget_is_read_as_the_picture_a_model_is_sent ... ok
test load::tests::a_file_over_the_cap_is_refused_before_it_is_read ... ok
test load::tests::a_file_that_is_not_a_picture_is_refused_whatever_it_is_called ... ok
test load::tests::a_jpeg_over_http_is_fetched_by_this_machine_and_passed_through ... ok
test load::tests::a_page_at_the_end_of_a_url_is_not_a_picture ... ok
test load::tests::a_path_that_is_not_there_says_so ... ok
test load::tests::a_png_on_disk_is_read_and_passed_through ... ok
test load::tests::a_tiff_over_http_becomes_a_png_whatever_the_header_claims ... ok
test load::tests::a_url_over_the_budget_is_fetched_and_bounded ... ok
test load::tests::a_url_over_the_cap_is_refused ... ok
test load::tests::a_url_read_twice_is_fetched_once ... ok
test load::tests::a_url_that_is_not_there_says_so_and_names_no_url_twice ... ok
test load::tests::an_entry_past_its_time_is_fetched_again_and_rewritten ... ok
test load::tests::no_days_writes_nothing_and_fetches_every_time ... ok
test load::tests::the_cap_refuses_a_size_known_before_the_bytes_are ... ok
test pixels::tests::a_png_decodes_to_four_bytes_a_pixel ... ok
test pixels::tests::a_sample_outside_the_picture_is_transparent_black ... ok
test pixels::tests::bytes_no_decoder_reads_are_refused ... ok
test source::tests::a_short_or_wide_word_is_a_path ... ok
test source::tests::a_word_with_a_scheme_is_a_url_and_everything_else_is_a_path ... ok
test source::tests::an_absolute_path_is_itself_and_a_relative_one_is_under_the_directory ... ok
test source::tests::every_extension_a_decoder_knows_names_a_picture_and_a_url_always_does ... ok
test source::tests::prose_and_a_source_file_name_no_picture ... ok
test source::tests::the_scheme_is_read_case_insensitively_and_nothing_near_it_counts ... ok
test tests::a_box_of_nothing_still_leaves_a_pixel ... ok
test tests::a_gif_is_its_first_frame ... ok
test tests::a_jpeg_inside_its_box_becomes_a_png_of_its_own_size ... ok
test tests::a_jpeg_is_decoded_and_written_back_out_as_a_png ... ok
test tests::a_payload_no_decoder_reads_has_no_size_and_will_not_fit ... ok
test tests::a_payload_no_decoder_reads_is_an_error_and_not_a_guess ... ok
test tests::a_payload_that_is_not_base64_says_so ... ok
test tests::a_picture_already_inside_its_box_is_handed_back_as_it_came ... ok
test tests::a_picture_larger_than_the_head_is_measured_from_its_head ... ok
test tests::a_picture_too_big_for_its_box_is_shrunk_to_fit_it ... ok
test tests::a_pictures_size_is_read_off_its_header_whatever_it_is ... ok
test tests::a_png_passes_through_with_the_size_off_its_header ... ok
test tests::only_the_pixels_the_cells_show_are_sent ... ok
test tests::the_size_comes_off_the_header_and_nothing_else_answers ... ok

test result: ok. 68 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.11s

     Running unittests src/lib.rs (target/debug/deps/bingo_plugin_rpc-8999ad51b99d38bf)

running 145 tests
test bridge::tests::a_run_of_failures_backs_off_and_then_stops_growing ... ok
test bridge::tests::protocol_one_is_refused_with_both_numbers ... ok
test bridge::tests::the_first_attempt_after_a_death_waits_for_nothing ... ok
test bridge::tests::this_host_speaks_one_major_and_refuses_the_rest ... ok
test codec::tests::a_notification_has_no_id ... ok
test codec::tests::a_request_round_trips ... ok
test codec::tests::a_result_and_an_error_are_never_both_present ... ok
test codec::tests::absent_params_parse_as_null ... ok
test codec::tests::another_version_is_not_a_message ... ok
test compactor::tests::a_compactor_past_its_deadline_fails_the_call ... ok
test compactor::tests::the_remote_context_carries_the_native_parent_request ... ok
test config::tests::a_person_with_no_plugins_key_has_no_slices ... ok
test config::tests::a_slice_reaches_the_plugin_it_is_named_for ... ok
test connection::tests::a_command_that_does_not_exist_says_so_rather_than_pretending ... ok
test connection::tests::a_delta_reaches_the_stream_it_names_and_no_other ... ok
test connection::tests::a_line_that_is_not_a_message_is_ignored_not_fatal ... ok
test connection::tests::a_log_the_process_cannot_open_sends_stderr_to_the_void_not_the_screen ... ok
test connection::tests::a_plugin_logs_its_stderr_under_the_data_directory ... ok
test connection::tests::a_process_that_ends_ends_every_stream_it_was_feeding ... ok
test connection::tests::a_progress_line_reaches_the_call_it_names_and_no_other ... ok
test connection::tests::a_request_the_host_does_not_serve_is_refused_and_the_reader_goes_on ... ok
test connection::tests::a_request_whose_process_ends_fails_instead_of_hanging ... ok
test connection::tests::an_unserved_request_gets_method_not_found ... ok
test connection::tests::dropping_a_connection_closes_a_grandchilds_stdout ... ok
test connection::tests::notifications_after_handshake_are_not_overwritten_by_initial_lists ... ok
test connection::tests::one_process_mints_one_series_of_stream_names ... ok
test contributor::tests::a_contributor_is_named_for_its_plugin_and_itself ... ok
test contributor::tests::a_contributor_past_its_deadline_contributes_nothing_and_says_whose ... ok
test contributor::tests::the_placement_is_the_one_the_handshake_declared ... ok
test deadline::tests::the_hot_path_has_the_shortest_deadline_of_them_all ... ok
test discovery::tests::a_directory_with_no_manifest_is_somebody_else_s ... ok
test discovery::tests::a_home_that_is_the_working_directory_is_read_once ... ok
test discovery::tests::a_layer_that_is_not_there_finds_nothing_and_says_nothing ... ok
test discovery::tests::a_manifest_that_names_another_plugin_is_refused ... ok
test discovery::tests::a_manifest_that_will_not_parse_is_reported_not_skipped ... ok
test discovery::tests::explicit_directories_win_by_name_in_argument_order ... ok
test discovery::tests::the_person_s_own_plugins_come_before_the_project_s ... ok
test discovery::tests::the_project_wins_a_name_the_person_also_installed ... ok
test discovery::tests::what_is_found_is_the_directory_and_the_manifest_in_it ... ok
test doors::tests::a_call_that_has_ended_is_refused_in_words ... ok
test doors::tests::a_level_outside_the_kernel_s_three_is_not_a_notice_at_all ... ok
test doors::tests::a_line_that_is_not_what_the_door_takes_is_refused_in_words ... ok
test doors::tests::a_method_the_host_does_not_speak_is_refused_with_the_set_it_speaks ... ok
test doors::tests::a_notice_carries_a_level_and_a_line_and_answers_nothing ... ok
test doors::tests::a_notice_needs_no_call_and_lands_on_the_one_channel ... ok
test doors::tests::a_permission_prompt_is_not_a_question_a_plugin_may_open ... ok
test doors::tests::a_process_may_write_a_question_and_the_door_is_what_refuses_the_rest ... ok
test doors::tests::a_question_on_a_running_call_comes_back_as_the_person_answered ... ok
test doors::tests::an_answer_crosses_as_the_sdk_writes_it ... ok
test doors::tests::an_ask_carries_the_running_call_and_the_sdk_s_own_question ... ok
test doors::tests::another_connection_s_running_call_is_not_this_caller_s_to_ask_on ... ok
test doors::tests::the_answers_a_question_takes_are_read_off_the_question ... ok
test doors::tests::the_host_s_own_face_runs_no_call_and_says_so ... ok
test doors::tests::the_host_speaks_two_doors_and_no_more ... ok
test doors::tests::the_question_that_crosses_is_the_sdk_s_own ... ok
test events_ts::tests::committed_events_are_generated ... ok
test hook::tests::a_hook_is_named_for_its_plugin_and_itself ... ok
test hook::tests::an_observation_returns_at_once_however_silent_the_process_is ... ok
test hook::tests::the_matcher_is_the_one_the_handshake_declared ... ok
test intercept::catalogue_contract::initialization_retains_open_catalogue_subscriptions ... ok
test intercept::tests::catalogue_tool_fields_and_completion_spelling ... ok
test manager::tests::a_home_and_a_project_with_no_plugins_directory_start_quietly ... ok
test manager::tests::a_manager_that_has_discovered_nothing_answers_nothing ... ok
test manager::tests::a_plugin_nobody_switched_off_is_spawned_and_answers_for_itself ... ok
test manager::tests::a_switched_off_plugin_is_listed_and_never_spawned ... ok
test manifest::tests::a_key_this_host_does_not_know_does_not_refuse_the_manifest ... ok
test manifest::tests::a_manifest_is_the_four_keys_the_adr_names ... ok
test manifest::tests::a_value_that_names_the_root_twice_has_both_resolved ... ok
test manifest::tests::an_entry_that_names_no_root_is_left_as_it_was ... ok
test manifest::tests::deadline_manifest_round_trips_the_override ... ok
test manifest::tests::the_root_reaches_the_command_the_arguments_and_the_environment ... ok
test notice::tests::a_line_nobody_is_open_to_hear_waits_where_it_was ... ok
test notice::tests::a_notice_becomes_the_transcript_body_a_surface_renders ... ok
test notice::tests::a_notice_waits_until_it_is_drained_and_then_is_gone ... ok
test notice::tests::a_plugin_s_own_line_is_said_under_the_name_it_is_installed_as ... ok
test notice::tests::a_plugin_that_dies_in_a_loop_keeps_the_newest_not_the_oldest ... ok
test provider::tests::a_model_the_declaration_never_named_can_do_nothing ... ok
test provider::tests::a_provider_is_what_the_handshake_declared_and_no_more ... ok
test provider::tests::a_provider_that_named_no_family_is_filed_under_its_own_id ... ok
test provider::tests::a_stream_that_hears_nothing_gives_up_at_the_deadline ... ok
test reload::tests::every_stamp_dimension_and_deletion_are_changes ... ok
test schema::tests::every_named_reference_resolves ... ok
test schema::tests::every_property_is_camel_case ... ok
test schema::tests::protocol_two_keeps_every_frozen_shape ... ok
test schema::tests::protocol_two_names_the_new_doors ... ok
test schema::tests::the_committed_schema_is_this_document ... ok
test schema::tests::the_committed_schema_names_every_method_and_every_notification ... ok
test schema::tests::the_committed_schema_names_the_host_s_own_service_and_its_doors ... ok
test schema::tests::the_freeze_rejects_removal_and_retyping_but_allows_additions ... ok
test schema::tests::the_sdk_types_a_plugin_answers_with_are_all_defined ... ok
test service::tests::a_call_the_host_can_route_comes_back_as_the_service_answered ... ok
test service::tests::a_declaration_names_one_service_per_key ... ok
test service::tests::a_key_nobody_holds_is_refused_in_words ... ok
test service::tests::a_line_that_is_not_a_service_call_is_refused_as_bad_params ... ok
test service::tests::a_method_the_declaration_never_named_is_refused_with_the_set_it_speaks ... ok
test service::tests::a_service_that_named_no_method_speaks_nothing ... ok
test service::tests::a_service_that_says_nothing_gives_up_at_the_deadline_and_names_the_plugin ... ok
test service::tests::a_service_whose_plugin_is_gone_refuses_in_words ... ok
test service::tests::a_service_with_no_wire_face_does_not_exist_to_a_process ... ok
test state::tests::compare_and_swap_only_accepts_the_held_version ... ok
test tests::a_host_with_no_plugins_starts_and_stops_quietly ... ok
test tests::the_claimed_schema_describes_the_key ... ok
test tests::the_manifest_says_what_it_provides_and_what_it_claims ... ok
test tests::the_plugin_registers_one_source_of_every_kind_it_bridges ... ok
test tool::tests::a_display_this_host_has_no_word_for_folds_instead_of_failing ... ok
test tool::tests::a_name_that_already_holds_the_separator_is_still_copied ... ok
test tool::tests::a_three_megabyte_plugin_picture_is_bounded_before_the_model_sees_it ... ok
test tool::tests::a_tool_is_named_for_its_plugin_and_itself ... ok
test tool::tests::an_unreadable_plugin_picture_becomes_a_reasoned_text_part ... ok
test tool::tests::the_catalogue_learns_which_plugin_a_tool_came_from ... ok
test validate::tests::missing_entry_and_bad_name_name_the_field ... ok
test validate::tests::protocol_one_names_both_numbers ... ok
test validate::tests::subscriptions_accept_every_catalogue_entry_and_dynamic_patterns ... ok
test wire::tests::a_call_carries_the_ids_the_kernel_minted ... ok
test wire::tests::a_call_may_take_nothing_and_an_answer_may_be_nothing ... ok
test wire::tests::a_cancel_names_the_stream_it_stops_and_no_other ... ok
test wire::tests::a_compaction_comes_back_as_the_sdk_writes_it ... ok
test wire::tests::a_compaction_is_asked_for_with_the_reason_the_trait_speaks ... ok
test wire::tests::a_contributor_may_answer_with_no_pieces_at_all ... ok
test wire::tests::a_declared_compaction_failure_preserves_native_usage_in_its_tagged_fixture ... ok
test wire::tests::a_declared_contributor_carries_the_placement_it_speaks_at ... ok
test wire::tests::a_declared_hook_carries_the_matcher_the_kernel_skips_on ... ok
test wire::tests::a_declared_provider_carries_its_models_and_what_its_endpoint_does ... ok
test wire::tests::a_declared_service_names_the_methods_it_speaks_and_their_schemas ... ok
test wire::tests::a_delta_carries_one_sdk_event_and_the_stream_it_belongs_to ... ok
test wire::tests::a_finish_crosses_as_the_sdk_writes_it ... ok
test wire::tests::a_handshake_may_name_no_tools_and_no_commands ... ok
test wire::tests::a_hook_that_names_no_point_wants_them_all ... ok
test wire::tests::a_piece_crosses_as_the_sdk_writes_it ... ok
test wire::tests::a_placement_this_host_does_not_know_is_refused_in_words ... ok
test wire::tests::a_point_this_host_does_not_know_is_refused_in_words ... ok
test wire::tests::a_process_that_serves_no_service_says_nothing_about_services ... ok
test wire::tests::a_provider_that_says_only_its_name_is_a_declaration_too ... ok
test wire::tests::a_service_call_is_one_shape_whichever_way_it_travels ... ok
test wire::tests::a_site_crosses_without_the_host_the_context_carries ... ok
test wire::tests::a_stream_closes_with_nothing_or_with_the_error_the_trait_speaks ... ok
test wire::tests::a_stream_opens_with_the_request_the_sdk_writes ... ok
test wire::tests::an_observation_carries_the_phase_and_the_turn_it_watched ... ok
test wire::tests::an_observed_frame_crosses_as_the_journal_writes_it ... ok
test wire::tests::an_outcome_crosses_as_the_sdk_writes_it ... ok
test wire::tests::capabilities_are_only_the_intersection ... ok
test wire::tests::no_name_is_used_twice ... ok
test wire::tests::the_handshake_travels_in_camel_case ... ok
test wire::tests::the_round_crosses_without_the_host_the_query_carries ... ok
test wire_ts::tests::committed_wire_is_generated ... ok

test result: ok. 145 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.54s

     Running tests/plugin/main.rs (target/debug/deps/plugin-4a18b6f0b42c52a7)

running 49 tests
test context::a_plugin_s_compaction_strategy_answers_a_compaction ... ok
test context::a_plugin_s_contributor_speaks_at_the_placement_it_declared ... ok
test context::a_remote_declared_compaction_failure_preserves_each_observed_usage_counter_once ... ok
test doors::a_call_that_is_not_running_is_refused_in_words ... ok
test doors::a_door_the_host_does_not_have_is_answered_with_the_ones_it_does ... ok
test doors::a_permission_prompt_is_not_a_question_a_plugin_may_open ... ok
test doors::a_plugin_s_own_line_surfaces_with_no_tool_call_in_flight ... ok
test doors::a_tool_s_mid_call_question_comes_back_as_the_person_answered ... ok
test doors::another_plugin_s_live_call_is_refused_in_words ... ok
test doors::the_host_s_own_service_is_in_the_registry_under_its_key ... ok
test hooks::a_remote_tool_link_sees_its_result_after_next ... ok
test hooks::an_observation_point_lands_as_a_notification_nobody_waits_on ... ok
test hooks::dropping_a_first_phase_notifies_the_process ... ok
test host::contribution_notifications_replace_the_whole_list ... ok
test host::model_completion_reaches_the_hosts_default_refusal ... ok
test host::reverse_verbs_bind_the_process_identity_and_do_not_wait_for_a_turn ... ok
test host::state_versions_refuse_stale_writes_and_survive_a_restart ... ok
test lifecycle::a_declaration_this_host_cannot_read_refuses_the_handshake_in_words ... ok
test lifecycle::a_killed_process_leaves_one_notice_empty_sources_and_a_working_respawn ... ok
test lifecycle::a_plugin_s_stderr_goes_to_a_log_under_the_data_directory ... ok
test lifecycle::a_plugin_whose_command_is_gone_is_reported_and_contributes_nothing ... ok
test lifecycle::an_unknown_protocol_major_refuses_the_handshake_with_a_notice ... ok
test lifecycle::protocol_one_is_refused_with_both_numbers ... ok
test provider::a_cancelled_stream_ends_and_the_process_is_told ... ok
test provider::a_dropped_stream_tells_the_process_to_stop ... ok
test provider::a_failed_response_crosses_as_the_error_the_trait_speaks ... ok
test provider::a_plugin_s_provider_serves_the_models_it_declared ... ok
test provider::a_process_that_dies_mid_stream_yields_the_error_the_trait_speaks ... ok
test provider::a_tool_round_trip_crosses_as_two_streams ... ok
test provider::a_whole_response_crosses_the_pipe_event_by_event ... ok
test provider::two_streams_at_once_never_mix_their_deltas ... ok
test reload::a_manifest_change_reloads_once_and_keeps_global_state ... ok
test reload::reload_fails_a_held_link_and_reports_changed_services ... ok
test service::a_call_to_a_service_nobody_serves_is_refused_in_words ... ok
test service::a_caller_reads_back_what_it_wrote_across_two_processes ... ok
test service::a_second_plugin_claiming_the_same_key_is_reported_and_the_first_keeps_it ... ok
test service::an_in_process_consumer_reaches_an_external_service_by_key ... ok
test service::an_unknown_method_is_answered_with_the_set_the_service_speaks ... ok
test service::two_processes_pair_on_a_service_through_the_host ... ok
test tools::a_call_crosses_the_pipe_and_the_output_comes_back ... ok
test tools::a_dropped_call_tells_the_plugin_it_is_no_longer_waited_for ... ok
test tools::a_plugin_s_commands_keep_the_name_the_plugin_gave_them ... ok
test tools::a_plugin_s_tools_are_named_for_it_and_are_untrusted ... ok
test tools::a_progress_notification_becomes_the_call_s_live_output_line ... ok
test tools::deadline_bounds_a_plugin_command ... ok
test tools::deadline_bounds_a_plugin_tool_and_abandons_it ... ok
test tools::deadline_uses_the_manifest_for_a_plugin_command ... ok
test tools::deadline_uses_the_manifest_for_a_plugin_tool ... ok
test tools::the_plugin_root_reaches_the_process_through_its_environment ... ok

test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.71s

     Running unittests src/lib.rs (target/debug/deps/bingo_process-e90fc0dabd1631b2)

running 10 tests
test shell::tests::a_missing_override_never_falls_through ... ok
test shell::tests::an_absolute_override_wins_on_either_platform ... ok
test shell::tests::deriving_a_shell_requires_git_to_exist ... ok
test shell::tests::relative_and_empty_overrides_are_rejected_even_if_present ... ok
test shell::tests::the_real_shell_exists_and_the_memoized_answer_agrees ... ok
test shell::tests::unix_prefers_bash_then_sh_and_reports_both_when_missing ... ok
test shell::tests::windows_prefers_the_first_safe_path_shell ... ok
test shell::tests::windows_skips_wsl_case_insensitively_and_by_directory_boundary ... ok
test shell::tests::windows_tries_all_git_installations_before_deriving_from_git ... ok
test tree_contract::dropping_a_tree_closes_a_grandchilds_stdout ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/bingo_provider_acp-bf6acc10e226f218)

running 187 tests
test bridge::handshake::tests::a_first_line_that_never_ends_is_refused_rather_than_buffered ... ok
test bridge::handshake::tests::a_stream_that_says_nothing_is_not_a_conversation ... ok
test bridge::handshake::tests::the_line_is_read_and_the_bytes_behind_it_are_left_alone ... ok
test bridge::handshake::tests::what_the_proxy_writes_is_what_the_listener_reads ... ok
test bridge::offer::tests::a_catalogue_entry_reads_back_as_the_spec_it_was_made_from ... ok
test bridge::offer::tests::a_forwarded_servers_tools_leave_the_offer ... ok
test bridge::offer::tests::a_name_nothing_answers_to_is_reported_not_refused ... ok
test bridge::offer::tests::a_servers_tools_are_found_by_the_fact_not_by_the_name ... ok
test bridge::offer::tests::a_tool_this_crate_never_heard_of_is_offered ... ok
test bridge::offer::tests::an_entry_with_nothing_beside_it_is_still_a_tool ... ok
test bridge::offer::tests::an_explicit_list_replaces_the_derivation_whole ... ok
test bridge::offer::tests::the_machines_own_hands_stay_on_the_agents_side ... ok
test bridge::shape::tests::a_call_carries_its_arguments_under_the_id_the_bridge_minted ... ok
test bridge::shape::tests::a_call_with_no_arguments_is_an_empty_object_not_null ... ok
test bridge::shape::tests::a_refusal_is_an_error_result_in_words ... ok
test bridge::shape::tests::a_schema_that_is_not_an_object_is_offered_as_the_empty_object ... ok
test bridge::shape::tests::a_tool_crosses_as_the_body_a_client_parses ... ok
test bridge::shape::tests::an_answer_crosses_as_content_and_a_failure_says_so ... ok
test bridge::shape::tests::an_image_keeps_its_media_type ... ok
test bridge::shape::tests::the_view_a_person_would_have_seen_does_not_cross ... ok
test bridge::shape::tests::what_a_catalogue_shows_beside_a_tool_does_not_cross ... ok
test bridge::socket::tests::a_dropped_listener_stops_answering ... ok
test bridge::socket::tests::a_run_listens_under_its_own_data_directory ... ok
test bridge::socket::tests::dialling_an_address_nobody_listens_on_fails ... ok
test bridge::socket::tests::two_runs_do_not_share_an_address ... ok
test bridge::socket::tests::what_is_dialled_is_what_was_accepted ... ok
test bridge::token::tests::a_token_is_43_url_safe_characters_and_never_repeats ... ok
test bridge::token::tests::a_token_matches_itself_and_nothing_else ... ok
test bridge::token::tests::a_token_never_prints_itself ... ok
test child::tests::a_command_that_is_not_there_names_itself_and_says_what_is_missing ... ok
test child::tests::a_spawned_adapter_speaks_both_ways_and_dies_with_its_handle ... ok
test config::tests::a_name_that_could_not_be_typed_is_refused ... ok
test config::tests::a_person_with_no_acp_key_registers_nothing ... ok
test config::tests::a_row_may_name_its_own_offer_and_keep_its_servers_home ... ok
test config::tests::a_row_may_say_what_to_set_once_a_session_opens ... ok
test config::tests::a_row_that_is_turned_off_registers_no_provider ... ok
test config::tests::a_row_that_says_nothing_about_the_bridge_forwards_and_derives_its_offer ... ok
test config::tests::a_row_that_says_nothing_sets_nothing ... ok
test config::tests::a_row_with_nothing_to_run_is_refused_by_name ... ok
test config::tests::an_adapter_is_a_command_its_arguments_and_its_environment ... ok
test config::tests::an_empty_offer_list_is_a_choice_and_not_an_absent_one ... ok
test config::tests::an_option_missing_either_half_is_refused_by_name ... ok
test config::tests::the_schema_describes_the_key_it_claims ... ok
test connection::tests::a_call_is_answered_by_its_id ... ok
test connection::tests::a_call_whose_adapter_died_fails_instead_of_waiting ... ok
test connection::tests::a_method_this_client_does_not_have_is_refused_not_ignored ... ok
test connection::tests::a_notification_goes_out_with_no_id ... ok
test connection::tests::a_question_the_agent_asks_reaches_the_client_and_is_answered_by_id ... ok
test connection::tests::a_seventeen_mib_line_ends_the_connection_with_the_named_error ... ok
test connection::tests::an_update_reaches_the_client_and_an_unknown_one_does_not_end_the_stream ... ok
test connection::tests::answers_that_arrive_out_of_order_still_find_their_callers ... ok
test crossing::tests::a_crossing_that_is_dropped_dismisses_its_token ... ok
test crossing::tests::a_forwarding_row_carries_ours_and_theirs ... ok
test crossing::tests::a_house_with_no_mcp_plugin_forwards_nothing ... ok
test crossing::tests::a_row_that_forwards_nothing_carries_only_ours ... ok
test crossing::tests::a_row_the_agent_cannot_take_is_not_forwarded ... ok
test ear::tests::a_restated_extension_is_how_the_id_comes_back ... ok
test ear::tests::a_session_that_ended_keeps_no_child_and_no_id ... ok
test ear::tests::nothing_else_on_the_stream_is_mistaken_for_ours ... ok
test error::tests::a_method_the_agent_does_not_have_is_unsupported_not_a_server_fault ... ok
test error::tests::a_pipe_that_closed_is_retryable_transport ... ok
test error::tests::an_agent_with_no_credential_is_an_auth_error_in_its_own_words ... ok
test events::tests::a_diff_and_a_terminal_both_survive_whole ... ok
test events::tests::a_failed_call_says_so_and_is_still_a_call ... ok
test events::tests::a_fall_between_two_turns_is_still_a_cut ... ok
test events::tests::a_partial_update_keeps_what_it_does_not_name ... ok
test events::tests::a_plan_a_mode_and_a_command_list_say_nothing ... ok
test events::tests::a_reading_that_fell_is_the_cut_the_agent_made_and_the_banners_are_dropped ... ok
test events::tests::a_reading_that_rose_is_no_cut_at_all ... ok
test events::tests::a_replayed_user_turn_is_never_an_event ... ok
test events::tests::a_thought_is_reasoning_and_carries_no_acp_mark ... ok
test events::tests::a_turn_that_ends_mid_call_closes_everything_it_opened ... ok
test events::tests::an_adapter_that_counts_nothing_reports_zero ... ok
test events::tests::an_agents_tool_call_is_journalled_whole_and_marked_external ... ok
test events::tests::chunks_with_no_message_id_are_still_one_block ... ok
test events::tests::chunks_with_one_message_id_are_one_block ... ok
test events::tests::every_stop_reason_keeps_its_own_word ... ok
test events::tests::every_usage_update_is_a_reading ... ok
test events::tests::the_end_turn_count_is_what_the_turn_cost ... ok
test events::tests::the_failure_banner_is_the_agents_own_words ... ok
test events::tests::without_an_end_turn_count_the_context_stands_in ... ok
test guide::tests::the_page_is_one_line_of_description_and_a_body_worth_reading ... ok
test guide::tests::the_page_names_every_settings_key_and_field_this_plugin_claims ... ok
test guide::tests::the_page_names_every_tool_that_does_not_cross_the_bridge ... ok
test guide::tests::the_page_names_the_command_a_held_context_refuses ... ok
test guide::tests::the_page_names_the_label_that_never_crosses_the_wire ... ok
test guide::tests::the_page_promises_no_command_this_plugin_does_not_register ... ok
test guide::tests::the_settings_example_is_toml_and_no_json_is_left ... ok
test knobs::tests::a_row_option_crosses_in_the_agents_own_words ... ok
test knobs::tests::a_row_option_the_agent_does_not_have_sends_nothing ... ok
test knobs::tests::a_row_that_names_the_effort_knob_is_what_the_diff_then_reads ... ok
test knobs::tests::an_adapter_with_only_the_legacy_door_goes_through_it ... ok
test knobs::tests::an_agent_with_neither_knob_is_sent_nothing ... ok
test knobs::tests::the_agent_label_is_never_sent_as_a_model ... ok
test knobs::tests::the_level_the_agent_already_reports_is_not_sent_again ... ok
test knobs::tests::two_asks_that_clamp_to_one_value_are_one_message ... ok
test knobs::tests::what_a_person_is_told_about_their_row_names_the_row ... ok
test knobs::tests::what_a_person_is_told_names_what_they_would_change ... ok
test knobs::tests::which_knob_a_row_names_is_found_the_way_the_diff_finds_it ... ok
test ladder::tests::a_conversation_with_no_past_simply_begins ... ok
test ladder::tests::a_door_that_refuses_at_the_moment_of_use_drops_one_rung ... ok
test ladder::tests::history_with_no_remembered_id_is_carried_as_a_file ... ok
test ladder::tests::only_a_degradation_is_worth_saying_and_it_says_what_was_lost ... ok
test ladder::tests::the_ladder_takes_the_highest_rung_the_agent_offers ... ok
test legacy::tests::an_answer_without_the_legacy_state_lists_nothing ... ok
test legacy::tests::the_legacy_list_is_read_off_the_answer_that_carried_it ... ok
test legacy::tests::the_request_writes_the_two_fields_the_adapter_reads ... ok
test method::tests::a_body_that_does_not_fit_its_method_is_an_error ... ok
test method::tests::a_method_the_table_does_not_know_is_unsupported_not_a_crash ... ok
test method::tests::a_turn_round_trips ... ok
test method::tests::every_session_door_round_trips ... ok
test method::tests::the_elicitation_door_round_trips ... ok
test method::tests::the_handshake_round_trips ... ok
test method::tests::the_knobs_door_round_trips ... ok
test method::tests::the_lines_the_agent_starts_are_read_by_their_method ... ok
test options::tests::a_bracketed_id_is_found_by_the_model_it_names ... ok
test options::tests::a_knob_in_words_nobody_shares_is_left_where_it_is ... ok
test options::tests::a_level_is_read_off_the_label_when_the_id_does_not_say_it ... ok
test options::tests::a_level_is_the_deepest_the_agent_offers_at_or_below_what_was_asked ... ok
test options::tests::a_model_is_found_by_its_own_id_whatever_its_case ... ok
test options::tests::an_agent_with_neither_knob_has_neither ... ok
test options::tests::an_unknown_id_is_found_by_the_category_the_spec_reserves ... ok
test options::tests::both_adapters_knobs_are_found_under_their_own_ids ... ok
test options::tests::grouped_values_are_read_as_one_list ... ok
test options::tests::neither_default_nor_none_is_ever_chosen_as_a_level ... ok
test options::tests::the_last_rung_is_the_options_own_words_and_never_a_toggle ... ok
test options::tests::the_models_served_are_the_ones_the_agent_declared ... ok
test probe::tests::a_reason_nobody_heard_is_still_owed ... ok
test probe::tests::an_adapter_that_will_not_start_is_asked_once_and_serves_nothing ... ok
test probe::tests::what_a_failed_ask_says_names_the_row_and_what_is_served_instead ... ok
test provider::tests::a_fold_with_no_new_user_turn_says_nothing_new ... ok
test provider::tests::a_request_can_be_told_whose_turn_it_is ... ok
test provider::tests::a_request_with_no_session_is_refused_before_anything_is_spawned ... ok
test provider::tests::a_turn_has_said_nothing_until_an_event_goes_out ... ok
test provider::tests::a_window_the_agent_named_is_what_the_next_session_is_told ... ok
test provider::tests::absent_purpose_keeps_ordinary_session_extraction_unchanged ... ok
test provider::tests::an_adapter_nobody_has_met_yet_promises_nothing_but_holding_its_own ... ok
test provider::tests::an_instance_answers_to_its_own_name_and_files_under_acp ... ok
test provider::tests::explicit_purposes_never_delegate_even_with_a_session_and_tools ... ok
test provider::tests::only_a_dead_pipe_is_worth_asking_again ... ok
test provider::tests::only_the_last_user_turn_crosses_the_wire ... ok
test question::tests::a_call_that_named_nothing_is_named_by_its_id ... ok
test question::tests::a_second_option_of_the_same_kind_is_left_to_the_person ... ok
test question::tests::a_standing_option_is_marked_when_it_is_the_only_one_of_its_kind ... ok
test question::tests::a_standing_refusal_is_only_taken_when_it_is_the_only_one ... ok
test question::tests::an_answer_that_is_not_one_of_the_agents_options_is_no_answer ... ok
test question::tests::the_agents_own_options_become_the_questions_own ... ok
test question::tests::the_persons_choice_is_the_agents_own_option_id ... ok
test refusal::tests::an_agent_that_offers_no_way_to_refuse_is_told_the_question_is_gone ... ok
test refusal::tests::an_elicitation_is_declined_in_the_protocols_own_word ... ok
test refusal::tests::the_notice_says_nobody_answered_and_names_the_row ... ok
test refusal::tests::the_refusal_is_the_agents_own_reject_option ... ok
test render::tests::a_diff_shows_what_went_and_what_came ... ok
test render::tests::a_file_being_created_has_nothing_to_remove ... ok
test render::tests::a_heading_says_what_the_agent_is_doing_in_words ... ok
test render::tests::a_long_body_is_cut_and_the_cut_is_said ... ok
test render::tests::a_terminal_is_named_never_joined ... ok
test servers::tests::a_row_that_named_a_working_directory_is_told_it_lost_one ... ok
test servers::tests::a_row_that_names_no_transport_is_a_child_process ... ok
test servers::tests::a_row_with_nothing_to_dial_is_skipped_and_named ... ok
test servers::tests::a_stdio_row_crosses_verbatim ... ok
test servers::tests::a_transport_with_no_server_row_is_skipped_and_named ... ok
test servers::tests::an_http_row_an_agent_cannot_take_is_skipped_and_named ... ok
test servers::tests::an_http_row_crosses_to_an_agent_that_claims_http ... ok
test servers::tests::our_row_points_the_agent_back_at_this_run ... ok
test shared::tests::a_call_with_nothing_to_serve_it_is_refused_with_a_reason ... ok
test shared::tests::a_forwarded_servers_tools_are_not_offered_over_the_bridge ... ok
test shared::tests::a_row_that_chose_is_offered_what_it_chose ... ok
test shared::tests::a_session_nobody_has_prompted_yet_offers_what_the_catalogue_holds ... ok
test shared::tests::only_a_moved_offer_is_worth_telling_the_agent_about ... ok
test shared::tests::the_offer_is_the_last_requests_own_tool_list ... ok
test tests::a_row_that_could_not_work_stops_the_boot ... ok
test tests::every_row_becomes_a_provider_of_its_own_name ... ok
test tests::nothing_but_the_page_is_registered_until_an_adapter_is_configured ... ok
test tests::the_manifest_claims_one_key_and_promises_no_provider_of_its_own ... ok
test transcript::tests::a_tool_bingo_ran_carries_what_it_asked_and_what_came_back ... ok
test transcript::tests::nothing_is_written_under_an_empty_heading ... ok
test transcript::tests::the_agents_own_calls_are_replayed_and_its_thoughts_are_not ... ok
test transcript::tests::the_file_is_written_where_the_agent_can_read_it ... ok
test transcript::tests::the_first_prompt_names_the_file ... ok
test transcript::tests::the_transcript_is_the_conversation_oldest_first ... ok
test windows::tests::a_model_names_the_window_it_last_reported_and_nothing_before_that ... ok
test wire::tests::a_notification_has_no_id ... ok
test wire::tests::a_request_round_trips ... ok
test wire::tests::a_result_and_an_error_are_never_both_present ... ok
test wire::tests::an_agents_string_id_survives_the_round_trip ... ok
test wire::tests::another_version_is_not_a_message ... ok

test result: ok. 187 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s

     Running unittests src/bin/fake_agent/main.rs (target/debug/deps/bingo_fake_acp_agent-0624c801d96a1efd)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/bridge.rs (target/debug/deps/bridge-7ce2cf8a6d1b4167)

running 9 tests
test a_call_reaches_the_doors_and_its_answer_comes_back ... ok
test a_call_the_doors_refused_is_an_error_result_not_a_protocol_error ... ok
test a_changed_offer_reaches_a_live_client ... ok
test a_dismissed_conversation_is_no_longer_an_address ... ok
test a_dropped_bridge_stops_listening ... ok
test a_second_concurrent_stream_on_one_token_is_refused ... ok
test a_token_may_be_dialled_again_once_its_stream_has_closed ... ok
test a_token_nobody_minted_never_gets_a_conversation ... ok
test what_the_doors_offer_is_what_tools_list_shows ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/catalogue.rs (target/debug/deps/catalogue-ee05691e73cac74a)

running 7 tests
test a_cold_ask_opens_one_session_and_keeps_what_the_agent_declared ... ok
test a_cold_ask_reads_the_older_door_as_well ... ok
test a_cold_ask_turns_none_of_the_rows_own_knobs ... ok
test a_second_ask_is_answered_from_the_first_and_spawns_nothing ... ok
test an_adapter_that_will_not_start_serves_nothing_and_does_not_fail ... ok
test an_agent_that_refuses_the_opening_declares_nothing ... ok
test two_askers_at_once_are_one_child ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s

     Running tests/deadlines.rs (target/debug/deps/deadlines-832ef6bd69ed8790)

running 2 tests
test deadline_keeps_an_agent_that_answers_cancel ... ok
test deadline_kills_an_agent_that_ignores_cancel_after_the_turn_is_dropped ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

     Running tests/fake_agent.rs (target/debug/deps/fake_agent-0e236994cffce09e)

running 6 tests
test a_cancel_ends_the_turn_and_the_child_goes_with_the_handle ... ok
test a_load_replays_and_the_replay_is_ours_to_swallow ... ok
test a_question_the_agent_asks_is_refused_and_the_turn_goes_on ... ok
test a_scripted_agent_answers_a_turn_over_real_pipes ... ok
test an_agent_that_advertises_neither_door_refuses_both ... ok
test an_agent_with_no_login_refuses_the_session_in_its_own_words ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/pictures.rs (target/debug/deps/pictures-5288b0e335ab7274)

running 2 tests
test an_acp_agent_without_images_receives_one_note_per_picture ... ok
test an_image_capable_acp_agent_receives_each_picture_in_order ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running unittests src/lib.rs (target/debug/deps/bingo_provider_anthropic-8e2c58b1fdd4f5d1)

running 58 tests
test error::tests::a_400_naming_the_context_window_is_an_overflow_the_loop_compacts ... ok
test error::tests::a_408_is_a_timeout ... ok
test error::tests::a_429_carries_the_retry_after_header_in_milliseconds ... ok
test error::tests::a_429_without_the_header_still_rate_limits ... ok
test error::tests::a_message_that_merely_contains_512_characters_is_not_a_server_error ... ok
test error::tests::a_stream_error_event_is_named_by_its_type ... ok
test error::tests::an_error_envelope_is_unwrapped_and_a_bare_body_is_kept ... ok
test error::tests::an_http_date_retry_after_becomes_a_delay_and_a_past_date_becomes_zero ... ok
test error::tests::an_unnamed_stream_error_is_read_from_its_sentence ... ok
test error::tests::credentials_are_an_auth_failure_and_never_retried ... ok
test error::tests::other_client_errors_are_bad_requests ... ok
test error::tests::server_errors_and_529_overloaded_are_retryable ... ok
test events::tests::a_malformed_frame_is_a_stream_error ... ok
test events::tests::a_max_tokens_stop_finishes_as_length ... ok
test events::tests::a_mid_stream_error_event_ends_the_stream_and_keeps_what_came_before ... ok
test events::tests::a_signature_rides_home_on_the_reasoning_end ... ok
test events::tests::a_text_turn_starts_streams_and_finishes ... ok
test events::tests::a_tool_block_is_keyed_by_its_tool_use_id_and_ends_with_the_whole_call ... ok
test events::tests::a_tool_called_with_no_arguments_still_carries_parsable_json ... ok
test events::tests::an_unknown_block_type_and_an_unknown_event_are_ignored ... ok
test events::tests::every_stop_reason_maps_and_an_unknown_one_is_kept_raw ... ok
test events::tests::the_three_input_counts_stay_apart ... ok
test instances::tests::a_blank_value_counts_as_unset_wherever_it_is_written ... ok
test instances::tests::a_name_that_could_not_be_typed_is_refused ... ok
test instances::tests::a_name_that_is_taken_is_refused_by_name ... ok
test instances::tests::an_instance_has_no_variable_to_read ... ok
test instances::tests::an_instance_talks_to_its_own_base_url_or_the_public_one ... ok
test models::tests::a_bare_array_and_an_unreadable_body_are_both_tolerated ... ok
test models::tests::the_catalogue_is_read_from_data_and_sorted ... ok
test request::tests::a_message_with_no_cacheable_block_spends_none_of_the_budget ... ok
test request::tests::a_model_without_caching_gets_no_breakpoints ... ok
test request::tests::a_picture_that_knows_its_path_is_followed_by_it ... ok
test request::tests::a_text_turn_encodes_to_a_streaming_messages_body ... ok
test request::tests::a_tool_round_encodes_the_schema_the_call_and_its_result ... ok
test request::tests::an_image_becomes_a_base64_source_block ... ok
test request::tests::caching_marks_the_last_system_block_and_the_newest_messages ... ok
test request::tests::counting_tokens_measures_the_same_conversation_without_generation_parameters ... ok
test request::tests::every_effort_level_maps_to_a_level_the_api_accepts ... ok
test request::tests::no_reasoning_sends_neither_parameter ... ok
test request::tests::provider_options_reach_the_body_unchanged ... ok
test request::tests::reasoning_sends_the_adaptive_shape_and_replays_only_signed_blocks ... ok
test request::tests::the_budget_never_exceeds_four_however_long_the_conversation ... ok
test settings::tests::an_absent_key_is_the_default_endpoint_and_no_instances ... ok
test settings::tests::an_instance_carries_its_parents_fields_and_no_instances_of_its_own ... ok
test stream_tests::a_cancelled_turn_stops_the_stream_before_it_finishes ... ok
test stream_tests::a_chunked_body_decodes_to_the_same_events_as_one_piece ... ok
test stream_tests::a_mid_stream_error_arrives_after_everything_that_preceded_it ... ok
test stream_tests::a_token_cancelled_before_the_first_poll_yields_nothing ... ok
test tests::a_claimed_api_key_reaches_the_provider ... ok
test tests::an_instance_reads_its_own_key_and_nothing_ambient ... ok
test tests::an_instance_that_takes_a_registered_name_is_refused_at_boot ... ok
test tests::every_instance_serves_the_anthropic_family ... ok
test tests::no_key_anywhere_leaves_authentication_missing ... ok
test tests::the_base_url_defaults_and_a_setting_overrides_it_without_a_trailing_slash ... ok
test tests::the_claimed_key_merges_by_replacement_and_has_a_schema ... ok
test tests::the_default_key_provider_takes_a_pasted_key ... ok
test tests::the_plugin_registers_a_provider_per_endpoint_the_settings_name ... ok
test tests::without_a_key_a_turn_fails_before_it_reaches_the_wire ... ok

test result: ok. 58 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s

     Running tests/messages_api.rs (target/debug/deps/messages_api-70fa213077ba53c2)

running 12 tests
test a_400_naming_the_context_limit_becomes_an_overflow_the_loop_compacts ... ok
test a_429_becomes_a_rate_limit_carrying_the_retry_after_delay ... ok
test a_529_is_a_retryable_server_error_and_a_401_is_not ... ok
test a_failing_read_endpoint_is_classified_like_any_other_response ... ok
test a_mid_stream_error_event_ends_the_stream_as_a_retryable_failure ... ok
test a_signature_reaches_the_reasoning_end_over_the_wire ... ok
test a_text_turn_streams_events_and_the_usage_the_wire_reported ... ok
test a_tool_turn_ends_with_the_call_and_a_tool_calls_finish ... ok
test a_truncated_turn_finishes_as_length ... ok
test counting_tokens_reads_the_input_count ... ok
test the_body_on_the_wire_is_the_encoding_the_snapshots_pin ... ok
test the_model_catalogue_comes_from_the_data_envelope ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.12s

     Running unittests src/lib.rs (target/debug/deps/bingo_provider_fake-d6f05e84352e854a)

running 29 tests
test addressed_tests::a_matcher_reads_tool_calls_and_their_results ... ok
test addressed_tests::a_request_matching_nothing_left_is_refused ... ok
test addressed_tests::a_side_question_takes_the_side_response_addressed_to_it ... ok
test addressed_tests::a_spent_script_is_exhausted ... ok
test addressed_tests::an_addressed_response_waits_for_its_own_asker ... ok
test addressed_tests::an_open_response_is_taken_by_whoever_asks_first ... ok
test addressed_tests::an_unaddressed_script_is_dealt_in_order ... ok
test addressed_tests::when_is_optional_in_the_script_schema ... ok
test side_tests::a_side_question_is_answered_with_nothing_and_takes_no_response ... ok
test tests::a_cancelled_token_stops_the_stream ... ok
test tests::a_delay_before_the_stream_waits_before_the_stream_exists ... ok
test tests::a_delay_step_waits_on_the_timer ... ok
test tests::a_missing_script_file_is_a_read_error ... ok
test tests::a_reasoning_step_yields_reasoning_events ... ok
test tests::a_script_round_trips_through_json ... ok
test tests::a_text_step_yields_start_chunked_deltas_end_then_finish ... ok
test tests::a_tool_call_step_yields_start_delta_end_then_the_call ... ok
test tests::a_tool_call_without_an_explicit_finish_finishes_as_tool_calls ... ok
test tests::an_answered_tool_use_is_accepted ... ok
test tests::an_empty_assistant_message_is_rejected ... ok
test tests::an_empty_conversation_is_rejected ... ok
test tests::an_error_step_ends_the_stream_with_that_error ... ok
test tests::an_unanswered_tool_use_is_rejected ... ok
test tests::counting_tokens_estimates_four_characters_each ... ok
test tests::each_call_takes_the_next_response_and_then_the_script_is_exhausted ... ok
test tests::every_request_is_recorded_in_order ... ok
test tests::from_env_reads_the_scripted_path_and_is_none_without_it ... ok
test tests::the_fake_advertises_one_model ... ok
test tests::the_plugin_registers_the_provider_it_was_built_with ... ok

test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running unittests src/lib.rs (target/debug/deps/bingo_provider_openai-50a364812e7395b1)

running 86 tests
test effort::tests::a_dated_snapshot_reads_its_familys_row ... ok
test effort::tests::every_family_clamps_every_level_into_what_it_accepts ... ok
test effort::tests::every_row_offers_a_level_and_lists_it_ascending ... ok
test effort::tests::no_family_ever_answers_with_reasoning_turned_off ... ok
test error::tests::a_400_naming_the_context_length_is_an_overflow_kept_verbatim ... ok
test error::tests::a_429_reads_the_delay_from_the_header_or_the_body ... ok
test error::tests::a_nonsense_delay_is_no_delay ... ok
test error::tests::a_stream_error_is_named_by_its_code ... ok
test error::tests::a_stream_overflow_keeps_the_sentence_the_kernel_measures ... ok
test error::tests::an_error_envelope_is_unwrapped_and_a_bare_body_is_kept ... ok
test error::tests::an_http_date_retry_after_becomes_a_delay_and_a_past_date_becomes_zero ... ok
test error::tests::credentials_are_an_auth_failure_and_never_retried ... ok
test error::tests::other_client_errors_are_bad_requests ... ok
test error::tests::server_errors_are_retryable_and_a_408_is_a_timeout ... ok
test error::tests::the_code_falls_back_to_the_error_type ... ok
test error::tests::the_header_wins_over_the_body_and_neither_is_required ... ok
test events::tests::a_failed_response_ends_the_stream_and_keeps_what_came_before ... ok
test events::tests::a_malformed_frame_is_a_stream_error ... ok
test events::tests::a_reasoning_turn_reports_the_tokens_it_spent_thinking ... ok
test events::tests::a_text_turn_starts_streams_and_finishes ... ok
test events::tests::a_tool_called_with_no_arguments_still_carries_parsable_json ... ok
test events::tests::a_tool_item_is_keyed_by_its_call_id_and_ends_with_the_whole_call ... ok
test events::tests::an_error_event_ends_the_stream_too ... ok
test events::tests::an_incomplete_response_finishes_as_length ... ok
test events::tests::an_unknown_item_type_and_an_unknown_event_are_ignored ... ok
test events::tests::arguments_that_never_streamed_are_backfilled_as_one_delta ... ok
test events::tests::both_reasoning_delta_names_reach_the_same_block ... ok
test events::tests::every_incomplete_reason_maps_and_an_unknown_one_is_kept_raw ... ok
test events::tests::the_cached_prefix_is_subtracted_out_of_the_input_count ... ok
test events::tests::the_encrypted_state_rides_home_on_the_reasoning_end ... ok
test input::tests::a_call_carries_its_arguments_as_a_json_string ... ok
test input::tests::a_failed_tool_keeps_the_flag_the_wire_has_no_field_for ... ok
test input::tests::a_picture_that_knows_its_path_is_followed_by_it ... ok
test input::tests::a_tool_result_is_its_own_item_and_precedes_what_the_user_said ... ok
test input::tests::an_empty_user_turn_produces_no_item ... ok
test input::tests::an_image_a_tool_returned_follows_as_an_input_image ... ok
test input::tests::reasoning_replays_only_when_both_halves_came_back ... ok
test instances::tests::a_blank_value_counts_as_unset_wherever_it_is_written ... ok
test instances::tests::a_name_that_could_not_be_typed_is_refused ... ok
test instances::tests::a_name_that_is_taken_is_refused_by_name ... ok
test instances::tests::an_instance_has_no_variable_to_read ... ok
test instances::tests::an_instance_serves_its_variants_family ... ok
test instances::tests::an_instance_talks_to_its_own_base_url_or_the_public_one ... ok
test models::tests::a_codex_body_that_is_not_a_catalogue_is_empty_rather_than_a_failure ... ok
test models::tests::a_listed_model_carries_no_display_name ... ok
test models::tests::a_model_with_no_priority_sorts_last ... ok
test models::tests::a_models_envelope_a_bare_array_and_plain_strings_are_all_read ... ok
test models::tests::an_unreadable_body_is_an_empty_catalogue_rather_than_a_failure ... ok
test models::tests::the_catalogue_is_read_from_data_and_sorted ... ok
test models::tests::the_codex_catalogue_is_ordered_by_priority_and_hides_what_it_is_told_to ... ok
test models::tests::the_fallback_is_the_nine_models_the_old_project_proved ... ok
test request::tests::a_text_turn_encodes_to_a_stateless_streaming_body ... ok
test request::tests::a_tool_round_encodes_the_schema_the_call_and_its_result ... ok
test request::tests::an_image_becomes_a_data_url_input_image ... ok
test request::tests::no_reasoning_sends_neither_the_object_nor_the_include ... ok
test request::tests::no_system_and_no_tools_leave_the_fields_off_the_wire ... ok
test request::tests::provider_options_reach_the_body_unchanged ... ok
test request::tests::reasoning_asks_for_encrypted_state_and_replays_only_what_carries_it ... ok
test request::tests::the_codex_variant_differs_only_by_the_output_budget ... ok
test request::tests::the_effort_is_clamped_to_what_the_model_takes ... ok
test request::tests::the_system_blocks_join_into_one_instructions_string ... ok
test settings::tests::an_absent_key_is_the_default_endpoint_and_no_instances ... ok
test settings::tests::an_instance_carries_its_parents_fields_and_no_instances_of_its_own ... ok
test stream_tests::a_cancelled_turn_stops_the_stream_before_it_finishes ... ok
test stream_tests::a_chunked_body_decodes_to_the_same_events_as_one_piece ... ok
test stream_tests::a_mid_stream_failure_arrives_after_everything_that_preceded_it ... ok
test stream_tests::a_token_cancelled_before_the_first_poll_yields_nothing ... ok
test tests::a_claimed_api_key_reaches_the_provider ... ok
test tests::a_key_provider_signs_in_by_paste_alone ... ok
test tests::a_receipt_names_the_account_only_when_the_issuer_did ... ok
test tests::a_subscription_says_how_to_sign_in_and_how_to_sign_in_again ... ok
test tests::a_subscription_token_with_no_account_claim_omits_the_header ... ok
test tests::an_instance_reads_its_own_key_and_its_own_endpoint ... ok
test tests::an_instance_that_takes_a_registered_name_is_refused_at_boot ... ok
test tests::both_claimed_keys_merge_by_replacement_and_have_a_schema ... ok
test tests::images_default_to_forwarded_and_a_proxy_can_turn_them_off ... ok
test tests::no_key_anywhere_leaves_authentication_missing ... ok
test tests::one_plugin_registers_a_provider_per_endpoint_the_settings_name ... ok
test tests::the_base_url_defaults_and_a_setting_overrides_it_without_a_trailing_slash ... ok
test tests::the_codex_settings_key_moves_both_endpoints_for_a_proxy ... ok
test tests::the_default_key_provider_takes_a_pasted_key ... ok
test tests::the_missing_key_hint_names_the_login_the_variable_and_the_settings_file ... ok
test tests::the_public_endpoint_sends_a_bearer_key_and_nothing_of_the_subscription ... ok
test tests::the_subscription_endpoint_adds_its_originator_and_account ... ok
test tests::without_a_key_a_turn_fails_before_it_reaches_the_wire ... ok
test variant::tests::each_variant_names_its_own_path_and_budget_rule ... ok

test result: ok. 86 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s

     Running tests/codex_subscription.rs (target/debug/deps/codex_subscription-17a9e090b2862f7d)

running 5 tests
test a_catalogue_that_will_not_answer_leaves_the_static_list ... ok
test a_refused_request_is_retried_once_with_a_renewed_bearer ... ok
test a_second_refusal_asks_the_person_to_sign_in_again ... ok
test a_signed_out_subscription_names_the_command_before_any_request ... ok
test the_model_menu_is_the_subscriptions_own_catalogue ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.14s

     Running tests/responses_api.rs (target/debug/deps/responses_api-1bbd1125a45f01e7)

running 16 tests
test a_400_naming_the_context_length_becomes_an_overflow_the_loop_compacts ... ok
test a_401_is_an_auth_failure_and_a_503_is_a_retryable_server_error ... ok
test a_429_reads_the_delay_from_the_header ... ok
test a_429_without_the_header_reads_the_delay_from_the_body ... ok
test a_failed_response_ends_the_stream_as_a_retryable_failure ... ok
test a_failing_read_endpoint_is_classified_like_any_other_response ... ok
test a_reasoning_turn_streams_both_delta_names_and_returns_its_encrypted_state ... ok
test a_text_turn_streams_events_and_the_usage_the_wire_reported ... ok
test a_tool_turn_ends_with_the_call_and_a_tool_calls_finish ... ok
test a_truncated_turn_finishes_as_length ... ok
test an_encrypted_reasoning_item_round_trips_over_the_wire ... ok
test cache_affinity_follows_the_session_unless_the_caller_overrides_it ... ok
test cache_reads_and_writes_are_disjoint_from_fresh_input_tokens ... ok
test the_body_on_the_wire_is_the_stateless_encoding_the_snapshots_pin ... ok
test the_codex_variant_uses_its_own_path_and_headers ... ok
test the_model_catalogue_comes_from_the_data_envelope ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.13s

     Running unittests src/lib.rs (target/debug/deps/bingo_rooms-34e574e1923b6478)

running 230 tests
test chase::tests::a_debt_already_chased_is_not_chased_twice_over ... ok
test chase::tests::a_debt_found_overdue_is_nudged_once_and_at_once ... ok
test chase::tests::a_debt_the_holder_owes_is_nudged_at_the_session_the_room_hangs_under ... ok
test chase::tests::a_fresh_question_waits_and_an_overdue_one_does_not ... ok
test chase::tests::a_member_whose_seat_is_gone_is_skipped_and_nothing_wedges ... ok
test chase::tests::a_nudge_says_where_who_and_what_and_is_not_a_post ... ok
test chase::tests::an_unanswered_question_is_nudged_three_times_and_then_no_more ... ok
test chase::tests::answering_cancels_the_timer_mid_sleep ... ok
test chase::tests::one_room_answering_leaves_another_room_s_question_standing ... ok
test chase::tests::the_room_s_own_debt_is_never_chased ... ok
test command::tests::a_member_with_a_patience_beside_it_is_seated_wearing_that_ear ... ok
test command::tests::a_name_that_is_not_one_is_refused_and_opens_nothing ... ok
test command::tests::a_patience_under_the_floor_is_refused_and_opens_nothing ... ok
test command::tests::a_room_nobody_is_in_says_so ... ok
test command::tests::a_session_with_no_rooms_says_so_in_one_line ... ok
test command::tests::closing_a_room_ends_it_and_the_listing_says_so ... ok
test command::tests::closing_needs_a_name_and_a_room_that_is_there ... ok
test command::tests::opening_a_room_acks_with_the_room_and_who_is_in_it ... ok
test command::tests::the_listing_says_what_each_room_is_for ... ok
test command::tests::the_listing_says_what_seating_the_holder_gets_you ... ok
test command::tests::the_owed_column_names_who_has_not_answered_and_for_how_long ... ok
test command::tests::the_spec_runs_now_and_takes_a_name_and_names ... ok
test command::tests::the_table_names_every_room_under_this_session_and_its_members ... ok
test cursor::tests::a_cursor_is_one_post_id_under_a_kind_of_its_own ... ok
test cursor::tests::a_room_s_own_journal_says_where_each_seat_has_read_up_to ... ok
test cursor::tests::a_seat_reads_the_others_and_its_own_post_still_moves_the_cursor ... ok
test cursor::tests::what_is_unread_is_what_landed_after_the_cursor ... ok
test deadline::tests::a_cursor_at_the_head_leaves_nothing_unread ... ok
test deadline::tests::a_later_post_does_not_move_the_deadline_the_first_one_set ... ok
test deadline::tests::a_live_seat_is_nobody_s_deadline ... ok
test deadline::tests::a_live_seat_s_backlog_is_nobody_s_deadline ... ok
test deadline::tests::a_seat_left_behind_is_woken_once_when_its_patience_is_up ... ok
test deadline::tests::a_seat_seated_at_the_head_starts_level ... ok
test deadline::tests::a_seat_that_reads_the_room_before_the_deadline_is_not_nudged ... ok
test deadline::tests::a_seat_this_process_finds_already_behind_is_nudged_once ... ok
test deadline::tests::four_patient_members_and_one_poster_cost_one_wake_each_per_patience ... ok
test deadline::tests::the_session_the_room_hangs_under_is_a_patient_seat_too ... ok
test door::tests::a_name_no_room_here_answers_to_is_refused_with_the_ones_that_do ... ok
test door::tests::a_room_argument_names_a_room_under_the_caller_before_one_beside_it ... ok
test door::tests::only_the_session_it_hangs_under_and_the_one_that_opened_it_may_change_it ... ok
test door::tests::what_the_door_hands_back_is_the_room_its_journal_says_it_is ... ok
test ear::tests::a_door_s_roster_is_its_members_with_the_listeners_ears ... ok
test ear::tests::a_journal_is_read_rather_than_refused ... ok
test ear::tests::a_listener_is_a_name_or_a_name_and_a_patience ... ok
test ear::tests::a_number_is_the_whole_of_the_dial_and_the_dead_band_is_refused ... ok
test ear::tests::a_roster_word_says_the_ear_beside_the_name ... ok
test ear::tests::a_seat_reads_back_as_the_word_that_asked_for_it ... ok
test ear::tests::a_seat_s_ear_is_published_under_a_kind_of_its_own ... ok
test ear::tests::an_ear_is_the_roster_s_until_the_seat_says_otherwise ... ok
test ear::tests::the_default_ear_is_a_patient_one_at_the_chaser_s_own_patience ... ok
test ear::tests::the_two_layers_are_read_in_their_own_order_whatever_order_they_arrived_in ... ok
test guide::tests::the_page_is_one_line_of_description_and_a_body_worth_reading ... ok
test guide::tests::the_page_names_every_column_the_listing_shows ... ok
test guide::tests::the_page_names_every_tool_and_command_this_plugin_registers ... ok
test guide::tests::the_page_names_the_word_that_ends_a_room_and_the_one_that_is_the_holder ... ok
test guide::tests::the_page_says_the_patience_a_seat_has_and_the_band_that_is_refused ... ok
test guide::tests::this_plugin_claims_no_settings_for_the_page_to_name ... ok
test hook::tests::a_backlog_found_at_the_announce_is_nudged_once ... ok
test hook::tests::a_closing_takes_the_room_s_debts_off_the_card_and_cancels_their_chase ... ok
test hook::tests::a_patient_seat_the_post_does_not_name_is_written_to_not_at_all ... ok
test hook::tests::a_post_nobody_signed_came_from_the_session_the_room_hangs_under ... ok
test hook::tests::a_post_that_says_at_all_wakes_every_patient_seat_but_its_author ... ok
test hook::tests::a_post_wakes_every_live_member_but_the_one_who_wrote_it ... ok
test hook::tests::a_project_s_rooms_are_seated_when_a_person_s_session_opens ... ok
test hook::tests::a_project_that_declares_none_seats_none ... ok
test hook::tests::a_question_nobody_asked_of_a_member_chases_nobody ... ok
test hook::tests::a_question_this_process_finds_already_overdue_is_chased_once ... ok
test hook::tests::a_reopened_room_is_the_same_room_and_keeps_its_members ... ok
test hook::tests::a_room_found_closed_is_chased_for_nothing_and_carries_no_card ... ok
test hook::tests::a_seat_a_post_left_behind_is_woken_by_the_hook_at_its_patience ... ok
test hook::tests::a_session_ending_seats_nothing ... ok
test hook::tests::a_team_file_that_will_not_parse_seats_nobody_and_fails_nothing ... ok
test hook::tests::a_user_item_outside_a_room_is_nobody_s_business_here ... ok
test hook::tests::an_agent_s_session_opening_seats_nothing ... ok
test hook::tests::it_asks_for_the_two_points_it_uses ... ok
test hook::tests::the_card_says_what_stands_and_goes_when_the_last_debt_closes ... ok
test listen::tests::a_caller_off_the_roster_is_refused_and_a_room_that_is_not_there_says_so ... ok
test listen::tests::a_caller_reaches_a_room_under_it_and_a_room_beside_it ... ok
test listen::tests::a_member_tunes_its_own_ear_and_is_told_what_it_now_wears ... ok
test listen::tests::a_patience_under_the_floor_is_refused_and_writes_nothing ... ok
test listen::tests::the_card_names_the_room_and_the_ear ... ok
test listen::tests::the_description_says_what_each_end_of_the_dial_does ... ok
test listen::tests::the_spec_asks_for_a_room_and_a_number ... ok
test listen::tests::the_traits_fail_closed ... ok
test mentions::tests::a_debt_carries_the_head_of_the_post_that_opened_it ... ok
test mentions::tests::a_debt_is_owed_in_the_roster_s_spelling_however_it_was_typed ... ok
test mentions::tests::a_head_is_the_first_line_of_it ... ok
test mentions::tests::a_hyphenated_member_is_one_word ... ok
test mentions::tests::a_member_named_all_takes_the_word_back_from_the_sigil ... ok
test mentions::tests::a_member_s_post_closes_every_debt_they_owe ... ok
test mentions::tests::a_name_is_a_word_and_only_a_member_s ... ok
test mentions::tests::a_post_that_answers_and_asks_settles_before_it_opens ... ok
test mentions::tests::a_room_nobody_is_in_owes_nothing ... ok
test mentions::tests::an_at_a_member_opens_a_debt_and_the_member_s_next_post_closes_it ... ok
test mentions::tests::at_all_is_answered_by_a_member_the_room_gained_after_it_was_said ... ok
test mentions::tests::at_all_is_one_debt_against_the_room_that_any_other_member_closes ... ok
test mentions::tests::one_post_owes_once_however_often_it_says_the_name ... ok
test mentions::tests::only_a_user_item_in_the_journal_is_a_post ... ok
test mentions::tests::the_holder_on_the_roster_owes_and_answers_like_any_member ... ok
test owed::tests::a_member_who_owes_twice_is_named_once_for_the_older ... ok
test owed::tests::an_age_reads_the_way_a_person_says_it ... ok
test owed::tests::the_card_carries_the_table_and_the_debts_it_is_drawn_from ... ok
test owed::tests::the_card_is_both_a_table_and_the_debts_beside_it ... ok
test owed::tests::the_column_names_who_owes_oldest_first_with_how_long ... ok
test owed::tests::the_holder_owes_by_the_name_its_members_call_it ... ok
test owed::tests::the_room_is_owed_under_its_own_sigil ... ok
test placement::tests::a_room_hangs_under_the_caller_by_default ... ok
test placement::tests::a_root_asking_for_its_peers_is_told_why_it_has_none ... ok
test placement::tests::shared_hangs_it_under_the_session_that_started_the_caller ... ok
test placement::tests::shared_picks_the_parents_tree_and_the_default_picks_the_callers_own ... ok
test placement::tests::the_card_names_the_tree_a_room_will_hang_in ... ok
test plugin_tests::a_second_plugin_rearms_persisted_debt_and_patience_once ... ok
test plugin_tests::registering_reads_nothing_and_contributes_what_the_manifest_promises ... ok
test plugin_tests::the_manifest_says_what_it_provides_and_claims_no_settings ... ok
test post::tests::a_member_named_all_takes_the_word_back_and_the_rest_keep_waiting ... ok
test post::tests::a_name_nobody_has_is_skipped_and_the_rest_are_still_woken ... ok
test post::tests::a_patient_seat_is_left_waiting_and_named_by_the_fan_out ... ok
test post::tests::a_patient_seat_waits_for_what_it_is_not_called_on_by_name ... ok
test post::tests::a_retuning_decides_the_wake_over_what_the_roster_declared ... ok
test post::tests::a_room_is_never_a_nudge_target ... ok
test post::tests::a_rostered_holder_is_never_woken_for_its_own_post ... ok
test post::tests::a_rostered_holder_is_woken_by_a_post_like_any_member ... ok
test post::tests::a_woken_seat_is_nudged_and_the_post_itself_is_copied_nowhere ... ok
test post::tests::an_agent_holder_hears_the_room_but_not_itself ... ok
test post::tests::at_all_reaches_the_room_as_it_stands_rather_than_as_it_stood ... ok
test post::tests::at_all_wakes_every_member_but_the_one_who_wrote_it ... ok
test post::tests::each_seat_is_woken_at_most_once_per_post ... ok
test post::tests::every_seat_but_the_author_is_decided_for_and_a_rostered_holder_with_them ... ok
test post::tests::the_parent_hears_nothing_of_its_own_room ... ok
test reader::tests::a_closed_room_is_read_to_its_end_and_then_never_again ... ok
test reader::tests::a_member_reads_what_the_room_said_since_its_cursor_and_no_more ... ok
test reader::tests::a_reading_carries_the_purpose_the_room_was_opened_for ... ok
test reader::tests::a_reading_of_no_posts_is_no_piece_at_all ... ok
test reader::tests::a_rostered_holder_reads_the_room_and_an_off_roster_one_reads_nothing ... ok
test reader::tests::a_seat_is_told_the_protocol_once_ahead_of_its_first_reading ... ok
test reader::tests::a_seat_joins_a_running_room_at_its_head ... ok
test reader::tests::a_seat_with_nothing_to_read_is_still_told_what_a_room_is ... ok
test reader::tests::a_session_in_no_room_is_told_nothing ... ok
test reader::tests::a_session_no_roster_names_reads_nothing ... ok
test reader::tests::it_speaks_at_the_head_of_a_round_under_its_own_name ... ok
test reader::tests::reseating_the_same_roster_marks_nothing_read ... ok
test reader::tests::ten_posts_are_read_as_one_piece_under_one_label ... ok
test reader::tests::the_holder_s_post_is_read_by_its_name_and_a_seat_skips_its_own ... ok
test reader::tests::the_protocol_says_how_a_room_is_read_what_wakes_a_seat_and_what_it_owes ... ok
test room::tests::a_closing_is_a_moment_a_name_and_what_was_said_about_it ... ok
test room::tests::a_log_session_keyed_rooms_under_a_parent_is_a_room ... ok
test room::tests::a_membership_round_trips_through_the_payload_it_is_published_as ... ok
test room::tests::a_room_is_closed_by_the_frame_and_not_by_what_is_in_it ... ok
test room::tests::a_roster_that_names_no_ear_reads_as_the_default_one ... ok
test room::tests::an_opening_is_a_purpose_and_the_name_that_opened_it ... ok
test room::tests::an_opening_without_a_purpose_still_says_who_opened_it ... ok
test room::tests::every_other_session_is_not_one ... ok
test room::tests::the_payload_carries_the_names_the_listeners_and_the_tree_it_draws_as ... ok
test room::tests::the_payload_is_both_a_membership_and_a_view ... ok
test roster::tests::a_room_enters_when_it_announces_itself_and_a_reopen_changes_nothing ... ok
test roster::tests::a_room_folds_what_it_was_opened_for_and_that_it_has_closed ... ok
test roster::tests::a_seat_s_own_ear_is_folded_beside_the_roster_that_declared_it ... ok
test roster::tests::nothing_that_is_not_a_room_enters_and_nothing_unknown_takes_members ... ok
test seat::tests::a_close_posts_the_last_line_and_then_says_the_room_has_ended ... ok
test seat::tests::a_close_without_a_reason_says_only_that_it_closed ... ok
test seat::tests::a_closed_name_is_refused_at_both_doors ... ok
test seat::tests::a_closed_room_takes_no_verb ... ok
test seat::tests::a_join_appends_to_the_standing_roster_and_the_room_reads_it ... ok
test seat::tests::a_join_that_names_a_standing_seat_retunes_it_rather_than_seating_it_twice ... ok
test seat::tests::a_name_that_is_not_one_opens_nothing ... ok
test seat::tests::a_new_room_is_a_log_session_under_the_caller_with_a_key_of_its_own ... ok
test seat::tests::a_new_room_says_what_it_is_for_and_who_opened_it ... ok
test seat::tests::a_reseat_declares_the_ears_whole_and_clears_what_a_seat_retuned ... ok
test seat::tests::a_room_that_stands_is_reused_and_its_membership_replaced ... ok
test seat::tests::a_room_without_members_has_none ... ok
test seat::tests::a_standing_room_that_was_opened_without_a_purpose_says_so ... ok
test seat::tests::an_agent_of_the_same_name_is_not_a_room ... ok
test seat::tests::an_unseat_of_a_name_the_room_is_not_seating_is_refused_and_changes_nothing ... ok
test seat::tests::an_unseat_publishes_the_roster_without_the_name_and_tells_the_seat ... ok
test seat::tests::the_agent_s_door_refuses_a_name_that_stands_and_names_the_verbs ... ok
test serial::tests::a_bounce_quotes_the_missed_posts_and_reads_back_as_a_ledger ... ok
test serial::tests::a_caller_level_with_the_room_lands_and_one_behind_it_does_not ... ok
test serial::tests::a_journaled_bounce_counts_as_read_and_unlocks_the_next_attempt ... ok
test serial::tests::a_post_is_a_user_item_and_its_author_is_who_signed_it ... ok
test serial::tests::another_room_s_bounce_and_a_call_after_the_cut_are_not_drafts ... ok
test serial::tests::only_what_was_journaled_before_the_calling_item_is_read ... ok
test serial::tests::the_caller_s_cursor_into_the_room_is_the_count_of_what_it_has_read ... ok
test serial::tests::the_latest_post_this_room_bounced_is_the_draft ... ok
test serial::tests::what_the_caller_could_have_heard_and_what_it_could_not ... ok
test team::tests::a_declared_room_names_its_members ... ok
test team::tests::a_declared_room_says_which_of_them_listen ... ok
test team::tests::a_directory_json_cannot_name_is_refused_rather_than_panicked_on ... ok
test team::tests::a_host_with_no_team_file_service_declares_no_rooms ... ok
test team::tests::a_patience_nobody_can_hold_is_refused_where_it_is_declared ... ok
test team::tests::a_rooms_key_that_is_not_a_roster_says_so ... ok
test team::tests::a_team_file_the_owner_refused_is_refused_here_in_its_own_words ... ok
test team::tests::nothing_declared_is_no_room ... ok
test team::tests::the_rooms_key_is_asked_for_by_key_method_and_directory ... ok
test tool::close::tests::a_caller_that_neither_holds_nor_opened_the_room_is_refused ... ok
test tool::close::tests::a_room_is_closed_and_the_block_says_who_was_in_it ... ok
test tool::close::tests::a_room_that_has_ended_is_not_closed_twice ... ok
test tool::close::tests::the_card_names_the_room_that_is_about_to_end ... ok
test tool::close::tests::the_spec_asks_for_a_room_and_says_what_closing_costs ... ok
test tool::close::tests::the_traits_fail_closed ... ok
test tool::join::tests::a_caller_that_neither_holds_nor_opened_the_room_is_refused ... ok
test tool::join::tests::a_name_is_seated_beside_the_roster_that_stands ... ok
test tool::join::tests::a_room_the_caller_cannot_reach_and_a_call_naming_nobody_are_refused ... ok
test tool::join::tests::the_card_names_the_room_and_the_seats_it_takes ... ok
test tool::join::tests::the_spec_asks_for_a_room_and_says_what_seating_is_for ... ok
test tool::join::tests::the_traits_fail_closed ... ok
test tool::leave::tests::a_caller_that_neither_holds_nor_opened_the_room_is_refused ... ok
test tool::leave::tests::a_name_leaves_the_roster_and_the_seat_that_left_is_told ... ok
test tool::leave::tests::a_name_the_room_is_not_seating_is_refused_and_the_roster_stands ... ok
test tool::leave::tests::the_card_names_the_room_and_who_leaves_it ... ok
test tool::leave::tests::the_spec_asks_for_a_room_and_the_names_to_take_out ... ok
test tool::leave::tests::the_traits_fail_closed ... ok
test tool::open::tests::a_call_that_says_no_purpose_is_refused_with_the_schema ... ok
test tool::open::tests::a_call_that_will_not_parse_names_nothing ... ok
test tool::open::tests::a_name_that_is_not_one_is_refused_by_the_same_rule_room_uses ... ok
test tool::open::tests::a_patience_under_the_floor_is_refused_and_opens_nothing ... ok
test tool::open::tests::a_room_hangs_under_the_caller_and_its_own_workers_hear_it ... ok
test tool::open::tests::a_room_says_what_it_was_opened_for_and_who_opened_it ... ok
test tool::open::tests::a_room_with_nobody_in_it_is_opened_and_says_so ... ok
test tool::open::tests::a_root_asking_to_share_is_refused_with_the_reason_and_opens_nothing ... ok
test tool::open::tests::a_shared_room_hangs_under_the_parent_and_a_peer_hears_it ... ok
test tool::open::tests::a_standing_name_is_refused_and_the_room_is_left_alone ... ok
test tool::open::tests::listeners_seat_a_patient_ear_and_the_receipt_says_so ... ok
test tool::open::tests::room_lists_what_the_tool_opened ... ok
test tool::open::tests::the_block_draws_the_room_and_the_seats_it_took ... ok
test tool::open::tests::the_card_names_the_room_the_members_and_where_it_will_hang ... ok
test tool::open::tests::the_description_says_a_room_is_opened_once_for_one_purpose ... ok
test tool::open::tests::the_description_says_what_naming_the_holder_gets_you ... ok
test tool::open::tests::the_spec_asks_for_a_name_and_a_purpose_and_leaves_the_rest_optional ... ok
test tool::open::tests::the_traits_fail_closed ... ok
test tool::open::tests::the_two_placements_are_two_rooms ... ok

test result: ok. 230 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running unittests src/lib.rs (target/debug/deps/bingo_schedule-a0eadfcf509a8c70)

running 115 tests
test command::schedule::tests::an_empty_store_folds_to_two_lines ... ok
test command::schedule::tests::every_entry_is_a_row_and_the_holder_is_named_under_them ... ok
test command::schedule::tests::the_spec_runs_now_and_takes_nothing ... ok
test command::wake::tests::a_session_with_nothing_standing_says_so ... ok
test command::wake::tests::a_word_it_does_not_know_changes_nothing_and_says_what_it_takes ... ok
test command::wake::tests::another_sessions_wake_is_not_this_ones ... ok
test command::wake::tests::off_ends_the_wake_and_leaves_another_sessions_standing ... ok
test command::wake::tests::the_pending_wake_is_shown_with_when_it_comes_and_what_it_says ... ok
test command::wake::tests::the_spec_runs_now_and_takes_one_word ... ok
test entry::tests::a_file_that_names_a_session_still_reads_and_the_key_means_nothing ... ok
test entry::tests::a_fire_is_reckoned_from_the_last_one_and_the_first_from_the_day_it_was_written ... ok
test entry::tests::a_once_spends_itself_and_a_disabled_entry_has_no_next_fire ... ok
test entry::tests::an_entry_is_the_json_a_person_can_hand_edit ... ok
test entry::tests::an_entry_names_one_session_of_its_own ... ok
test entry::tests::an_entry_that_will_not_parse_is_an_error_and_not_a_default ... ok
test entry::tests::what_was_never_set_is_not_written_and_reads_back_unset ... ok
test guide::tests::the_page_is_one_line_of_description_and_a_body_worth_reading ... ok
test guide::tests::the_page_names_every_column_a_listing_shows ... ok
test guide::tests::the_page_names_every_command_word_a_person_may_type ... ok
test guide::tests::the_page_names_every_tool_this_plugin_registers ... ok
test guide::tests::the_page_names_the_setting_that_turns_wakes_off ... ok
test guide::tests::the_settings_example_is_toml_and_no_json_is_left ... ok
test hook::tests::a_new_process_clears_the_dead_wake_and_its_status_slot ... ok
test lock::tests::the_scheduler_protocol_is_unchanged ... ok
test plugin_tests::a_person_who_turned_wakes_off_is_offered_no_wake_tool ... ok
test plugin_tests::a_plugin_that_never_registered_starts_and_stops_without_a_store ... ok
test plugin_tests::a_plugin_that_started_holds_the_store_and_gives_it_back_on_stop ... ok
test plugin_tests::registering_reads_nothing_and_contributes_what_the_manifest_promises ... ok
test plugin_tests::the_manifest_says_what_it_provides_and_claims_no_settings ... ok
test plugin_tests::the_settings_slice_says_whether_the_model_may_wake_itself ... ok
test render::tests::a_disabled_entry_has_no_next_fire_and_says_so ... ok
test render::tests::a_file_that_could_not_be_read_is_said_out_loud ... ok
test render::tests::a_fire_that_opened_no_turn_is_shown_where_a_person_looks ... ok
test render::tests::a_head_is_the_first_line_and_says_when_it_left_something_out ... ok
test render::tests::a_row_says_when_it_fires_next_and_what_it_will_say ... ok
test render::tests::an_empty_store_is_one_line_and_still_says_who_runs_it ... ok
test render::tests::every_entry_is_a_row_and_the_holder_is_the_line_under_them ... ok
test runner::tests::a_disabled_entry_is_neither_due_nor_waited_for ... ok
test runner::tests::a_fire_already_past_is_waited_for_no_time_at_all ... ok
test runner::tests::a_fire_opens_the_entry_s_own_session_at_the_entry_s_directory ... ok
test runner::tests::a_schedule_of_a_persons_own_is_written_down_rather_than_forgotten ... ok
test runner::tests::a_store_with_nothing_ahead_of_it_still_looks_again ... ok
test runner::tests::an_entry_whose_hour_has_come_is_due_and_one_that_is_late_is_due_once ... ok
test runner::tests::nothing_is_due_before_its_time_and_the_soonest_is_what_to_wake_for ... ok
test runner::tests::the_soonest_of_many_is_the_one_to_wake_for ... ok
test runner::voice_tests::producer_stamps_its_voice ... ok
test schedules::tests::a_cancelled_standby_does_not_take_over ... ok
test schedules::tests::a_process_that_has_not_started_holds_nothing ... ok
test schedules::tests::an_aborted_task_releases_ownership_without_a_stale_running_flag ... ok
test schedules::tests::an_existing_standby_takes_over_after_the_owner_stops ... ok
test schedules::tests::shutdown_keeps_ownership_until_in_flight_dispatch_finishes ... ok
test schedules::tests::storage_errors_are_reported_and_retried_after_the_store_is_repaired ... mbx[progress]: 240s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test spec::tests::a_daily_at_midnight_crosses_the_day_and_the_month_and_the_year ... ok
test spec::tests::a_daily_fires_today_when_the_hour_is_still_to_come_and_tomorrow_when_it_is_not ... ok
test spec::tests::a_daily_is_within_a_day_and_a_bit_of_now ... ok
test spec::tests::a_day_is_not_a_unit_but_daily_at_is_named_when_one_is_asked_for ... ok
test spec::tests::a_next_fire_is_always_after_the_moment_it_is_reckoned_from ... ok
test spec::tests::a_once_fires_when_it_comes_and_never_again ... ok
test spec::tests::a_spec_survives_the_words_it_is_written_in ... ok
test spec::tests::a_written_length_of_time_is_the_seconds_it_names ... ok
test spec::tests::an_hour_daylight_saving_deleted_fires_at_the_hour_that_replaced_it ... ok
test spec::tests::an_hour_daylight_saving_repeated_fires_once ... ok
test spec::tests::an_interval_across_the_jump_is_the_elapsed_time_it_says ... ok
test spec::tests::an_interval_fires_that_long_after_the_moment_it_is_given ... ok
test spec::tests::an_interval_is_exactly_as_far_away_as_it_says ... ok
test spec::tests::an_interval_reads_back_in_the_unit_that_divides_it ... ok
test spec::tests::an_offset_instant_reads_back_as_the_instant_it_is ... ok
test spec::tests::the_three_forms_round_trip_through_their_own_words ... ok
test spec::tests::what_is_not_a_spec_says_what_one_is ... ok
test store::tests::a_file_nobody_can_parse_costs_one_entry_and_is_named ... ok
test store::tests::a_minted_id_is_free_in_this_store ... ok
test store::tests::a_store_that_does_not_exist_is_an_empty_shelf ... ok
test store::tests::an_entry_round_trips_through_the_directory_under_its_id ... ok
test store::tests::saving_the_same_id_replaces_the_file_and_forgetting_removes_it ... ok
test store::tests::the_shelf_is_in_id_order_whatever_the_directory_says ... ok
test tools::create::tests::a_call_may_name_its_own_directory_and_mode ... ok
test tools::create::tests::a_card_is_not_drawn_for_a_call_that_would_be_refused ... ok
test tools::create::tests::a_schedule_becomes_one_file_named_by_its_minted_id ... ok
test tools::create::tests::a_schedule_with_nothing_to_say_is_refused ... ok
test tools::create::tests::a_spec_the_grammar_cannot_read_writes_nothing_and_says_what_one_is ... ok
test tools::create::tests::the_card_shows_the_entry_it_would_write ... ok
test tools::create::tests::the_receipt_says_when_it_fires_and_whether_anything_will_fire_it ... ok
test tools::create::tests::the_spec_is_an_edit_and_asks_before_it_writes ... ok
test tools::forget::tests::an_ambiguous_prefix_deletes_nothing ... ok
test tools::forget::tests::an_id_nobody_has_is_an_answer_and_not_a_failure ... ok
test tools::forget::tests::one_entry_goes_and_the_others_stay ... ok
test tools::forget::tests::the_card_shows_the_file_that_would_go ... ok
test tools::forget::tests::the_spec_is_destructive ... ok
test tools::list::tests::an_empty_store_says_so_and_says_who_would_run_it ... ok
test tools::list::tests::every_entry_is_a_row_the_model_can_name ... ok
test tools::list::tests::the_spec_is_read_only ... ok
test tools::tests::a_unique_prefix_names_one_schedule ... ok
test tools::tests::an_ambiguous_prefix_lists_what_it_could_have_meant ... ok
test tools::tests::an_unknown_prefix_names_the_tool_that_lists_them ... ok
test tools::wake::tests::a_length_of_time_reads_back_in_the_words_the_grammar_takes ... ok
test tools::wake::tests::a_second_call_takes_the_first_wakes_place ... ok
test tools::wake::tests::a_wake_outside_the_bounds_is_held_and_the_result_says_so ... ok
test tools::wake::tests::a_wake_stands_on_the_calling_session_and_touches_no_file ... ok
test tools::wake::tests::stop_leaves_none_standing_and_says_so_when_none_did ... ok
test tools::wake::tests::the_spec_teaches_the_loop_and_asks_nobody ... ok
test tools::wake::tests::what_a_call_wants_is_decided_before_anything_is_read ... ok
test tools::wake::tests::words_the_tool_cannot_read_write_nothing_and_say_what_it_takes ... ok
test wake::tests::a_wake_comes_this_long_from_now_and_says_the_note ... ok
test wake::tests::a_woken_turn_says_it_is_a_wake_and_names_nobody ... ok
test wake::tests::an_interval_inside_the_bounds_is_the_one_that_was_asked_for ... ok
test wake::tests::an_interval_outside_them_is_held_and_says_what_was_asked_for ... ok
test wake::tests::slots_place_the_wake_countdown_with_its_record ... ok
test wake::tests::what_a_surface_is_told_is_when_it_comes_and_what_it_says ... ok
test wake::voice_tests::producer_stamps_its_voice ... ok
test wakes::tests::a_cancelled_loop_does_not_start_a_due_delivery ... ok
test wakes::tests::a_due_wake_lands_on_its_session_and_is_taken_back_from_the_screen_first ... ok
test wakes::tests::one_wake_stands_per_session_and_the_next_takes_its_place ... ok
test wakes::tests::the_loop_delivers_a_wake_the_moment_it_is_set_for ... ok
test wakes::tests::the_sleep_ends_when_the_wake_is_due_and_never_runs_past_the_longest_wake ... ok
test wakes::tests::what_is_due_is_taken_out_soonest_first_and_the_rest_is_waited_for ... ok

test result: ok. 115 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.21s

     Running tests/shelf.rs (target/debug/deps/shelf-ee09cfeb94325c00)

running 1 test
test pre_m129_schedule_bytes_survive_loading_and_saving ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/bingo_schema_ts-7051c734b3bd2cb5)

running 2 tests
test tests::definitions_are_sorted_and_references_stay_references ... ok
test tests::small_schemas_render_without_runtime_types ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/bingo_sdk-44e74631790c7313)

running 133 tests
test bytes::tests::the_size_is_decimal_with_one_decimal_place ... ok
test catalogue::matcher::tests::pinned_missing_and_changed_fields_are_reported ... ok
test catalogue::matcher::tests::shared_patterns_and_scalar_matchers ... ok
test catalogue::payload::tests::arguments_cannot_replace_the_envelope_identity ... ok
test catalogue::tests::the_catalogue_names_both_turn_edges ... ok
test catalogue::tests::the_committed_catalogue_is_generated ... ok
test command::tests::a_verb_carries_the_spec_of_the_word_after_it ... ok
test command::tests::a_word_list_crosses_the_wire_as_its_kind_and_its_values ... ok
test compactor::tests::a_failed_summary_preserves_observed_usage_on_the_wire ... ok
test contributor::tests::a_changed_snapshot_compares_with_the_latest_visible_one ... ok
test contributor::tests::a_first_snapshot_uses_the_existing_user_piece_wire_form ... ok
test contributor::tests::a_snapshot_is_reissued_when_its_record_left_visible_context ... ok
test contributor::tests::an_unchanged_snapshot_is_silent_even_after_a_journal_round_trip ... ok
test error::tests::interrupted_is_a_stable_wire_code ... ok
test error::tests::the_wire_form_and_the_str_form_agree ... ok
test event::tests::a_bare_question_on_the_wire_is_the_shape_it_always_was ... ok
test event::tests::a_call_fails_on_its_status_or_on_the_output_it_marked ... ok
test event::tests::a_choice_carries_the_words_typed_beside_the_ticks ... ok
test event::tests::a_confirm_offers_both_ways_out_and_a_form_offers_no_rung ... ok
test event::tests::a_form_is_answered_by_every_questions_role_option ... ok
test event::tests::a_form_one_of_whose_questions_is_unmarked_is_a_persons_alone ... ok
test event::tests::a_login_offers_the_way_out ... ok
test event::tests::a_permission_offers_the_answers_it_says_it_takes ... ok
test event::tests::a_permission_with_no_scope_offers_no_session_rule ... ok
test event::tests::a_question_names_the_option_each_role_is_answered_with ... ok
test event::tests::a_question_nobody_marked_is_a_persons_alone ... ok
test event::tests::a_question_offers_one_rung_per_option_in_the_askers_own_words ... ok
test event::tests::an_unmarked_option_is_on_the_wire_as_it_was ... ok
test event::tests::contributor_context_does_not_count_as_a_conversation_message ... ok
test event::tests::every_event_variant_has_a_pinned_wire_form ... ok
test event::tests::only_deltas_notices_and_lag_are_ephemeral ... ok
test hook::tests::a_matcher_that_wants_everything_says_nothing ... ok
test hook::tests::an_outcome_crosses_tagged_by_its_kind ... ok
test hook::tests::no_spelling_of_allow_is_an_outcome ... ok
test host::tests::a_held_line_carries_the_word_and_answers_to_it ... ok
test host::tests::a_line_that_says_nothing_about_delivery_is_on_the_wire_as_it_was ... ok
test host::tests::a_spec_from_before_the_field_reads_as_absent ... ok
test host::tests::a_spec_tells_inheriting_a_level_apart_from_asking_for_none ... ok
test host::tests::an_action_is_never_held ... ok
test ids::tests::ids_are_prefixed_and_minted_in_order ... ok
test ids::tests::ids_round_trip_as_plain_strings ... ok
test intercept::stream::tests::a_failed_wrapper_cannot_consume_the_continuations_events ... ok
test model::tests::a_known_extension_resolves_and_an_unknown_one_does_not ... ok
test model::tests::a_level_is_spelled_one_way_for_a_person_and_another_on_the_wire ... ok
test model::tests::a_path_rides_with_the_picture_and_is_absent_when_there_is_none ... ok
test model::tests::a_payload_that_is_only_padding_is_bounded_not_a_panic ... ok
test model::tests::a_reading_and_a_cut_are_tagged_and_round_trip ... ok
test model::tests::a_tool_result_nests_parts_and_round_trips ... ok
test model::tests::an_endpoint_that_says_nothing_holds_nothing ... ok
test model::tests::an_image_part_tags_and_flattens_beside_it ... ok
test model::tests::cloning_an_image_shares_its_base64_allocation ... ok
test model::tests::decoded_len_reads_the_base64_length_without_decoding ... ok
test model::tests::empty_provider_metadata_is_omitted ... ok
test model::tests::from_bytes_encodes_a_known_type_within_the_cap ... ok
test model::tests::from_bytes_refuses_an_unknown_type_and_an_oversized_payload ... ok
test model::tests::off_is_the_seventh_word_and_the_list_is_lowest_first ... ok
test model::tests::read_loads_a_known_picture_off_disk ... ok
test model::tests::read_rejects_a_path_the_table_does_not_know ... ok
test model::tests::retryability_follows_the_error_kind ... ok
test model::tests::the_table_reads_both_ways ... ok
test page::tests::a_page_is_three_strings_and_nothing_else ... ok
test page::tests::a_plugin_s_pages_are_found_under_its_own_id ... ok
test plugin::tests::a_built_in_status_names_the_build_and_carries_its_reason ... ok
test plugin::tests::a_registrar_answers_with_the_switches_it_was_built_with ... ok
test service::tests::a_key_that_is_taken_stays_its_first_owner_s ... ok
test service::tests::a_service_family_is_found_by_its_computed_name ... ok
test service::tests::a_service_key_carries_a_trait_object_type ... ok
test service::tests::a_service_with_no_wire_face_is_still_a_service_in_process ... ok
test service::tests::an_opened_service_answers_by_type_and_by_wire ... ok
test service::tests::face_names_cannot_be_claimed_twice ... ok
test slot::tests::row_caption_is_optional_in_the_schema_and_wire ... ok
test slot::tests::slot_placed_orders_all_lanes_by_plugin_then_kind ... ok
test slot::tests::slot_placed_skips_only_the_unreadable_slot ... ok
test slot::tests::slot_variants_round_trip_the_golden_json ... ok
test slot::tests::slot_with_keeps_the_record_and_leaves_null_alone ... ok
test state::tests::a_lag_marker_leaves_seq_where_the_last_applied_frame_put_it ... ok
test state::tests::a_message_is_counted_where_it_is_completed_and_only_there ... ok
test state::tests::a_session_frame_after_a_close_opens_the_session_again ... ok
test state::tests::a_session_frame_hands_the_count_on ... ok
test state::tests::a_signal_keeps_the_latest_per_kind_and_null_removes_it ... ok
test state::tests::deltas_accumulate_until_the_authoritative_completion ... ok
test state::tests::interactions_open_and_close_by_id ... ok
test state::tests::only_prose_and_answers_are_messages ... ok
test state::tests::rewind_drops_items_and_bumps_the_generation ... ok
test state::tests::side_usage_adds_without_advancing_the_round_or_context ... ok
test state::tests::stale_and_duplicate_frames_are_ignored ... ok
test state::tests::transcript_snapshot_keeps_the_wire_bytes ... ok
test store::checkpoint::tests::checkpoint_codec_preserves_opaque_item_bytes ... ok
test store::checkpoint::tests::checkpoint_defaults_replay_without_losing_frames ... ok
test store::checkpoint::tests::checkpoint_fixture_preserves_model_and_client_orders ... ok
test surface::tests::a_turn_ends_a_run_the_way_a_shell_reads_it ... ok
test testing::tests::a_service_has_one_owner_and_both_faces ... ok
test testing::tests::gateway_events_are_driven_by_the_test ... ok
test testing::tests::scripts_can_suspend_and_observe_the_recorded_arguments ... ok
test testing::tests::scripts_override_the_session_table_without_changing_it ... ok
test testing::tests::seeded_sessions_open_detach_and_delete_without_losing_the_summary ... ok
test testing::tests::unscripted_calls_keep_the_no_host_defaults_and_are_recorded ... ok
test tokens::extreme_pixels::aggregate_pixel_estimates_saturate ... ok
test tokens::pixel_contract::dimensions_set_the_picture_estimate ... ok
test tokens::tests::a_request_counts_its_system_messages_and_tool_schemas ... ok
test tokens::tests::an_image_costs_a_flat_sixteen_hundred_wherever_it_sits ... ok
test tokens::tests::four_ascii_characters_are_one_token_and_a_cjk_character_is_one ... ok
test tool::tests::defaults_fail_closed ... ok
test tool::tests::input_schema_is_a_bare_object_schema ... ok
test tool::tests::nested_types_are_inlined_not_referenced ... ok
test transcript::tests::a_delta_does_not_mutate_an_existing_snapshot ... ok
test transcript::tests::checkpoint_window_can_restore_full_client_order_without_losing_live_bodies ... ok
test transcript::tests::checkpoint_window_keeps_model_only_items_out_of_client_history ... ok
test transcript::tests::client_snapshot_deserialization_rebuilds_the_lookup ... ok
test transcript::tests::open_items_belong_to_the_turn_and_exclude_terminal_or_removed_items ... ok
test transcript::tests::ten_thousand_items_share_bodies_in_a_snapshot ... ok
test transcript::tests::transcript_matches_client_and_replay ... ok
test tree::tests::a_child_announces_then_folds_and_unknown_frames_are_stale ... ok
test tree::tests::deletion_removes_descendants_and_late_frames_need_a_new_announcement ... ok
test tree::tests::gateway_removal_returns_the_subtree_and_preserves_the_root ... ok
test tree::tests::other_closes_preserve_the_state_for_reopening ... ok
test tree::tests::stale_deletion_does_not_remove_a_child ... ok
test tree::tests::tree_matches_replay_and_removal_oracle ... ok
test view::tests::a_caught_node_round_trips_without_nesting_itself_again ... ok
test view::tests::a_cell_that_would_break_its_row_is_written_as_markdown ... ok
test view::tests::a_custom_node_writes_the_reserved_shape_and_reads_back_as_itself ... ok
test view::tests::a_kind_this_sdk_has_never_heard_of_lands_in_custom_whole ... ok
test view::tests::a_known_kind_that_is_malformed_is_still_an_error ... ok
test view::tests::a_table_inside_another_node_is_drawn_by_the_same_rule ... ok
test view::tests::a_table_is_a_table_to_a_surface_that_draws_markdown ... ok
test view::tests::a_table_with_no_rows_keeps_its_header ... ok
test view::tests::an_unbounded_progress_folds_to_its_count ... ok
test view::tests::an_unknown_kind_with_no_fold_folds_to_its_name ... ok
test view::tests::an_unknown_node_inside_a_known_one_folds_in_place ... ok
test view::tests::every_line_ending_stays_inside_its_table_cell ... ok
test view::tests::every_node_folds_to_text ... ok
test view::tests::every_node_has_a_wire_shape_and_round_trips ... ok
test view::tests::the_known_kinds_are_the_vocabulary ... ok

test result: ok. 133 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 11.88s

     Running tests/intercept.rs (target/debug/deps/intercept-cce23c37d4bd0056)

running 2 tests
test a_continuation_passes_its_input_once ... ok
test the_seven_points_use_the_catalogue_names ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/record.rs (target/debug/deps/record-e329bd49d0e84696)

running 3 tests
test every_event_variant_has_a_golden_fixture ... ok
test every_item_variant_has_a_golden_fixture ... ok
test quiet_tool_calls_keep_their_flag_and_old_calls_keep_their_shape ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/unknown_record.rs (target/debug/deps/unknown_record-c17603c1f92334cc)

running 6 tests
test buffered_payloads_and_values_keep_the_unknown_data ... ok
test invalid_envelopes_are_not_opaque_payloads ... ok
test the_transcript_retains_opaque_items_through_snapshots_and_updates ... ok
test unknown_and_malformed_events_keep_their_bytes ... ok
test unknown_and_malformed_items_keep_their_envelope_and_bytes ... ok
test unknown_events_advance_sequence_without_changing_the_view ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/voice.rs (target/debug/deps/voice-f444d4c53bd59955)

running 2 tests
test voice_absence_keeps_the_original_record_readable_and_unchanged ... ok
test voice_round_trips_the_golden_json ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/bingo_services-3795cce8fd047a4e)

running 5 tests
test mcp::tests::a_typed_mcp_owner_is_found_without_a_wire_face ... ok
test mcp::tests::the_mcp_wire_contract_round_trips ... ok
test room_post::tests::both_service_faces_keep_the_golden_post_contract ... ok
test team::tests::a_typed_team_owner_is_found_without_a_wire_face ... ok
test team::tests::the_team_wire_contract_round_trips ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/lib.rs (target/debug/deps/bingo_shelf-68c9405a89f9bedc)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/diff.rs (target/debug/deps/diff-e29d3b1ef32f7b03)

running 3 tests
test a_changed_line_is_a_hunk_under_the_file_name ... ok
test a_new_file_is_all_additions_under_its_name ... ok
test nothing_changed_is_no_diff_at_all ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/id.rs (target/debug/deps/id-946c8c30b62bf34e)

running 3 tests
test a_minted_id_is_short_lowercase_and_alphanumeric ... ok
test a_unique_prefix_names_one_entry ... ok
test an_exact_id_wins_over_the_ids_it_prefixes ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/store.rs (target/debug/deps/store-e1d5d3efc0068423)

running 5 tests
test a_failed_rename_removes_the_temp_file ... ok
test a_missing_store_is_empty_and_a_round_trip_leaves_no_temp ... ok
test entries_and_unreadable_files_are_sorted_and_dotfiles_are_skipped ... ok
test loading_never_rewrites_hand_edited_bytes_and_renaming_changes_the_id ... ok
test saving_replaces_deleting_is_idempotent_and_minted_ids_are_free ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/lib.rs (target/debug/deps/bingo_skills-a61bd979e8848e02)

running 86 tests
test bundled::tests::a_build_whose_plugins_wrote_nothing_ends_at_the_map ... ok
test bundled::tests::the_guide_describes_this_product ... ok
test bundled::tests::the_guide_ends_in_the_pages_this_build_found ... ok
test bundled::tests::the_guide_names_the_settings_file_the_migration_and_the_switch ... ok
test bundled::tests::the_guide_parses_and_says_what_it_is_for ... ok
test bundled::tests::the_guide_stays_short_enough_to_read ... ok
test command::tests::a_skill_command_becomes_the_prompt_its_body_spells ... ok
test command::tests::a_skill_that_declared_no_hint_offers_its_description_instead ... ok
test command::tests::the_source_mints_one_command_per_skill ... ok
test command::tests::the_spec_waits_for_the_turn_and_takes_free_text ... ok
test contributor::tests::every_skill_is_listed_once_with_what_it_is_for ... ok
test contributor::tests::it_comes_after_the_project_s_own_instructions ... ok
test contributor::tests::the_block_is_cacheable_because_it_changes_only_when_the_disk_does ... ok
test expand::tests::a_body_that_used_its_arguments_gets_nothing_appended ... ok
test expand::tests::a_bundled_skill_has_no_directory_to_name ... ok
test expand::tests::a_declared_name_is_the_word_at_its_position ... ok
test expand::tests::a_declared_name_with_no_word_at_its_position_becomes_empty ... ok
test expand::tests::a_named_placeholder_counts_even_when_it_expands_to_nothing ... ok
test expand::tests::a_placeholder_glued_to_a_word_is_not_a_placeholder ... ok
test expand::tests::a_value_that_looks_like_a_placeholder_is_inserted_as_text ... ok
test expand::tests::an_expansion_says_where_the_skill_lives_before_its_body ... ok
test expand::tests::an_indexed_placeholder_that_matched_nothing_does_not_count_as_used ... ok
test expand::tests::arguments_no_placeholder_asked_for_are_appended_not_dropped ... ok
test expand::tests::the_substitution_table ... ok
test expand::tests::the_whole_argument_text_keeps_its_inner_spacing ... ok
test frontmatter::tests::a_block_yaml_cannot_read_leaves_the_skill_with_its_body ... ok
test frontmatter::tests::a_fence_that_is_not_the_first_line_is_content ... ok
test frontmatter::tests::a_file_without_frontmatter_is_all_body ... ok
test frontmatter::tests::a_folded_description_is_one_line ... ok
test frontmatter::tests::a_key_this_plugin_does_not_know_is_ignored ... ok
test frontmatter::tests::a_literal_description_keeps_its_newlines ... ok
test frontmatter::tests::allowed_tools_written_as_a_list_keeps_its_entries ... ok
test frontmatter::tests::an_unclosed_block_is_content ... ok
test frontmatter::tests::arguments_written_as_one_string_are_the_same_names ... ok
test frontmatter::tests::every_field_this_plugin_reads ... ok
test frontmatter::tests::the_block_is_split_from_the_body ... ok
test frontmatter::tests::the_first_line_is_the_first_one_with_something_on_it ... ok
test guide::tests::the_page_is_one_line_of_description_and_a_body_worth_reading ... ok
test guide::tests::the_page_names_every_tool_this_plugin_registers ... ok
test guide::tests::the_page_names_the_file_and_the_variable_a_skill_is_written_with ... ok
test guide::tests::this_plugin_claims_no_settings_for_the_page_to_name ... ok
test hook::tests::both_doors_place_the_same_row_and_ignore_old_prompts ... ok
test layers::tests::a_home_inside_the_working_tree_is_read_once ... ok
test layers::tests::a_nested_directory_speaks_before_the_repository_around_it ... ok
test layers::tests::the_person_s_own_skills_come_before_any_project_s ... ok
test layers::tests::the_walk_ends_at_the_filesystem_root ... ok
test library::tests::a_library_nobody_started_still_answers_with_what_is_on_disk ... ok
test library::tests::a_nearer_project_layer_overrides_a_farther_one ... ok
test library::tests::a_new_skill_is_seen_on_the_next_look ... ok
test library::tests::a_second_look_at_an_unchanged_tree_hands_back_the_same_scan ... ok
test library::tests::a_skill_file_written_into_a_directory_that_was_already_there_is_seen ... ok
test library::tests::an_edited_skill_is_seen_on_the_next_look ... ok
test library::tests::the_pages_read_at_start_are_listed_below_the_bundled_guide ... ok
test library::tests::the_person_s_own_skill_overrides_the_project_s_of_that_name ... ok
test library::tests::two_working_directories_see_their_own_project_skills ... ok
test listing::tests::a_description_that_grew_into_a_document_is_cut ... ok
test listing::tests::a_description_written_over_several_lines_reads_as_one ... ok
test listing::tests::a_skill_with_nothing_to_say_is_still_named ... ok
test listing::tests::one_line_per_skill_name_and_description ... ok
test listing::tests::the_names_alone_are_a_sentence_a_model_can_read ... ok
test pages::tests::a_plugin_s_page_arrives_as_a_guide_skill ... ok
test pages::tests::a_plugin_that_wrote_nothing_and_a_host_with_no_catalogue_are_both_empty ... ok
test pages::tests::every_page_gathered_is_shaped_like_a_page ... ok
test pages::tests::the_pages_read_in_one_order_whatever_order_the_plugins_loaded_in ... ok
test plugin_tests::registering_reads_nothing_and_contributes_what_the_manifest_promises ... ok
test plugin_tests::the_manifest_says_what_it_provides ... ok
test scan::tests::a_directory_without_a_skill_file_is_not_a_skill ... ok
test scan::tests::a_disk_skill_overrides_the_bundled_one_of_that_name ... ok
test scan::tests::a_disk_skill_overrides_the_page_of_that_name ... ok
test scan::tests::a_layer_lists_its_skills_by_name ... ok
test scan::tests::a_layer_that_is_not_there_is_read_as_nothing_and_still_watched ... ok
test scan::tests::a_page_a_plugin_wrote_is_a_skill_below_the_bundled_ones ... ok
test scan::tests::a_symlinked_skill_directory_loads_and_a_dangling_one_does_not ... ok
test scan::tests::an_empty_machine_still_has_the_bundled_guide ... ok
test scan::tests::every_path_read_is_watched_for_the_next_look ... ok
test scan::tests::the_earlier_layer_wins_a_name ... ok
test skill::tests::a_file_with_no_frontmatter_is_named_after_its_directory ... ok
test skill::tests::a_page_carries_its_three_strings_and_invents_nothing ... ok
test skill::tests::a_skill_with_neither_description_nor_body_still_loads ... ok
test skill::tests::the_frontmatter_name_wins_over_the_directory ... ok
test skill::tests::what_is_recorded_but_not_enforced_is_still_recorded ... ok
test tool::tests::a_name_nobody_has_says_which_names_exist ... ok
test tool::tests::a_named_skill_comes_back_expanded_under_its_own_directory ... ok
test tool::tests::a_skill_called_without_arguments_still_comes_back ... ok
test tool::tests::input_the_schema_does_not_describe_is_refused ... ok
test tool::tests::it_reads_and_a_rule_may_name_the_skill_it_reads ... ok

test result: ok. 86 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

     Running unittests src/lib.rs (target/debug/deps/bingo_store_jsonl-43dd0e8adc9276af)

running 67 tests
test append_measurement::append_thousand_frames ... ignored, manual before/after measurement; no time budget
test checkpoint::tests::a_version_one_checkpoint_is_replayed_after_the_bump ... ok
test checkpoint::tests::checkpoint_corruption_replays_and_repairs ... ok
test checkpoint::tests::checkpoint_pictures_are_references_and_missing_blobs_are_notes ... ok
test checkpoint::tests::checkpoint_resume_reads_only_tail_and_preserves_the_fold ... ok
test checkpoint::tests::checkpoint_resume_seeks_past_the_prefix_and_reads_versioned_tail ... ok
test checkpoint::tests::checkpoint_writes_cannot_regress_to_an_older_sequence ... ok
test gc::tests::a_second_run_within_a_day_does_nothing ... ok
test gc::tests::a_session_nobody_touched_for_a_month_goes ... ok
test gc::tests::an_open_session_is_never_collected ... ok
test gc::tests::the_oldest_beyond_the_keep_count_go ... ok
test journal::compatibility::an_unknown_event_journal_replays ... ok
test journal::compatibility::an_unknown_item_journal_replays ... ok
test journal::compatibility::historical_shapes_stay_known ... ok
test journal::compatibility::replay_and_append_preserve_unknown_payload_bytes ... ok
test journal::compatibility::version_one_opaque_payloads_keep_their_bytes_across_the_identity_step ... ok
test journal::migrate::tests::every_version_has_exactly_one_step_to_its_successor ... ok
test journal::migrate::tests::steps_chain_from_the_lines_version_before_decoding ... ok
test journal::pictures::a_missing_blob_becomes_a_text_part ... ok
test journal::pictures::deleting_a_blob_keeps_the_size_note_and_the_session_readable ... ok
test journal::pictures::inline_and_blob_fixtures_resolve_to_the_same_image ... ok
test journal::pictures::the_planned_v2_picture_line_is_under_one_kib ... ok
test journal::pictures::the_planned_v2_picture_line_length_is_independent_of_picture_size ... ok
test journal::pictures::two_picture_appends_keep_one_blob_and_upgrade_the_journal ... ok
test journal::tests::a_clean_journal_replays_every_frame ... ok
test journal::tests::a_corrupt_middle_line_names_its_line ... ok
test journal::tests::a_file_without_a_header_is_not_a_journal ... ok
test journal::tests::a_newer_version_is_refused ... ok
test journal::tests::a_second_create_keeps_what_the_first_wrote ... ok
test journal::tests::a_torn_last_line_is_dropped ... ok
test journal::tests::since_skips_the_frames_a_client_already_has ... ok
test journal::tests::the_writer_produces_the_clean_fixture ... ok
test journal::versions::a_malformed_version_line_in_the_middle_is_corruption ... ok
test journal::versions::a_newer_version_line_is_refused_even_at_the_tail_or_before_since ... ok
test journal::versions::releasing_the_claim_forgets_the_prepared_version ... ok
test journal::versions::the_version_writer_only_appends_the_marker ... ok
test journal::versions::version_lines_between_frames_do_not_take_a_sequence_number ... ok
test layout::tests::a_directory_without_a_journal_is_not_a_session ... ok
test layout::tests::a_root_that_does_not_exist_holds_no_sessions ... ok
test layout::tests::a_session_is_a_directory_named_by_its_id ... ok
test lock::tests::a_free_lock_can_be_taken_and_given_back ... ok
test lock::tests::a_second_holder_is_told_the_session_is_open ... ok
test lock::tests::one_store_does_not_take_a_session_twice ... ok
test lock::tests::releasing_what_nobody_holds_is_not_an_error ... ok
test record::tests::a_checkpoint_with_pictures_keeps_unknown_sibling_bytes ... ok
test record::tests::nested_parts_are_blobs_but_image_shaped_tool_input_is_opaque ... ok
test runtime_contract::blocking_io_leaves_current_thread_responsive ... ok
test summary::tests::a_deleted_summary_is_rebuilt_from_the_journal_unchanged ... ok
test summary::tests::a_file_that_never_counted_is_rebuilt_rather_than_guessed ... ok
test summary::tests::a_filter_admits_by_cwd_and_by_parent ... ok
test summary::tests::a_journal_without_a_summary_frame_has_none ... ok
test summary::tests::a_rebuilt_summary_recounts_the_whole_journal ... ok
test summary::tests::an_unreadable_summary_is_rebuilt_rather_than_reported ... ok
test summary::tests::the_count_in_the_file_moves_with_a_message_and_the_journal_grows_none ... ok
test tests::a_created_session_replays_what_it_appended ... ok
test tests::a_directory_that_is_not_a_session_is_skipped ... ok
test tests::a_second_store_on_one_root_cannot_open_a_held_session ... ok
test tests::an_ephemeral_frame_never_reaches_the_journal ... ok
test tests::an_unknown_session_is_not_found ... ok
test tests::deleting_every_summary_loses_nothing ... ok
test tests::list_answers_most_recently_updated_first ... ok
test tests::list_honours_cwd_parent_and_limit ... ok
test tests::summary_failure_counting_a_message_keeps_the_journal_line ... ok
test tests::summary_failure_does_not_reject_a_written_journal_line ... ok
test tests::the_holder_can_delete_what_it_holds ... ok
test tests::the_manifest_says_what_it_provides ... ok
test tests::the_plugin_registers_one_store_under_the_data_directory ... ok

test result: ok. 66 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 1.45s

     Running unittests src/lib.rs (target/debug/deps/bingo_surface_print-48ade4659c1cc859)

running 103 tests
test hosted::tests::a_control_request_this_surface_cannot_serve_is_refused ... ok
test hosted::tests::a_deny_carries_the_hosts_message_back_to_the_model ... ok
test hosted::tests::a_form_stays_refused_too ... ok
test hosted::tests::a_junk_line_is_a_diagnostic_and_the_run_goes_on ... ok
test hosted::tests::a_permission_is_asked_of_the_host_and_an_allow_reaches_the_kernel ... ok
test hosted::tests::a_question_stays_refused_even_when_the_host_answers_permissions ... ok
test hosted::tests::a_verdict_for_no_open_request_is_a_diagnostic ... ok
test hosted::tests::an_allow_that_rewrites_the_call_is_a_denial ... ok
test hosted::tests::an_echoed_input_is_the_same_call_and_still_an_allow ... ok
test hosted::tests::an_interrupt_stops_the_running_turn_and_is_acknowledged ... ok
test hosted::tests::each_turn_writes_one_result_line_and_no_prose ... ok
test hosted::tests::every_user_line_is_a_turn_of_its_own ... ok
test hosted::tests::the_prompt_argument_is_the_first_turn_of_a_hosted_run ... ok
test hosted::tests::without_a_prompt_tool_a_permission_is_refused ... ok
test input::tests::a_deny_carries_the_message_the_model_will_read ... ok
test input::tests::a_permission_request_names_the_tool_and_the_call ... ok
test input::tests::a_scope_that_is_not_a_tool_call_keeps_its_tool_and_a_null_rule ... ok
test input::tests::a_session_scope_becomes_the_rule_it_would_install ... ok
test input::tests::a_user_line_that_is_only_a_picture_is_a_prompt ... ok
test input::tests::a_user_line_with_no_content_says_so ... ok
test input::tests::a_user_line_with_no_text_is_no_prompt ... ok
test input::tests::a_user_line_with_string_content_is_a_prompt ... ok
test input::tests::an_allow_carries_the_input_the_host_would_run ... ok
test input::tests::an_allow_without_an_updated_input_is_still_an_allow ... ok
test input::tests::an_image_block_of_a_type_no_decoder_reads_is_said ... ok
test input::tests::an_image_block_of_a_wider_type_arrives_as_png ... ok
test input::tests::an_image_block_this_surface_cannot_carry_is_said ... ok
test input::tests::an_interrupt_is_a_control_request_with_its_id ... ok
test input::tests::an_unknown_control_request_keeps_its_id_so_it_can_be_refused ... ok
test input::tests::every_control_request_is_answered_by_its_id ... ok
test input::tests::everything_that_is_not_an_allow_is_a_denial ... ok
test input::tests::junk_is_an_error_a_person_can_read_and_never_a_panic ... ok
test input::tests::only_the_stdio_prompt_tool_is_this_surface_s_to_answer ... ok
test input::tests::the_input_format_argument_picks_the_source ... ok
test input::tests::the_text_blocks_are_the_prompt_and_the_image_blocks_its_pictures ... ok
test render::tests::a_commands_receipt_lands_on_stdout_as_its_message_or_its_fold ... ok
test render::tests::a_completion_longer_than_the_deltas_prints_only_the_remainder ... ok
test render::tests::a_display_of_a_kind_this_surface_never_learned_prints_its_fold ... ok
test render::tests::a_display_view_is_printed_as_its_fold_under_the_verdict ... ok
test render::tests::a_failed_tool_reports_error_with_its_duration ... ok
test render::tests::a_failed_turn_writes_one_error_line_to_stderr ... ok
test render::tests::a_long_error_message_is_cut_at_two_hundred_characters ... ok
test render::tests::a_shell_line_reports_beside_the_answer_and_never_in_it ... ok
test render::tests::a_stale_delta_for_a_tool_item_never_reaches_stdout ... ok
test render::tests::a_user_item_is_not_echoed ... ok
test render::tests::an_item_completed_with_no_deltas_at_all_still_prints ... ok
test render::tests::json_mode_puts_one_frame_per_line_on_stdout_and_nothing_else ... ok
test render::tests::json_mode_still_reports_a_failed_turn_on_stderr ... ok
test render::tests::opaque_records_are_silent_except_in_raw_json ... ok
test render::tests::text_and_json_report_the_root_alone_and_the_envelope_the_tree ... ok
test render::tests::text_mode_prints_the_deltas_once_and_ends_with_one_newline ... ok
test render::tests::the_output_format_argument_picks_the_mode ... ok
test render::tests::the_two_registers_of_a_report ... ok
test render::tests::tool_calls_notices_and_retries_are_stderr_only ... ok
test stream_json::tests::a_budget_failure_is_error_max_turns ... ok
test stream_json::tests::a_failed_tool_call_says_so_in_its_result ... ok
test stream_json::tests::a_failed_turn_is_an_error_result_carrying_the_message ... ok
test stream_json::tests::a_session_with_no_model_omits_the_field_rather_than_inventing_one ... ok
test stream_json::tests::a_shell_line_is_a_user_message_of_the_line_and_its_output ... ok
test stream_json::tests::a_sub_sessions_lines_name_the_root_and_the_call_that_spawned_it ... ok
test stream_json::tests::a_text_turn_is_the_preamble_the_message_and_the_result ... ok
test stream_json::tests::a_tool_round_is_a_tool_use_then_a_tool_result ... ok
test stream_json::tests::a_turn_that_never_started_still_ends_with_a_result ... ok
test stream_json::tests::an_empty_assistant_completion_writes_no_line ... ok
test stream_json::tests::an_image_in_a_tool_result_becomes_a_block_array ... ok
test stream_json::tests::an_interrupted_turn_is_an_error_during_execution ... ok
test stream_json::tests::deltas_and_notices_never_reach_stdout ... ok
test stream_json::tests::every_line_is_an_envelope_naming_the_session ... ok
test stream_json::tests::side_charges_are_events_and_never_model_rounds ... ok
test stream_json::tests::stderr_is_what_json_mode_would_have_written ... ok
test stream_json::tests::the_preamble_is_the_first_line ... ok
test stream_json::tests::the_result_reports_the_rounds_the_duration_and_the_tokens ... ok
test tests::a_deleted_child_leaves_the_print_fold ... ok
test tests::a_failed_turn_reports_the_error_and_exits_one ... ok
test tests::a_form_is_put_one_question_at_a_time_and_answered_once ... ok
test tests::a_json_run_reports_the_root_alone_while_answering_the_tree ... ok
test tests::a_person_at_the_terminal_reads_prose_not_the_machine_line ... ok
test tests::a_question_answered_with_nonsense_is_cancelled ... ok
test tests::a_question_is_answered_with_the_option_the_person_typed ... ok
test tests::a_question_of_a_form_left_blank_is_skipped_and_the_rest_still_land ... ok
test tests::a_rejected_submit_is_one_error_line_and_exit_one ... ok
test tests::a_relative_image_is_read_from_the_sessions_directory ... ok
test tests::a_rung_the_interaction_does_not_carry_is_not_offered ... ok
test tests::a_session_closed_before_the_turn_ends_is_an_error_exit ... ok
test tests::a_stream_that_ends_without_a_completion_is_an_error_exit ... ok
test tests::a_sub_sessions_turn_ending_does_not_end_the_run ... ok
test tests::a_text_run_refuses_a_sub_sessions_prompt_and_keeps_its_prose_off_stdout ... ok
test tests::a_text_turn_prints_prose_and_exits_zero ... ok
test tests::a_tool_round_keeps_stdout_prose_only ... ok
test tests::a_user_item_is_never_echoed_to_stdout ... ok
test tests::always_installs_the_session_scope_the_kernel_offered ... ok
test tests::an_empty_prompt_is_invalid_input ... ok
test tests::an_image_of_a_wider_type_is_png_by_the_time_it_is_submitted ... ok
test tests::an_image_that_does_not_read_is_invalid_input_before_any_turn ... ok
test tests::an_image_that_reads_goes_beside_the_prompt ... ok
test tests::an_interrupted_turn_exits_with_the_signal_code ... ok
test tests::anything_else_denies ... ok
test tests::at_a_terminal_yes_allows_the_call_once ... ok
test tests::json_mode_writes_one_frame_per_line_and_no_prose ... ok
test tests::stream_json_mode_opens_with_the_preamble_and_ends_with_the_result ... ok
test tests::the_plugin_registers_the_print_surface ... ok
test tests::the_prompt_is_read_from_stdin_when_the_command_line_has_none ... ok
test tests::without_a_terminal_a_permission_is_denied_immediately ... ok

test result: ok. 103 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s

     Running unittests src/lib.rs (target/debug/deps/bingo_surface_rpc-440e5766e9f2edd0)

running 15 tests
test codec::tests::a_kernel_error_keeps_its_code_across_the_wire ... ok
test codec::tests::a_protocol_error_comes_back_as_invalid_input ... ok
test methods::tests::initialize_reports_the_table_it_dispatches_from ... ok
test methods::tests::no_name_is_used_twice ... ok
test methods::tests::the_wire_has_explicit_methods_and_notifications ... ok
test schema::tests::every_method_reference_resolves ... ok
test schema::tests::every_property_is_camel_case ... ok
test schema::tests::the_committed_schema_is_this_document ... ok
test server::bounded::tests::child_event_pin_belongs_to_its_open_root_and_exact_session ... ok
test server::bounded::tests::quota_exhaustion_never_mints_a_fake_available_token ... ok
test server::discovery::tests::escaped_display_fields_fit_by_omission_before_rejecting_the_true_head ... ok
test session::tests::a_rewound_frame_updates_the_generation_of_later_references ... ok
test session::tests::event_reference_keeps_the_actual_event_tag_and_identity ... ok
test tests::only_stdio_is_a_transport ... ok
test tests::the_surface_owns_stdio_alone ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s

     Running tests/wire/main.rs (target/debug/deps/wire-7ae7f4e474957b31)

running 42 tests
test a_delivery_an_extension_and_a_signal_reach_the_kernel_verbatim ... ok
test a_kernel_error_carries_its_stable_code ... ok
test a_lagged_frame_travels_like_any_other ... ok
test a_line_may_ask_on_the_wire_to_wait_for_the_turn_to_end ... ok
test a_line_that_is_not_json_is_a_parse_error ... ok
test a_line_that_is_not_json_rpc_is_an_invalid_request ... ok
test a_remote_kernel_folds_to_the_state_the_host_scripted ... ok
test a_submitted_image_reaches_the_kernel_beside_the_text ... ok
test a_tree_attachment_routes_a_childs_frames_to_the_root_stream ... ok
test a_write_reaches_the_actor_and_answers_nothing ... ok
test a_write_to_a_session_that_is_not_open_is_not_found ... ok
test an_interrupted_intent_crosses_ndjson_as_a_rejection ... ok
test an_unknown_method_is_not_found ... ok
test bounded::a_near_limit_string_request_id_gets_a_small_null_id_error ... mbx[progress]: 255s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test bounded::a_pin_budget_failure_is_explicit_and_not_a_fake_empty_field ... ok
test bounded::a_valid_child_page_budget_that_fits_no_id_fails_instead_of_looping ... ok
test bounded::bounded_extensions_are_discoverable_without_changing_legacy_defaults ... ok
test bounded::bounded_gateway_created_event_invalidates_head_without_a_large_summary ... ok
test bounded::bounded_tree_open_declares_all_old_descendants_unloaded ... ok
test bounded::canonical_remote_kernel_rejects_bounded_refs_instead_of_dropping_them ... ok
test bounded::child_ids_page_without_repeating_or_serializing_huge_titles ... ok
test bounded::event_above_the_desktop_queue_budget_is_read_only_on_demand ... ok
test bounded::first_connect_lists_verified_heads_without_large_root_title_or_key ... ok
test bounded::history_cursor_can_represent_an_empty_initial_window_without_a_fake_item_id ... ok
test bounded::hundred_megabyte_event_is_not_pushed_on_open ... ignored, 100MiB pinned-event stress; run explicitly after bounded server implementation
test bounded::invalid_byte_budgets_fail_boundedly_instead_of_returning_empty_pages ... ok
test bounded::oversized_event_reference_does_not_block_next_lifecycle_event ... ok
test bounded::oversized_history_item_has_no_cursor_loop_and_is_exact_on_demand ... mbx[progress]: 270s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test bounded::oversized_nonitem_open_retains_trusted_cwd_and_serves_immutable_field_by_part ... ok
test bounded::same_generation_item_change_never_mints_a_token_for_wrong_bytes ... ok
test closing_stops_the_forwarder ... ok
test events_since_resends_from_that_seq ... ok
test every_method_in_the_table_is_dispatched ... ok
test every_other_method_before_initialize_is_not_initialized ... ok
test history_and_catalog_answer_sdk_types ... ok
test host::session_open_fixture_keeps_its_wire_bytes ... ok
test initialize_answers_the_table_it_dispatches_from ... ok
test initializing_twice_is_an_invalid_request ... ok
test params_that_do_not_fit_are_invalid_params ... ok
test the_gateway_is_subscribed_once ... ok
test the_open_reply_precedes_the_first_event ... ok
test the_remote_catalog_answers_on_a_multi_threaded_runtime ... ok

test result: ok. 41 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 24.77s

     Running unittests src/lib.rs (target/debug/deps/bingo_surface_tui-32e6bd41d7d9a66d)

running 1134 tests
test acp::tests::a_call_that_said_nothing_has_no_result ... ok
test acp::tests::a_creation_removes_nothing_and_a_terminal_is_named ... ok
test acp::tests::a_diff_block_becomes_the_houses_own_diff ... ok
test acp::tests::a_failed_call_says_so_and_carries_what_it_said ... ok
test acp::tests::a_kind_this_build_does_not_know_is_a_tool ... ok
test acp::tests::a_marked_item_reads_back_as_the_call_it_was ... ok
test acp::tests::a_plain_thought_is_no_call_and_neither_is_an_unmarked_namespace ... ok
test acp::tests::a_thin_mark_is_still_a_call ... ok
test acp::tests::content_that_is_not_text_is_named_rather_than_dropped ... ok
test activity::tests::a_child_under_a_running_root_is_not_waiting_on_an_agent ... ok
test activity::tests::a_session_idle_over_running_agents_says_how_many_it_waits_for ... ok
test activity::tests::a_session_with_no_agent_at_work_says_nothing_at_all ... ok
test activity::tests::a_turn_on_the_viewed_session_takes_the_row_back_from_the_wait ... ok
test activity::tests::the_wait_carries_no_key_no_clock_and_no_token_count ... ok
test activity::tests::the_working_word_is_beamed_while_the_turn_runs_and_plain_when_still ... ok
test blocks::tests::a_blank_row_separates_the_blocks_and_never_opens_the_transcript ... ok
test blocks::tests::a_block_knows_which_lines_are_its_own ... ok
test blocks::tests::a_dropped_item_takes_its_block_with_it ... ok
test blocks::tests::a_failed_turn_keeps_its_line_at_the_foot_of_the_transcript ... ok
test blocks::tests::a_finished_item_is_drawn_once_and_scrolling_draws_nothing ... ok
test blocks::tests::a_flash_owes_frames_until_its_resting_form_is_drawn ... ok
test blocks::tests::a_picture_read_in_after_its_block_makes_it_draw_again ... ok
test blocks::tests::a_shell_that_ends_makes_its_block_draw_again ... ok
test blocks::tests::a_thought_a_newer_one_joined_gives_its_block_back ... ok
test blocks::tests::a_width_change_drops_every_block_once ... ok
test blocks::tests::a_window_is_an_exact_slice_of_the_whole_transcript ... ok
test blocks::tests::the_item_that_is_still_arriving_is_the_only_one_drawn_again ... ok
test blocks::tests::the_tail_is_the_last_rows_as_plain_text ... ok
test clipboard::tests::a_scratch_file_that_was_never_written_reads_as_nothing ... ok
test clipboard::tests::a_tool_that_does_not_exist_is_no_picture ... ok
test clock::tests::a_breath_rises_and_falls_across_its_period ... ok
test clock::tests::a_cycle_is_the_wall_clocks_own_turn_of_the_period ... ok
test clock::tests::a_pulse_alternates_on_the_boundary_of_its_period ... ok
test clock::tests::a_span_is_said_in_the_largest_unit_that_fits ... ok
test clock::tests::a_step_counts_the_frames_an_animation_is_drawn_in ... ok
test clock::tests::a_sweep_crosses_its_run_and_leaves_it_at_rest_behind_it ... ok
test clock::tests::a_sweep_reveals_its_run_from_the_near_end_and_leaves_none_of_it_behind ... ok
test clock::tests::a_swell_rises_to_the_middle_of_its_run_and_comes_back ... ok
test clock::tests::an_animation_runs_from_zero_to_one_and_stops_there ... ok
test clock::tests::how_long_ago_is_said_in_the_largest_unit_that_fits ... ok
test clock::tests::only_off_stills_the_surface ... ok
test clock::tests::the_two_easings_are_the_only_two_and_both_end_where_they_start_from ... ok
test commands::tests::a_catalogue_argument_completes_from_that_catalogue ... ok
test commands::tests::a_catalogue_that_has_not_arrived_yet_offers_nothing ... ok
test commands::tests::a_catalogue_value_is_ranked_and_not_substring_tested ... ok
test commands::tests::a_free_argument_offers_nothing ... ok
test commands::tests::a_half_typed_verb_is_still_a_verb_and_a_third_word_is_nobody_s ... ok
test commands::tests::a_line_that_is_not_a_command_has_no_dropdown ... ok
test commands::tests::a_second_word_is_not_completed_from_the_first_word_s_list ... ok
test commands::tests::a_subsequence_of_a_command_offers_it ... ok
test commands::tests::a_verb_and_a_space_offers_every_name_the_catalogue_holds ... ok
test commands::tests::a_verb_may_name_its_own_words_for_the_word_after_it ... ok
test commands::tests::a_word_list_completes_from_the_command_s_own_words ... ok
test commands::tests::an_alias_completes_the_argument_its_command_does ... ok
test commands::tests::an_alias_offers_the_command_once_under_the_name_that_matched ... ok
test commands::tests::catalogue_entries_that_are_not_specs_are_skipped ... ok
test commands::tests::the_four_local_commands_are_recognised_and_nothing_else_is ... ok
test commands::tests::the_word_after_a_verb_comes_from_the_catalogue_the_verb_named ... ok
test complete::tests::a_bare_mark_offers_the_names_then_what_paths_fit ... ok
test complete::tests::a_chosen_name_completes_the_line_the_way_a_path_does ... ok
test complete::tests::a_completion_replaces_the_mention_and_leaves_the_rest_alone ... ok
test complete::tests::a_format_only_the_decoder_reads_is_an_attachment_too ... ok
test complete::tests::a_mention_is_the_word_being_typed_and_a_finished_one_is_not ... ok
test complete::tests::a_session_with_no_names_offers_the_paths_alone ... ok
test complete::tests::a_url_is_an_attachment_whatever_it_ends_in ... ok
test complete::tests::car_completes_to_the_manifest ... ok
test complete::tests::names_enough_to_fill_the_rows_leave_no_room_for_a_path ... ok
test complete::tests::only_a_picture_becomes_an_attachment ... ok
test complete::tests::the_dropdown_offers_files_and_never_a_url ... ok
test complete::tests::the_names_come_first_and_the_paths_after_them ... ok
test complete::tests::the_rows_are_capped_and_the_bare_mark_offers_what_there_is ... ok
test complete::tests::the_walk_offers_the_sources_and_not_the_build ... ok
test composer::strip::tests::a_carried_picture_is_a_thumbnail_of_the_band ... ok
test composer::strip::tests::a_line_with_no_picture_and_a_terminal_with_none_have_no_strip ... ok
test composer::strip::tests::a_narrow_box_cuts_thumbnails_and_never_the_band ... ok
test composer::strip::tests::four_are_shown_and_the_rest_are_counted ... ok
test composer::strip::tests::the_strip_is_in_the_lines_own_order ... ok
test composer::tests::a_caret_at_the_right_edge_shows_on_the_next_row ... ok
test composer::tests::a_paste_lands_verbatim_at_the_caret ... ok
test composer::tests::a_picture_token_is_deleted_and_stepped_over_whole ... ok
test composer::tests::a_short_line_clamps_the_kept_column ... ok
test composer::tests::home_and_end_are_line_wise ... ok
test composer::tests::insert_and_backspace_move_by_grapheme ... ok
test composer::tests::layout_keeps_explicit_newlines ... ok
test composer::tests::layout_wraps_at_the_box_width_and_places_the_caret ... ok
test composer::tests::the_three_kill_chords_cut_word_line_start_and_line_end ... ok
test composer::tests::vertical_motion_reports_when_there_is_no_line_to_go_to ... ok
test composer::tests::word_motions_stop_at_whitespace ... ok
test fold::tests::a_click_goes_round_three_states_on_a_thought_and_two_on_a_result ... ok
test fold::tests::an_agents_own_call_peeks_the_way_a_result_does ... ok
test fold::tests::ctrl_o_climbs_to_the_sheet_and_never_back ... ok
test fold::tests::every_kind_starts_at_its_peek ... ok
test fold::tests::what_a_person_set_is_what_the_row_wears ... ok
test foot::tests::a_narrow_row_keeps_the_count_and_then_the_arrow ... ok
test foot::tests::the_label_says_how_far_the_foot_is_and_how_to_reach_it ... ok
test foot::tests::the_pill_sits_in_the_middle_of_the_row ... ok
test form::tests::a_digit_picks_the_row_it_names_and_fixes_it ... ok
test form::tests::a_mockup_taller_than_the_frame_says_how_much_it_kept_back ... ok
test form::tests::a_narrow_card_puts_the_framed_preview_above_the_answers_never_below ... ok
test form::tests::a_question_settled_on_nothing_is_a_cancel_in_its_place ... ok
test form::tests::a_row_wider_than_its_column_is_cut_and_says_so ... ok
test form::tests::a_screen_too_short_for_the_mockup_holds_the_shorter_height ... ok
test form::tests::a_set_ticked_and_written_under_answers_with_both ... ok
test form::tests::a_set_wears_brackets_and_the_last_row_of_a_question_is_the_way_out ... ok
test form::tests::enter_fixes_one_question_and_submit_sends_them_all_in_order ... ok
test form::tests::space_ticks_a_set_and_a_choice_has_no_boxes_to_tick ... ok
test form::tests::submit_with_a_question_still_open_walks_to_it_instead_of_sending ... ok
test form::tests::tab_walks_the_questions_and_stops_at_submit ... ok
test form::tests::the_arrows_walk_the_tabs_and_each_keeps_the_cursor_it_was_left_on ... ok
test form::tests::the_chat_row_cancels_the_whole_form ... ok
test form::tests::the_cursor_moving_moves_the_preview_with_it ... ok
test form::tests::the_frame_hugs_the_mockup_and_cuts_a_row_too_wide_for_its_room ... ok
test form::tests::the_key_line_is_the_cards_last_row_and_a_set_adds_its_own_key ... ok
test form::tests::the_preview_stands_in_a_frame_beside_the_option_under_the_cursor ... ok
test form::tests::the_rule_that_closes_the_band_is_on_one_row_for_every_tab ... ok
test form::tests::the_submit_tab_says_how_many_questions_are_still_open ... ok
test form::tests::the_tab_row_wears_a_box_per_question_an_arrow_each_end_and_submit ... ok
test form::tests::the_words_row_answers_in_the_persons_own_words ... ok
test form::tests::the_words_row_is_read_wherever_the_cursor_was_fixed ... ok
test frame::tests::a_strip_stands_on_the_box_without_changing_it ... ok
test frame::tests::every_region_stacks_without_overlapping ... ok
test frame::tests::the_activity_rows_give_way_before_the_transcript_does ... ok
test frame::tests::the_box_grows_with_the_draft_and_stops_at_ten_rows ... ok
test frame::tests::the_composer_and_the_status_line_survive_the_smallest_screens ... ok
test frame::tests::the_four_sizes_lay_out_the_same_frame ... ok
test frame::tests::the_rail_appears_at_a_hundred_and_twenty_columns_and_not_before ... ok
test frame::tests::the_strip_is_all_there_or_not_there ... ok
test graphics::band::tests::a_narrow_band_cuts_thumbnails_and_never_the_band ... ok
test graphics::band::tests::a_picture_is_three_rows_of_cells_in_the_bands_own_box ... ok
test graphics::band::tests::a_short_thumbnail_stands_on_the_bands_last_row ... ok
test graphics::band::tests::four_are_shown_and_the_rest_are_counted ... ok
test graphics::band::tests::four_thumbnails_and_a_count_fit_seventy_four_columns ... ok
test graphics::band::tests::nothing_is_no_band ... ok
test graphics::band::tests::the_band_keeps_the_order_it_was_given ... ok
test graphics::decoded::tests::a_picture_inside_its_cells_is_not_blown_up_to_fill_them ... ok
test graphics::decoded::tests::a_picture_is_fitted_to_its_cells_once_and_answered_from_the_memo_after ... ok
test graphics::decoded::tests::a_picture_is_measured_once_and_answered_from_the_memo_after ... ok
test graphics::decoded::tests::a_picture_no_decoder_reads_is_remembered_as_such ... ok
test graphics::decoded::tests::a_rectangle_is_not_yet_until_the_run_has_fitted_it ... ok
test graphics::decoded::tests::a_rectangle_of_a_picture_no_decoder_reads_is_never ... ok
test graphics::decoded::tests::an_answer_nobody_is_waiting_for_is_dropped ... ok
test graphics::decoded::tests::no_more_is_held_than_the_terminal_holds ... ok
test graphics::kitty::tests::a_cell_says_which_picture_it_is_a_cell_of ... ok
test graphics::kitty::tests::a_delete_takes_the_bytes_with_it ... ok
test graphics::kitty::tests::a_long_picture_is_chunked_at_four_thousand_and_ninety_six ... ok
test graphics::kitty::tests::a_placeholder_cell_is_one_column_wide ... ok
test graphics::kitty::tests::a_row_of_cells_says_where_each_of_them_is_and_whose_it_is ... ok
test graphics::kitty::tests::no_cell_is_drawn_that_could_not_say_where_it_is ... ok
test graphics::kitty::tests::one_short_picture_is_one_sequence ... ok
test graphics::kitty::tests::the_diacritics_are_the_protocols_own ... ok
test graphics::kitty::tests::under_tmux_each_chunk_is_its_own_envelope ... ok
test graphics::linked::tests::a_destination_is_a_path_unless_it_is_a_web_address ... ok
test graphics::linked::tests::a_destination_is_taken_once_and_keeps_the_answer_it_got ... ok
test graphics::linked::tests::a_failure_is_the_answer_for_the_rest_of_the_session ... ok
test graphics::linked::tests::a_frames_list_is_taken_once_and_answers_where_each_one_points ... ok
test graphics::linked::tests::a_picture_on_disk_is_read_through_the_seam_and_a_missing_one_says_so ... ok
test graphics::linked::tests::a_servers_answer_is_named_by_its_status ... ok
test graphics::linked::tests::a_transcript_of_links_stops_at_the_bound ... ok
test graphics::linked::tests::an_answer_nobody_asked_for_is_dropped ... ok
test graphics::linked::tests::every_failure_has_a_word_short_enough_to_stand_after_a_name ... ok
test graphics::picture::tests::a_draft_is_read_out_of_the_file_the_composer_is_holding ... ok
test graphics::picture::tests::a_picture_finds_itself_in_the_session_it_was_drawn_from ... ok
test graphics::picture::tests::a_picture_that_fits_takes_the_cells_it_covers ... ok
test graphics::picture::tests::a_picture_too_big_keeps_its_shape ... ok
test graphics::picture::tests::a_pictures_pixels_are_the_pixels_of_the_cells_it_covers ... ok
test graphics::picture::tests::an_id_fits_a_colour_and_is_never_the_protocols_none ... ok
test graphics::picture::tests::nothing_is_drawn_past_the_end_of_the_diacritics ... ok
test graphics::picture::tests::the_id_is_where_the_picture_is_and_nothing_else ... ok
test graphics::picture::tests::the_pictures_of_an_item_are_its_parts_in_order ... ok
test graphics::picture::tests::the_same_destination_is_one_picture_wherever_it_is_named ... ok
test graphics::picture::tests::the_three_kinds_of_picture_never_share_a_number ... ok
test graphics::placed::tests::a_pictures_rows_are_one_rectangle_of_the_screen ... ok
test graphics::placed::tests::a_run_that_moved_column_is_not_one_rectangle ... ok
test graphics::placed::tests::a_split_run_of_cells_is_one_rectangle ... ok
test graphics::placed::tests::nothing_is_placed_outside_the_region_it_was_drawn_in ... ok
test graphics::placed::tests::one_picture_drawn_twice_is_two_places ... ok
test graphics::placed::tests::two_pictures_on_one_row_are_two_rectangles ... ok
test graphics::placed::tests::words_place_no_picture ... ok
test graphics::placeholders::tests::a_terminal_past_its_floor_still_draws ... ok
test graphics::placeholders::tests::a_terminal_that_says_ok_and_draws_no_placeholder_is_not_on_the_list ... ok
test graphics::placeholders::tests::a_version_that_will_not_parse_is_below_every_floor ... ok
test graphics::placeholders::tests::the_four_draw_placeholders_from_the_version_that_learned_to ... ok
test graphics::probe::tests::a_cell_of_no_pixels_is_no_cell ... ok
test graphics::probe::tests::a_late_reply_joins_the_answer_and_takes_nothing_back ... ok
test graphics::probe::tests::a_name_is_read_whether_or_not_there_are_pictures_behind_it ... ok
test graphics::probe::tests::a_terminal_that_answers_only_da1_draws_no_pictures ... ok
test graphics::probe::tests::a_terminal_that_draws_pictures_says_ok_and_how_big_a_cell_is ... ok
test graphics::probe::tests::a_version_is_read_as_far_as_it_is_numbers ... ok
test graphics::probe::tests::an_answer_that_is_not_ok_for_this_query_is_a_no ... ok
test graphics::probe::tests::every_name_that_answers_is_kept_in_the_order_it_arrived ... ok
test graphics::probe::tests::passthrough_off_leaves_nothing_but_tmuxs_own_name ... ok
test graphics::probe::tests::the_first_cell_reply_is_the_one_taken ... ok
test graphics::probe::tests::the_four_queries_go_out_in_one_write ... ok
test graphics::probe::tests::the_name_is_read_from_either_shape_and_from_nothing_else ... ok
test graphics::probe::tests::the_read_ends_on_da1_and_on_nothing_else ... ok
test graphics::probe::tests::under_tmux_the_four_go_wrapped_behind_one_question_for_tmux ... ok
test graphics::probe::tests::what_arrives_after_the_da1_reply_is_not_the_answer ... ok
test graphics::stored::tests::a_first_picture_still_being_fitted_is_not_claimed_held ... ok
test graphics::stored::tests::a_picture_a_frame_did_not_place_is_kept_for_its_return ... ok
test graphics::stored::tests::a_picture_is_transmitted_once_however_many_frames_draw_it ... ok
test graphics::stored::tests::a_picture_still_being_fitted_is_neither_sent_nor_taken_away ... ok
test graphics::stored::tests::a_picture_that_grew_is_sent_again_at_the_size_it_grew_to ... ok
test graphics::stored::tests::a_picture_without_pixels_is_never_claimed_to_be_held ... ok
test graphics::stored::tests::no_more_than_the_cap_is_ever_held ... ok
test graphics::stored::tests::one_picture_placed_twice_is_sent_once ... ok
test graphics::stored::tests::the_way_out_lets_everything_go ... ok
test graphics::stored::tests::through_tmux_every_sequence_goes_out_wrapped ... ok
test graphics::stored::tests::what_is_held_is_deleted_when_it_falls_off_the_end ... ok
test graphics::tests::a_ghostty_behind_a_new_enough_tmux_draws_pictures ... ok
test graphics::tests::a_late_answer_with_no_cell_still_takes_the_wrong_word_back ... ok
test graphics::tests::a_passthrough_tmux_says_is_off_is_told_about_by_name ... ok
test graphics::tests::a_run_that_draws_nothing_has_no_transport_to_send_it_on ... ok
test graphics::tests::a_terminal_that_names_itself_to_nobody_gets_the_chip ... ok
test graphics::tests::a_terminal_that_says_ok_and_draws_no_placeholder_gets_the_chip ... ok
test graphics::tests::a_tmux_below_the_floor_draws_no_pictures ... ok
test graphics::tests::a_wezterm_behind_tmux_draws_no_pictures ... ok
test graphics::tests::an_answer_completed_late_turns_the_pictures_on_once ... ok
test graphics::tests::only_a_tmux_nobody_answered_behind_is_told_about ... ok
test graphics::tests::only_off_switches_the_pictures_off ... ok
test graphics::tests::pictures_are_drawn_only_when_every_part_of_the_answer_came_back ... ok
test graphics::tests::what_the_probe_settles_is_the_drawing_and_the_word_about_it ... ok
test graphics::tmux::tests::a_chunk_is_wrapped_with_its_escapes_doubled ... ok
test graphics::tmux::tests::no_other_name_is_tmux ... ok
test graphics::tmux::tests::only_tmux_is_a_multiplexer_this_reaches_through ... ok
test graphics::tmux::tests::the_passthrough_is_read_from_what_tmux_says_and_nothing_else ... ok
test graphics::tmux::tests::tmux_carries_pictures_from_the_version_that_draws_them_right ... ok
test guide::tests::the_page_is_one_line_of_description_and_a_body_worth_reading ... ok
test guide::tests::the_page_names_every_command_this_surface_answers_itself ... ok
test guide::tests::the_page_names_every_key_this_surface_binds ... ok
test guide::tests::the_page_names_every_settings_key_this_surface_claims ... ok
test guide::tests::the_page_says_what_becomes_of_a_picture ... ok
test highlight::tests::a_block_kept_warm_is_drawn_again_when_the_look_changes ... ok
test highlight::tests::a_block_that_changed_behind_its_last_row_is_parsed_again ... ok
test highlight::tests::a_comment_is_dim_and_a_keyword_is_the_cool_colour ... ok
test highlight::tests::a_half_written_row_is_drawn_and_never_kept ... ok
test highlight::tests::a_language_nothing_claims_still_reads_as_code ... ok
test highlight::tests::a_line_of_rust_wears_the_three_inks_and_nothing_else ... ok
test highlight::tests::json_keeps_its_text_and_colours_only_its_keywords ... ok
test highlight::tests::one_delta_parses_only_the_added_line ... ok
test highlight::tests::only_a_handful_of_blocks_stay_warm ... ok
test highlight::tests::resuming_draws_exactly_what_a_fresh_parse_would ... ok
test history::tests::a_missing_file_is_an_empty_history ... ok
test history::tests::a_repeated_prompt_is_not_stored_twice ... ok
test history::tests::an_empty_history_never_moves ... ok
test history::tests::the_file_round_trips_through_load_and_append ... ok
test history::tests::walking_back_and_forth_returns_to_the_draft ... ok
test input::tests::a_chip_is_words_and_a_click_on_it_opens_nothing ... ok
test input::tests::a_chord_that_opens_something_else_puts_the_list_away ... ok
test input::tests::a_click_beside_a_picture_is_still_the_blocks ... ok
test input::tests::a_click_cycles_a_finished_thought_through_its_three_states ... ok
test input::tests::a_click_in_the_transcript_focuses_the_block_it_landed_on ... ok
test input::tests::a_click_inside_a_picture_opens_it ... ok
test input::tests::a_click_on_a_card_row_answers_it ... ok
test input::tests::a_click_on_a_child_row_steps_into_it ... ok
test input::tests::a_click_on_a_strips_thumbnail_opens_it ... ok
test input::tests::a_click_on_the_band_above_a_persons_line_opens_it ... ok
test input::tests::a_click_on_the_foot_row_follows_the_tail_again ... ok
test input::tests::a_click_on_the_rail_focuses_the_card_it_landed_on ... ok
test input::tests::a_click_opens_a_fold_and_a_second_click_folds_it ... ok
test input::tests::a_click_opens_a_folded_notice_too ... ok
test input::tests::a_click_opens_the_whole_run_and_ctrl_o_takes_it_to_the_sheet ... ok
test input::tests::a_digit_with_no_card_focused_is_typed ... ok
test input::tests::a_drag_back_inside_the_transcript_stops_the_walk ... ok
test input::tests::a_drag_below_the_transcript_scrolls_by_the_overshoot ... ok
test input::tests::a_drag_copies_the_cells_it_went_over ... ok
test input::tests::a_drag_takes_the_far_end_of_the_run_with_it ... ok
test input::tests::a_failed_turn_does_not_change_what_a_key_does ... ok
test input::tests::a_form_is_answered_once_for_all_of_its_questions ... ok
test input::tests::a_free_text_question_sends_the_words_as_text ... ok
test input::tests::a_half_typed_line_keeps_the_arrows_for_itself ... ok
test input::tests::a_hand_held_at_the_top_walks_the_view_back_up ... ok
test input::tests::a_hand_held_past_the_foot_takes_a_line_every_pace ... ok
test input::tests::a_key_before_the_guard_sends_nothing ... ok
test input::tests::a_key_between_the_two_escapes_is_what_says_they_were_not_one_gesture ... ok
test input::tests::a_key_on_the_focused_card_fires_the_action_it_names ... ok
test input::tests::a_key_the_focused_card_does_not_offer_is_a_letter_like_any_other ... ok
test input::tests::a_letter_ends_a_run_and_is_typed ... ok
test input::tests::a_line_with_tokens_goes_as_typed_and_its_pictures_are_read_later ... ok
test input::tests::a_mentioned_picture_is_words_the_loop_reads ... ok
test input::tests::a_multiple_choice_question_toggles_then_confirms ... ok
test input::tests::a_notice_frame_is_not_a_key_concern ... ok
test input::tests::a_paste_lands_verbatim ... ok
test input::tests::a_press_and_a_release_on_one_cell_is_still_a_click ... ok
test input::tests::a_prompt_a_child_raised_is_answered_from_the_root_view ... ok
test input::tests::a_question_that_offers_cancel_takes_esc_as_one ... ok
test input::tests::a_release_event_is_not_a_press ... ok
test input::tests::a_sheet_over_the_frame_takes_the_click_with_it ... ok
test input::tests::a_single_choice_question_answers_on_the_first_key ... ok
test input::tests::a_trailing_backslash_turns_enter_into_a_newline ... ok
test input::tests::a_typed_query_narrows_the_list_and_the_view_follows_it ... ok
test input::tests::a_view_block_is_dismissed_by_the_next_key ... ok
test input::tests::a_walk_past_the_foot_stops_at_the_last_line ... ok
test input::tests::a_word_chord_edits_the_query_and_leaves_the_list_up ... ok
test input::tests::an_at_sign_offers_the_paths_under_the_session_and_enter_takes_one ... ok
test input::tests::an_empty_refusal_carries_no_feedback ... ok
test input::tests::an_empty_thought_opens_nothing ... ok
test input::tests::an_esc_that_closes_something_stops_no_turn ... ok
test input::tests::an_open_dialog_keeps_shift_tab ... ok
test input::tests::any_key_skips_the_opening_and_is_spent_on_nothing_else ... ok
test input::tests::backspace_gives_the_rows_back_letter_by_letter ... ok
test input::tests::clear_opens_a_new_session_in_the_same_directory ... ok
test input::tests::clear_starts_beside_the_root_even_from_a_child_view ... ok
test input::tests::ctrl_b_fires_the_plugins_command_naming_the_call_that_is_running ... ok
test input::tests::ctrl_b_is_documented_where_every_key_is ... ok
test input::tests::ctrl_b_says_so_when_no_shell_command_is_running ... ok
test input::tests::ctrl_c_interrupts_then_clears_then_arms_then_exits ... ok
test input::tests::ctrl_d_leaves_only_on_an_empty_composer ... ok
test input::tests::ctrl_g_lists_the_tree_and_enter_switches_the_view ... ok
test input::tests::ctrl_g_on_a_lone_root_still_asks_what_the_store_holds ... ok
test input::tests::ctrl_g_toggles_and_esc_closes_the_switcher ... ok
test input::tests::ctrl_o_climbs_a_thought_from_its_hold_through_the_whole_to_the_sheet ... ok
test input::tests::ctrl_o_walks_past_a_run_whose_last_thought_is_still_being_had ... ok
test input::tests::ctrl_p_toggles_the_plugin_state_panel_and_esc_closes_it ... ok
test input::tests::ctrl_t_hides_the_task_list_and_again_shows_it ... ok
test input::tests::ctrl_v_asks_for_the_clipboard ... ok
test input::tests::down_keeps_its_old_meaning_where_there_is_nowhere_to_walk_to ... ok
test input::tests::down_on_an_empty_line_opens_the_list_ctrl_g_opens ... ok
test input::tests::down_opens_the_list_and_then_walks_what_the_query_left ... ok
test input::tests::enter_in_the_panel_sheet_pins_a_panel_and_again_takes_it_back ... ok
test input::tests::enter_on_a_focused_thought_opens_the_sheet ... ok
test input::tests::enter_on_a_stored_row_steps_into_the_session_it_names ... ok
test input::tests::enter_on_an_empty_composer_does_nothing ... ok
test input::tests::esc_closes_the_innermost_thing_then_interrupts ... ok
test input::tests::esc_gives_back_where_the_walk_started_and_enter_keeps_where_it_ended ... ok
test input::tests::esc_leaves_the_whole_form ... ok
test input::tests::esc_lets_a_run_go ... ok
test input::tests::esc_lets_go_of_a_hand_held_past_the_edge ... ok
test input::tests::esc_on_the_list_does_not_reach_the_turn ... ok
test input::tests::esc_takes_the_query_back_before_it_closes_the_list ... ok
test input::tests::esc_twice_on_an_empty_composer_lists_the_turns_and_enter_rewinds ... ok
test input::tests::every_newline_chord_inserts_one ... ok
test input::tests::exit_and_quit_leave_and_resume_opens_or_asks ... ok
test input::tests::help_is_the_surfaces_own_and_reaches_nobody ... ok
test input::tests::home_and_end_walk_the_transcript_while_nothing_is_typed ... ok
test input::tests::nothing_opens_where_the_session_cannot_rewind ... ok
test input::tests::one_layer_at_a_time_replaces_the_last ... ok
test input::tests::prose_and_the_kernels_own_commands_are_submitted_verbatim ... ok
test input::tests::refusing_collects_the_words_that_go_with_it ... ok
test input::tests::releasing_a_drag_copies_what_it_reached_and_says_how_much ... ok
test input::tests::shift_tab_asks_for_the_next_mode_and_wraps ... ok
test input::tests::shift_tab_leaves_the_draft_where_it_was ... ok
test input::tests::shift_tab_says_so_when_the_mode_is_not_one_it_knows ... ok
test input::tests::stopping_a_turn_marks_the_turn_it_stopped ... ok
test input::tests::submitting_clears_the_scroll_so_the_answer_is_visible ... ok
test input::tests::tab_completes_a_rooms_name_with_its_sigil ... ok
test input::tests::tab_completes_and_enter_runs_what_is_already_whole ... ok
test input::tests::tab_completes_before_it_queues_and_queues_only_while_a_turn_runs ... ok
test input::tests::tab_completes_nothing_where_the_query_left_no_row ... ok
test input::tests::tab_completes_the_name_the_cursor_is_on ... ok
test input::tests::tab_is_left_alone_when_the_rail_has_no_cards_to_walk ... ok
test input::tests::tab_mid_turn_sends_a_line_that_waits_and_enter_sends_one_that_steers ... ok
test input::tests::tab_walks_the_rail_cards_and_comes_back_round_to_none ... ok
test input::tests::the_arrows_move_inside_a_multi_line_draft_first ... ok
test input::tests::the_arrows_walk_the_prompt_history_at_the_edges ... ok
test input::tests::the_column_stops_at_its_ends ... ok
test input::tests::the_dialog_state_resets_when_the_next_interaction_opens ... ok
test input::tests::the_key_after_the_skip_is_typed ... ok
test input::tests::the_kill_chords_cut_the_buffer ... ok
test input::tests::the_line_being_written_is_set_aside_and_given_back ... ok
test input::tests::the_list_opens_with_nothing_typed_into_it ... ok
test input::tests::the_page_keys_scroll_a_screenful_and_come_back_to_the_tail ... ok
test input::tests::the_picker_opens_the_session_it_lands_on ... ok
test input::tests::the_query_offers_no_command_of_its_own ... ok
test input::tests::the_question_mark_only_opens_the_panel_on_an_empty_composer ... ok
test input::tests::the_session_option_answers_with_the_scope_the_kernel_named ... ok
test input::tests::the_switcher_opens_on_the_child_that_is_already_in_view ... ok
test input::tests::the_walk_goes_on_past_a_recalled_command_and_enter_runs_it ... ok
test input::tests::the_wheel_scrolls_the_transcript_and_the_foot_takes_it_back ... ok
test input::tests::the_wheel_scrolls_under_a_run_and_keeps_it ... ok
test input::tests::up_on_an_empty_box_asks_for_the_newest_line_of_ours_that_is_queued ... ok
test input::tests::up_walks_past_another_surfaces_queued_line_into_the_history ... ok
test input::tests::up_with_an_empty_queue_recalls_the_prompt_history ... ok
test input::tests::v_is_a_letter_while_the_transcript_is_at_its_foot ... ok
test input::tests::v_starts_a_run_the_arrows_extend_and_y_copies_it ... ok
test input::tests::walking_it_off_the_top_brings_the_view_back_up ... ok
test input::tests::walking_the_far_end_across_a_line_moves_nothing ... ok
test input::tests::walking_the_far_end_off_the_screen_brings_the_view_after_it ... ok
test input::tests::walking_the_list_switches_the_view_as_the_cursor_moves ... ok
test input::tests::what_is_closing_stays_on_screen_until_it_has_gone ... ok
test input::tests::y_and_n_name_the_answers_wherever_they_sit ... ok
test keys::tests::a_hundred_columns_is_where_the_second_column_starts ... ok
test keys::tests::a_narrow_terminal_falls_back_to_one_column ... ok
test keys::tests::a_wide_terminal_pairs_the_rows ... ok
test keys::tests::ctrl_c_stops_a_turn_clears_a_line_and_then_leaves ... ok
test keys::tests::esc_closes_the_innermost_thing_that_is_open ... ok
test keys::tests::every_chord_is_listed_once ... ok
test keys::tests::no_row_overflows_the_width_it_was_built_for ... ok
test keys::tests::tab_completes_then_queues_then_walks_the_cards ... ok
test keys::tests::the_help_prints_the_escape_stack ... ok
test keys::tests::the_help_prints_the_tab_stack ... ok
test late::tests::a_late_graphics_answer_is_heard_whole_and_nothing_of_it_is_typed ... ok
test late::tests::a_person_who_types_the_chord_by_hand_keeps_every_key ... ok
test late::tests::a_release_is_never_part_of_a_reply ... ok
test late::tests::a_reply_that_never_ends_is_bounded ... ok
test late::tests::a_reply_with_no_body_is_not_a_reply ... ok
test late::tests::a_sequence_that_stops_looking_like_a_reply_is_given_back ... ok
test late::tests::a_terminator_split_by_a_read_boundary_is_still_a_terminator ... ok
test late::tests::an_event_that_is_no_key_at_all_gives_back_what_was_held ... ok
test late::tests::an_osc_that_ends_in_bel_is_heard_and_its_ctrl_g_never_fires ... ok
test late::tests::every_shape_a_probe_is_answered_in_is_heard ... ok
test late::tests::one_reply_leaves_nothing_behind_for_the_next ... ok
test layers::tests::a_card_comes_down_over_three_frames ... ok
test layers::tests::a_card_hangs_under_the_row_that_asked ... ok
test layers::tests::a_card_with_no_room_under_that_row_sits_at_the_foot ... ok
test layers::tests::a_pictures_cells_are_not_dimmed ... ok
test layers::tests::a_sheet_takes_four_frames_to_fill_its_region ... ok
test layers::tests::each_frame_shows_its_share_of_the_rows ... ok
test layers::tests::esc_runs_the_same_frames_backwards ... ok
test layout::tests::layout_answers_picture_and_rail_clicks_without_painting ... ok
test layout::tests::layout_countdown_changes_at_the_deadline_it_reports ... ok
test layout::tests::layout_finds_a_rail_card_without_a_terminal ... ok
test layout::tests::layout_finds_the_foot_in_the_current_transcript ... ok
test layout::tests::layout_is_pure_and_idle_has_no_deadline ... ok
test layout::tests::layout_layer_send_and_drag_report_their_own_deadlines ... ok
test layout::tests::layout_notice_sleeps_through_its_hold_and_expires ... ok
test layout::tests::layout_renders_each_band_once_and_reuses_unchanged_blocks ... ok
test layout::tests::layout_running_turn_schedules_each_motion_sample ... ok
test layout::tests::layout_schedules_a_scroll_sample ... ok
test layout::tests::layout_schedules_exit_at_its_deadline ... ok
test layout::tests::layout_unbounded_rail_schedules_its_next_step ... ok
test markdown::spaced::tests::a_bang_that_opens_no_picture_is_kept ... ok
test markdown::spaced::tests::a_spaced_path_is_bracketed ... ok
test markdown::spaced::tests::a_title_stays_outside_the_brackets ... ok
test markdown::spaced::tests::an_unspaced_or_bracketed_path_is_left_alone ... ok
test markdown::spaced::tests::code_is_not_mended ... ok
test markdown::spaced::tests::two_pictures_on_one_line_are_both_mended ... ok
test markdown::tests::a_bare_path_with_spaces_is_still_a_picture ... ok
test markdown::tests::a_bulleted_picture_stands_on_its_bullet ... ok
test markdown::tests::a_cell_a_row_has_not_is_marked_and_emphasis_in_one_is_plain_text ... ok
test markdown::tests::a_document_with_no_picture_renders_exactly_as_it_did ... ok
test markdown::tests::a_fence_is_indented_and_highlighted_line_by_line ... ok
test markdown::tests::a_fence_that_names_a_diff_is_a_diff ... ok
test markdown::tests::a_heading_is_bold_and_stands_alone ... ok
test markdown::tests::a_link_shows_its_text_then_its_url ... ok
test markdown::tests::a_picture_with_no_name_wears_its_destination ... ok
test markdown::tests::a_pictures_column_is_the_column_of_the_words_it_was_written_among ... ok
test markdown::tests::a_quote_gets_a_bar_in_the_margin ... ok
test markdown::tests::a_reference_and_a_bracketed_destination_resolve_to_the_path ... ok
test markdown::tests::a_rule_spans_the_width ... ok
test markdown::tests::a_table_is_ruled_and_its_numbers_hug_the_right_edge ... ok
test markdown::tests::a_table_stands_apart_from_the_prose_around_it ... ok
test markdown::tests::a_table_wider_than_the_measure_folds_to_it ... ok
test markdown::tests::a_tables_header_is_bold_and_its_rule_is_dim ... ok
test markdown::tests::an_image_alone_is_its_own_line_and_nothing_else ... ok
test markdown::tests::an_image_in_a_paragraph_takes_the_line_after_it ... ok
test markdown::tests::an_inline_code_span_keeps_its_backticks_and_spends_no_colour ... ok
test markdown::tests::bullets_and_numbers_open_their_items ... ok
test markdown::tests::emphasis_and_strong_carry_their_modifiers ... ok
test markdown::tests::half_written_text_renders_as_what_it_is ... ok
test markdown::tests::two_pictures_in_one_paragraph_keep_their_order ... ok
test matching::tests::a_subsequence_of_the_name_finds_it ... ok
test matching::tests::a_typo_matches_nothing ... ok
test matching::tests::a_word_in_the_middle_finds_the_whole_id ... ok
test matching::tests::an_empty_query_is_the_whole_list_in_its_own_order ... ok
test matching::tests::an_upper_case_letter_in_the_query_is_meant ... ok
test matching::tests::the_key_says_what_is_matched ... ok
test matching::tests::the_tighter_match_ranks_first ... ok
test matching::tests::ties_keep_the_order_the_list_gave_them ... ok
test mentions::tests::a_room_offers_all_then_its_seats_without_the_holder ... ok
test mentions::tests::a_room_that_seats_a_member_named_all_offers_the_member_and_no_word ... ok
test mentions::tests::a_room_with_no_roster_offers_no_name ... ok
test mentions::tests::a_rostered_name_nobody_holds_is_offered_all_the_same ... ok
test mentions::tests::a_session_offers_the_agents_under_it_and_not_the_room_beside_them ... ok
test mentions::tests::only_the_children_of_the_session_in_view_are_offered ... ok
test motion::a_card_reveals_top_down_and_a_sheet_slides_up ... ok
test motion::a_cards_rows_are_dim_until_the_guard_lifts_and_plain_the_moment_it_does ... ok
test motion::a_comet_tail_cools_from_the_glow_to_the_text_behind_it ... ok
test motion::a_finished_turn_takes_the_whole_of_the_presence_with_it ... ok
test motion::a_live_bullet_pulses_between_presence_and_its_glow ... ok
test motion::a_notice_arrives_out_of_dim_and_leaves_into_it ... ok
test motion::a_refused_line_is_named_after_the_reason_that_refused_it ... ok
test motion::a_sent_line_runs_one_light_along_the_boxs_border ... ok
test motion::a_still_notice_is_said_at_once_and_still_leaves ... ok
test motion::a_still_surface_sends_a_line_with_no_light_at_all ... ok
test motion::a_turn_asked_to_stop_says_so_and_keeps_its_sparkle_and_its_clock ... ok
test motion::a_turn_that_answers_at_once_never_flashes_a_row ... ok
test motion::a_waiting_childs_row_and_its_switcher_line_pulse_with_it ... ok
test motion::an_idle_frame_is_the_same_frame_a_second_later ... ok
test motion::an_unanswered_cards_border_is_on_that_beat_too ... ok
test motion::esc_runs_a_sheet_back_down_the_way_it_came ... ok
test motion::every_verb_is_one_of_bingos_own ... ok
test motion::landing::a_completion_sweeps_its_name_for_six_frames_and_rests_on_the_seventh ... ok
test motion::landing::a_failure_flares_and_cools_into_the_words_behind_it ... ok
test motion::landing::a_new_block_takes_its_own_room_and_walks_nowhere_after_it ... ok
test motion::motion_off_holds_every_cue_at_its_resting_frame ... ok
test motion::motion_off_puts_a_layer_up_whole_and_takes_it_away_at_once ... ok
test motion::nothing_of_the_presence_is_on_screen_while_no_turn_runs ... ok
test motion::stopping_outruns_the_activity_row_s_own_delay ... ok
test motion::the_activity_row_says_a_verb_a_clock_and_what_the_turn_has_said ... ok
test motion::the_breath_quickens_while_words_arrive_and_slows_while_a_tool_holds_the_turn ... ok
test motion::the_context_count_is_always_there_and_warms_across_the_last_fifth ... ok
test motion::the_input_box_glows_on_the_same_breath_and_is_dim_when_idle ... ok
test motion::the_next_turn_is_not_the_one_that_was_stopped ... ok
test motion::the_presence_mark_breathes_between_two_thirds_and_all_of_itself ... ok
test motion::the_rhythms_are_the_ones_the_design_names ... ok
test motion::the_sparkle_is_drawn_on_the_period_the_state_asks_for ... ok
test motion::the_sparkle_walks_its_four_glyphs_at_a_hundred_and_fifty_milliseconds ... ok
test motion::the_tail_is_style_and_never_text ... ok
test motion::the_transcript_crossfades_through_dim_into_the_session_stepped_into ... ok
test motion::the_wait_breathes_at_the_pace_a_tool_holds_a_turn_and_leaves_the_box_dim ... ok
test motion::what_wants_a_person_alternates_on_the_second ... ok
test opening::beat::tests::no_beat_runs_past_the_end_of_the_piece ... ok
test opening::beat::tests::the_beats_begin_and_end_where_the_storyboard_says ... ok
test opening::beat::tests::the_marks_ignition_is_one_whole_turn_of_the_sparkle ... ok
test opening::beat::tests::the_piece_is_two_and_four_tenths_of_a_second_long ... ok
test opening::beat::tests::the_rows_arrive_in_order_and_the_last_lands_with_the_beat ... ok
test opening::cells::tests::a_cell_with_nothing_in_it_says_so ... ok
test opening::cells::tests::a_row_comes_apart_into_cells_and_back_into_the_runs_it_was ... ok
test opening::cells::tests::a_wide_glyph_takes_the_two_columns_the_terminal_gives_it ... ok
test opening::frame::tests::a_box_with_no_room_in_it_at_all_still_draws ... ok
test opening::frame::tests::a_newer_release_arrives_last_and_the_piece_is_no_longer_for_it ... ok
test opening::frame::tests::a_session_with_no_welcome_box_has_nothing_to_play ... ok
test opening::frame::tests::every_frame_is_the_boxs_own_shape_at_every_instant ... ok
test opening::frame::tests::the_border_arrives_before_anything_is_said_in_the_box ... ok
test opening::frame::tests::the_border_breathes_once_and_nothing_else_moves ... ok
test opening::frame::tests::the_frame_before_the_last_one_is_already_the_box ... ok
test opening::frame::tests::the_last_frame_is_the_welcome_box_and_nothing_else ... ok
test opening::frame::tests::the_mark_ignites_where_the_light_came_home ... ok
test opening::frame::tests::the_rows_arrive_in_the_order_they_are_read ... ok
test opening::frame::tests::the_same_second_draws_the_same_frame ... ok
test opening::lap::tests::the_light_is_one_point_with_one_tail_behind_it ... ok
test opening::lap::tests::the_light_leaves_the_corner_runs_the_perimeter_and_comes_home ... ok
test opening::lap::tests::the_perimeter_is_one_path_with_no_cell_on_it_twice ... ok
test opening::storyboard::play ... ignored, plays the opening in this terminal
test opening::storyboard::preview ... ignored, writes preview files for a person to look at
test opening::storyboard::the_twelve_frames_of_the_storyboard ... ok
test opening::tests::the_piece_is_over_when_its_seconds_have_run_out ... ok
test pager::tests::a_hit_below_the_fold_is_brought_onto_the_screen ... ok
test pager::tests::a_result_opens_with_every_line_it_folded_away ... ok
test pager::tests::a_row_that_already_shows_everything_opens_nothing ... ok
test pager::tests::a_run_of_thoughts_opens_as_the_whole_run ... ok
test pager::tests::a_table_too_wide_for_the_measure_has_more_room_in_the_sheet ... ok
test pager::tests::a_thought_opens_as_what_was_thought ... ok
test pager::tests::an_agents_call_that_said_nothing_opens_nothing ... ok
test pager::tests::an_agents_own_call_opens_on_what_came_back ... ok
test pager::tests::the_title_row_carries_the_keys_until_a_query_takes_it ... ok
test pager::tests::the_window_never_parks_past_either_end ... ok
test panel::tests::a_list_of_records_is_a_table_over_the_union_of_their_keys ... ok
test panel::tests::a_nested_value_is_its_compact_json ... ok
test panel::tests::a_payload_that_parses_as_a_view_is_drawn_as_one ... ok
test panel::tests::a_payload_whose_kind_this_surface_never_learned_is_its_fold ... ok
test panel::tests::a_pinned_row_says_so_and_only_in_the_session_it_was_pinned_in ... ok
test panel::tests::a_record_is_its_fields_one_to_a_line ... ok
test panel::tests::a_session_no_plugin_has_written_to_says_so ... ok
test panel::tests::anything_else_is_its_text ... ok
test panel::tests::every_plugin_and_kind_is_a_row_of_its_own ... ok
test panel::tests::internal_extension_namespaces_never_become_panels_or_pinned_cards ... ok
test paths::tests::a_long_path_keeps_its_ends_and_loses_its_middle ... ok
test paths::tests::a_path_is_named_from_the_nearest_root_a_person_knows ... ok
test paths::tests::a_path_of_a_hundred_and_twenty_cells_fits_one_row_at_eighty_columns ... ok
test paths::tests::a_wide_glyph_counts_two_cells_when_a_name_is_cut ... ok
test paths::tests::one_unbreakable_name_is_cut_through_the_middle ... ok
test paths::tests::only_the_path_shaped_words_of_a_summary_are_touched ... ok
test paths::tests::the_tilde_a_path_is_shown_with_reads_back_as_home ... ok
test pictures::tests::a_deleted_token_drops_its_picture_and_order_follows_the_line ... ok
test pictures::tests::a_paste_is_held_under_the_token_the_line_gets ... ok
test pictures::tests::a_token_typed_by_hand_is_words ... ok
test pictures::tests::a_tokens_edges_are_found_from_either_end ... ok
test pictures::tests::a_withdrawn_line_gets_its_pictures_back_under_the_tokens_it_names ... ok
test pictures::tests::the_next_token_is_one_past_the_highest ... ok
test pictures::tests::tokens_are_read_in_order_and_words_are_not_tokens ... ok
test pictures::tests::what_is_shown_carries_its_token_and_is_read_back_by_it ... ok
test plugin_tests::the_manifest_says_what_it_provides_and_the_two_keys_it_claims ... ok
test plugin_tests::the_plugin_registers_the_surface_and_its_page ... ok
test preview::tests::a_command_preview_shows_its_directory_and_the_line ... ok
test preview::tests::a_file_header_is_never_read_as_a_removal ... ok
test preview::tests::a_long_diff_is_bounded_until_it_is_expanded ... ok
test preview::tests::a_row_nothing_replaced_is_left_plain ... ok
test preview::tests::diff_rows_take_their_colour_from_the_first_column ... ok
test preview::tests::only_the_words_that_moved_are_emphasised_in_a_replaced_pair ... ok
test preview::tests::the_colour_still_comes_from_the_column_and_not_from_the_words ... ok
test rail::tests::a_panel_lends_the_card_its_title_and_anything_else_is_titled_by_kind ... ok
test rail::tests::a_pin_belongs_to_the_session_it_was_made_in ... ok
test rail::tests::a_signal_is_a_card_and_an_extension_is_one_only_once_it_is_pinned ... ok
test rail::tests::a_signal_with_slots_is_drawn_only_in_its_places ... ok
test rail::tests::every_row_of_the_rail_is_exactly_as_wide_as_the_rail ... ok
test rail::tests::one_plugin_takes_eight_rows_and_folds_the_rest ... ok
test rail::tests::the_focused_card_is_the_one_the_cursor_marks ... ok
test rail::tests::two_plugins_each_get_their_own_eight_rows ... ok
test rewind::tests::a_chosen_row_runs_the_command_on_its_own_turn ... ok
test rewind::tests::a_shell_line_between_turns_opens_no_row_of_its_own ... ok
test rewind::tests::a_shell_line_is_not_the_prompt_a_turn_answered ... ok
test rewind::tests::a_turn_a_command_opened_is_named_by_the_line_that_was_typed ... ok
test rewind::tests::a_turn_nobody_opened_by_typing_is_named_by_its_own_id ... ok
test rewind::tests::nothing_is_offered_that_could_not_be_done ... ok
test rewind::tests::the_card_marks_the_row_the_keyboard_is_on ... ok
test rewind::tests::the_turns_are_the_transcripts_own_boundaries_newest_first ... ok
test roster::tests::a_label_outside_the_window_is_not_drawn ... ok
test roster::tests::a_list_longer_than_its_room_keeps_the_row_the_cursor_is_on ... ok
test roster::tests::a_live_seat_wears_the_sigil_and_keeps_its_own_colour ... ok
test roster::tests::a_member_behind_the_room_says_how_much_it_has_not_read ... ok
test roster::tests::a_narrowed_list_keeps_every_row_of_its_own_room ... ok
test roster::tests::a_query_narrows_the_column_to_what_it_matches ... ok
test roster::tests::a_query_nothing_matches_draws_no_list ... ok
test roster::tests::a_room_and_the_seats_in_it_answer_to_the_rooms_name ... ok
test roster::tests::a_room_says_how_many_seats_it_has_and_what_it_is_owed ... ok
test roster::tests::a_row_that_wants_a_person_says_so_in_words ... ok
test roster::tests::a_seat_on_the_default_ear_says_nothing_about_what_it_hears ... ok
test roster::tests::a_session_that_has_not_run_says_what_it_was_asked ... ok
test roster::tests::a_stored_row_says_that_it_is_stored_and_nothing_more ... ok
test roster::tests::every_drawn_row_says_which_row_of_the_list_it_is ... ok
test roster::tests::no_width_makes_a_row_wider_than_the_list ... ok
test roster::tests::one_kind_of_session_alone_wears_no_label ... ok
test roster::tests::the_agents_come_first_and_the_rooms_under_their_own_label ... ok
test roster::tests::the_column_is_walked_to_its_ends_and_stops_there ... ok
test roster::tests::the_cursor_opens_on_the_row_of_the_session_in_view ... ok
test roster::tests::the_walk_steps_over_the_label_between_the_runs ... ok
test run::notices::tests::a_run_with_nothing_to_report_opens_in_silence ... ok
test run::notices::tests::what_the_probe_found_is_said_once_when_the_run_opens ... ok
test run::opening::tests::a_fresh_session_on_a_terminal_with_room_opens_with_the_piece ... ok
test run::opening::tests::a_session_that_already_happened_is_not_entered ... ok
test run::opening::tests::a_terminal_with_eight_colours_or_only_ascii_never_sees_it ... ok
test run::opening::tests::the_box_comes_back_when_the_piece_has_run_out ... ok
test run::opening::tests::the_frame_of_the_piece_is_what_the_box_draws ... ok
test run::submit::tests::every_named_picture_is_read_in_order_and_one_missing_stops_the_line ... ok
test run::submit::tests::the_lines_pictures_are_its_tokens_files_and_then_its_words ... ok
test run::tests::a_carried_picture_is_sent_small_and_kept_when_its_token_goes ... ok
test run::tests::a_child_closed_by_a_shutdown_keeps_what_it_said_for_its_reopening ... ok
test run::tests::a_child_that_is_deleted_leaves_the_tree_and_the_run_goes_on ... ok
test run::tests::a_childs_frames_are_folded_into_its_own_state ... ok
test run::tests::a_copied_selection_reaches_the_terminals_own_clipboard ... ok
test run::tests::a_dialog_answered_at_the_keyboard_reaches_the_kernel ... ok
test run::tests::a_frame_is_owed_while_the_one_on_the_screen_is_still_arriving ... ok
test run::tests::a_journal_pictures_bytes_are_written_out_and_the_file_opened ... ok
test run::tests::a_line_the_turn_took_first_leaves_the_box_empty_and_says_so ... ok
test run::tests::a_line_typed_in_a_childs_view_is_submitted_on_the_childs_handle ... ok
test run::tests::a_look_with_no_ground_to_follow_asks_the_terminal_nothing ... ok
test run::tests::a_mention_that_does_not_read_keeps_the_line_and_says_so ... ok
test run::tests::a_mentioned_bmp_reaches_the_session_as_a_png ... ok
test run::tests::a_mentioned_picture_is_read_from_the_sessions_directory ... ok
test run::tests::a_mentioned_url_is_fetched_by_this_machine ... ok
test run::tests::a_named_address_opens_the_file_its_fetched_bytes_were_written_to ... ok
test run::tests::a_named_path_is_opened_where_the_words_said_it_was ... ok
test run::tests::a_newer_release_reaches_the_welcome_box ... ok
test run::tests::a_notice_created_after_painting_owes_a_frame ... ok
test run::tests::a_permission_a_child_raised_is_answered_through_the_root_handle ... ok
test run::tests::a_picture_an_answer_named_is_read_in_between_frames_and_sent ... ok
test run::tests::a_picture_is_handed_to_the_terminal_once_and_not_again ... ok
test run::tests::a_picture_nothing_will_open_says_so_with_the_word ... ok
test run::tests::a_picture_that_is_not_there_is_a_dim_note_tried_once ... ok
test run::tests::a_picture_the_session_no_longer_holds_opens_nothing ... ok
test run::tests::a_question_that_opens_on_a_window_nobody_watches_says_so ... ok
test run::tests::a_selection_the_terminal_will_not_take_is_said_out_loud ... ok
test run::tests::a_slow_draw_is_owned_up_to_once_and_a_quick_one_never ... ok
test run::tests::a_stored_child_the_host_removed_leaves_the_tree ... ok
test run::tests::a_storm_of_deltas_costs_one_draw_a_frame_and_no_more ... ok
test run::tests::a_storm_of_wheel_events_costs_one_draw_a_frame_and_no_more ... ok
test run::tests::a_submitted_line_is_appended_to_the_prompt_history ... ok
test run::tests::a_terminal_that_draws_no_pictures_is_handed_nothing ... ok
test run::tests::a_terminal_that_draws_no_pictures_reads_nothing_in ... ok
test run::tests::a_turn_that_finishes_on_a_window_nobody_watches_says_so ... ok
test run::tests::a_url_that_is_not_a_picture_keeps_the_line_and_says_so ... ok
test run::tests::a_view_whose_mailbox_has_not_landed_writes_nowhere ... ok
test run::tests::a_welcome_box_that_has_scrolled_away_is_said_in_the_status_line ... ok
test run::tests::a_window_a_person_came_back_to_asks_what_ground_the_terminal_has ... ok
test run::tests::a_withdrawn_line_comes_back_to_the_composer_with_its_pictures ... ok
test run::tests::an_answer_in_the_key_stream_swaps_the_look_for_the_next_frame ... ok
test run::tests::an_idle_surface_draws_nothing_at_all ... ok
test run::tests::deleting_a_child_returns_its_viewed_descendant_and_drops_both_handles ... ok
test run::tests::esc_during_a_turn_interrupts_it ... ok
test run::tests::frames_are_folded_and_drawn_until_the_session_closes ... ok
test run::tests::leaving_hands_the_last_screenful_of_the_transcript_back ... ok
test run::tests::opening_the_switcher_reads_the_store_and_the_card_fills ... ok
test run::tests::the_cache_keeps_a_fortnight_unless_the_settings_said_otherwise ... ok
test run::tests::the_completions_follow_a_catalogue_the_host_rebuilt ... ok
test run::tests::the_first_prompt_is_submitted_as_soon_as_the_session_is_open ... ok
test run::tests::the_resume_picker_offers_roots_and_never_a_child_or_a_room ... ok
test run::tests::the_seconds_after_a_flash_cost_no_frames_at_all ... ok
test run::tests::the_slow_clock_runs_while_the_run_is_idle_and_stops_while_a_turn_does ... ok
test run::tests::the_switcher_opened_over_a_picture_writes_no_graphics ... ok
test run::tests::the_title_marks_a_session_that_needs_a_person ... ok
test run::tests::what_is_typed_reaches_the_kernel_and_ctrl_d_ends_the_run ... ok
test run::updates::tests::only_a_run_whose_bin_said_so_asks ... ok
test screens::a_call_that_lands_while_the_assistant_is_still_writing ... ok
test screens::a_child_row_that_wants_a_person ... ok
test screens::a_child_row_when_it_is_done ... ok
test screens::a_child_row_while_it_runs ... ok
test screens::a_child_transcript ... ok
test screens::a_diff_result_sits_on_its_tints ... ok
test screens::a_highlighted_code_block ... ok
test screens::a_long_path_fits_one_row ... ok
test screens::a_markdown_table_is_ruled ... ok
test screens::a_notice_and_a_context_warning ... ok
test screens::a_permission_card_previews_a_word_level_diff ... ok
test screens::a_view_a_command_answered ... ok
test screens::acp::a_call_still_running_reads_as_the_text_it_is ... ok
test screens::acp::an_agents_own_calls_read_as_rows ... ok
test screens::an_error_turn ... ok
test screens::an_interrupted_turn ... ok
test screens::colours::a_card_spends_its_colour_on_the_row_the_keyboard_is_on ... ok
test screens::colours::a_notice_is_marked_and_everybody_else_keeps_the_bar ... ok
test screens::colours::a_shell_line_sits_on_the_bar_your_own_words_sit_on ... ok
test screens::colours::a_skill_rows_mark_wears_the_state_and_spends_the_only_colour ... ok
test screens::colours::a_terminal_that_turns_light_under_a_drawn_screen_is_followed ... ok
test screens::colours::a_tool_rows_bullet_is_the_only_cell_a_colour_is_spent_on ... ok
test screens::colours::an_answer_row_is_white_after_its_bullet ... ok
test screens::colours::every_row_of_a_batch_wears_the_live_mark_at_once ... ok
test screens::colours::no_screen_paints_prose_in_a_colour_of_its_own ... ok
test screens::colours::the_foot_row_is_a_bar_with_the_count_in_text_and_the_hint_in_dim ... ok
test screens::colours::the_line_is_the_persons_own_and_only_a_bad_exit_is_coloured ... ok
test screens::colours::the_status_line_spends_no_colour_but_the_mode ... ok
test screens::colours::your_own_line_sits_on_a_bar_the_width_of_the_transcript ... ok
test screens::confirm ... ok
test screens::every_fenced_language_is_inked_by_the_token_table ... ok
test screens::forms::form_asked ... ok
test screens::forms::form_in_ascii ... ok
test screens::forms::form_layout ... ok
test screens::forms::form_part_answered ... ok
test screens::forms::form_set_ticked_and_typed_on ... ok
test screens::forms::form_submit_with_one_left ... ok
test screens::forms::the_band_holds_still_as_the_tabs_are_walked ... ok
test screens::held::held_back_from_the_foot ... ok
test screens::idle ... ok
test screens::in_daylight ... ok
test screens::login_browser ... ok
test screens::login_device ... ok
test screens::login_paste ... ok
test screens::opening::the_box_draws_itself_and_then_becomes_itself ... ok
test screens::opening::the_light_palette_draws_the_same_box_in_its_own_ink ... ok
test screens::opening::the_piece_moves_nothing_but_the_boxs_own_cells ... ok
test screens::permission_collapsed ... ok
test screens::permission_expanded ... ok
test screens::permission_with_the_feedback_row_open ... ok
test screens::pictures::one_picture_sent ... ok
test screens::pictures::pictures_on_a_terminal_that_draws_none ... mbx[progress]: 285s elapsed; 0 hits, 0 misses, 0 bypassed, 0 not looked up; ~0.0s compiler work saved
ok
test screens::pictures::six_pictures_sent ... ok
test screens::pictures::three_pictures_sent ... ok
test screens::questions::question_multi ... ok
test screens::questions::question_single ... ok
test screens::quiet_notices ... ok
test screens::ran::shell_lines ... ok
test screens::ran::shell_lines_in_ascii ... ok
test screens::shells::shell_and_wake_follow_plugin_order ... ok
test screens::shells::shell_ran ... ok
test screens::shells::shell_running ... ok
test screens::skills::a_skill_command ... ok
test screens::skills::a_skill_the_model_called ... ok
test screens::skills::a_skill_without_the_glyphs ... ok
test screens::stopping ... ok
test screens::streaming ... ok
test screens::tasks::activity_hidden ... ok
test screens::tasks::tasks_at_work ... ok
test screens::tasks::tasks_between_turns ... ok
test screens::tasks::tasks_cut_to_five ... ok
test screens::tasks::tasks_in_ascii ... ok
test screens::teams::a_room_post_in_a_member_s_own_transcript ... ok
test screens::teams::a_room_transcript ... ok
test screens::teams::the_roster ... ok
test screens::teams::the_roster_spends_colour_only_where_the_design_says ... ok
test screens::teams::the_roster_with_the_cursor_on_a_room ... ok
test screens::teams::the_two_doors_open_byte_identical_lists ... ok
test screens::teams::what_a_room_is_owed ... ok
test screens::teams::what_the_message_tools_answer ... ok
test screens::the_at_completion_dropdown ... ok
test screens::the_at_completion_dropdown_with_agents_to_reach ... ok
test screens::the_command_dropdown ... ok
test screens::the_help_sheet ... ok
test screens::the_pager_sheet ... ok
test screens::the_panel_sheet ... ok
test screens::the_panel_sheet_reads_a_word_it_has_not_learned ... ok
test screens::the_rewind_picker ... ok
test screens::the_switcher_dropdown ... ok
test screens::the_switcher_dropdown_narrowed_by_a_query ... ok
test screens::the_switcher_lists_what_is_only_in_the_store ... ok
test screens::thinking::a_finished_thought_holds_the_rows_it_ended_on ... ok
test screens::thinking::a_finished_thought_opens_whole_where_it_sits ... ok
test screens::thinking::a_finished_thought_shuts_where_the_ring_comes_round ... ok
test screens::thinking::a_run_of_thoughts_draws_as_one_thought ... ok
test screens::thinking::a_run_whose_last_thought_is_still_being_had ... ok
test screens::thinking::a_thought_being_had_streams_under_its_own_row ... ok
test screens::thinking::a_thought_between_a_question_and_its_answer ... ok
test screens::thinking::a_thought_of_one_paragraph_is_still_two_rows ... ok
test screens::thinking::a_thought_opens_in_a_sheet ... ok
test screens::thinking::an_empty_thought_draws_the_row_alone ... ok
test screens::tool_done_with_output ... ok
test screens::tool_running ... ok
test screens::two_tools_running_at_once ... ok
test screens::welcome_on_a_fresh_session ... ok
test screens::welcome_with_a_newer_release_out ... ok
test screens::windows::a_card_walks_its_answers_without_losing_the_one_it_is_on ... ok
test screens::windows::a_form_keeps_its_tabs_and_the_answer_the_cursor_is_on ... ok
test screens::windows::a_form_with_more_answers_than_room_windows_them_around_the_cursor ... ok
test screens::windows::the_command_dropdown_scrolls_under_the_row_the_keyboard_is_on ... ok
test screens::windows::the_panel_sheet_starts_at_the_row_the_keyboard_is_on ... ok
test screens::windows::the_resume_picker_scrolls_under_the_row_the_keyboard_is_on ... ok
test screens::windows::the_rewind_card_scrolls_under_the_row_the_keyboard_is_on ... ok
test screens::windows::the_switcher_scrolls_under_the_row_the_keyboard_is_on ... ok
test screens::without_colour ... ok
test screens::without_the_glyphs ... ok
test scroll::tests::a_burst_of_notches_lands_the_whole_of_itself ... ok
test scroll::tests::a_held_transcript_keeps_its_line_while_more_arrives ... ok
test scroll::tests::a_move_that_lands_past_the_top_stops_at_it ... ok
test scroll::tests::a_page_back_eases_over_a_tenth_of_a_second ... ok
test scroll::tests::a_transcript_shorter_than_its_region_is_never_scrolled ... ok
test scroll::tests::coming_back_to_the_foot_follows_the_tail_again ... ok
test scroll::tests::home_goes_to_the_first_line_and_stays_there ... ok
test scroll::tests::revealing_a_line_above_the_top_puts_it_at_the_top ... ok
test scroll::tests::revealing_a_line_below_the_bottom_puts_it_at_the_bottom ... ok
test scroll::tests::revealing_a_line_that_is_on_the_screen_moves_nothing ... ok
test scroll::tests::revealing_line_after_line_lands_every_one_of_them ... ok
test scroll::tests::showing_a_line_puts_it_a_third_down_the_region ... ok
test scroll::tests::the_tail_is_the_last_screenful_and_follows_what_arrives ... ok
test search::tests::a_committed_query_finds_every_occurrence_in_reading_order ... ok
test search::tests::a_hit_is_measured_in_cells_so_a_wide_glyph_does_not_shift_it ... ok
test search::tests::a_query_nothing_matches_says_so_and_steps_nowhere ... ok
test search::tests::an_empty_query_finds_nothing_rather_than_everything ... ok
test search::tests::stepping_walks_the_hits_and_wraps_both_ways ... ok
test search::tests::the_hits_on_a_line_know_which_one_the_eye_is_on ... ok
test seats::tests::a_card_from_before_the_debts_says_the_clock_time_it_carries ... ok
test seats::tests::a_debt_in_another_room_is_not_this_rooms ... ok
test seats::tests::a_listener_wears_the_patience_the_roster_declared_for_it ... ok
test seats::tests::a_mark_of_another_seat_or_another_shape_counts_nothing ... ok
test seats::tests::a_member_sits_in_the_room_whose_roster_names_it ... ok
test seats::tests::a_member_that_owes_twice_is_one_debtor_and_says_the_older ... ok
test seats::tests::a_payload_of_another_shape_says_nothing_rather_than_guessing ... ok
test seats::tests::a_room_says_how_many_seats_it_has_and_how_many_of_them_owe ... ok
test seats::tests::a_roster_that_declares_no_ear_seats_everyone_patient ... ok
test seats::tests::a_seat_behind_the_rooms_head_counts_what_it_has_not_read ... ok
test seats::tests::a_seat_that_retuned_its_own_ear_is_heard_over_the_roster ... ok
test select::tests::a_row_off_the_region_is_the_rows_it_is_past_it_by ... ok
test select::tests::a_row_on_the_region_is_past_no_edge ... ok
test select::tests::a_run_across_lines_keeps_the_line_breaks ... ok
test select::tests::a_run_drawn_backwards_is_the_same_run ... ok
test select::tests::a_run_inside_one_line_is_the_cells_between_its_ends ... ok
test select::tests::a_run_that_reaches_nowhere_is_a_click ... ok
test select::tests::a_selection_is_the_cells_that_were_read_not_the_bytes_behind_them ... ok
test select::tests::a_selection_too_large_for_the_terminal_is_refused_by_name ... ok
test select::tests::an_edge_at_the_end_of_the_transcript_has_nowhere_further_to_go ... ok
test select::tests::an_edge_scrolls_towards_what_it_points_at ... ok
test select::tests::osc_52_carries_the_selection_as_base64 ... ok
test select::tests::the_cells_it_holds_are_the_ones_that_are_tinted ... ok
test select::tests::the_line_at_an_edge_is_the_first_or_the_last_one_drawn ... ok
test select::tests::the_line_below_a_short_transcript_is_its_last ... ok
test select::tests::walking_moves_the_far_end_and_stops_at_the_foot ... ok
test settings::tests::a_neighbouring_key_is_not_this_ones_business ... ok
test settings::tests::a_slice_that_is_not_this_shape_leaves_the_transcript_where_it_was ... ok
test settings::tests::an_unknown_key_inside_a_claimed_slice_is_not_a_setting ... ok
test settings::tests::prose_has_the_width_it_is_drawn_in_until_a_person_names_one ... ok
test settings::tests::the_claim_carries_a_schema_for_both_keys ... ok
test settings::tests::what_the_bin_handed_over_is_what_the_run_draws_at ... ok
test status::tests::a_context_the_agent_holds_reads_as_its_own_count_and_its_own_cut ... ok
test status::tests::a_line_too_narrow_for_three_slots_keeps_the_place ... ok
test status::tests::a_model_with_no_provider_stands_alone ... ok
test status::tests::a_notice_takes_the_middle_while_it_lasts ... ok
test status::tests::a_pending_wake_is_counted_down_and_goes_when_it_is_taken_back ... ok
test status::tests::a_pending_wake_is_furniture_and_the_whole_line_still_fits ... ok
test status::tests::a_renamed_session_is_renamed_on_the_line ... ok
test status::tests::an_idle_root_says_only_the_hint_and_the_model ... ok
test status::tests::every_slot_filled_and_every_slot_empty ... ok
test status::tests::the_children_that_want_you_and_the_ones_at_work_are_counted ... ok
test status::tests::the_context_count_is_dim_until_it_is_bad ... ok
test status::tests::the_context_count_sits_before_the_model_and_says_the_way_out_late ... ok
test status::tests::the_hint_goes_when_something_is_typed ... ok
test status::tests::the_middle_is_the_slot_that_gives_way ... ok
test status::tests::the_mode_the_policy_published_sits_on_the_left ... ok
test status::tests::the_right_slot_says_provider_model_and_effort ... ok
test status::tests::the_session_in_view_is_never_one_of_its_own_notices ... ok
test terminal::tests::a_frame_goes_bare_through_a_multiplexer_and_bracketed_everywhere_else ... ok
test terminal::tests::a_frame_is_bracketed_by_the_two_halves_of_mode_2026 ... ok
test terminal::tests::a_multiplexer_is_passed_through_with_the_escape_doubled ... ok
test terminal::tests::a_notification_is_one_osc_sequence_in_the_dialect_the_terminal_takes ... ok
test terminal::tests::a_title_is_one_osc_two_sequence_with_the_control_bytes_stripped ... ok
test terminal::tests::the_title_stack_codes_are_the_xterm_ones ... ok
test terminal::tests::the_two_terminals_with_their_own_escape_get_it_and_the_rest_get_777 ... ok
test test_lanes::a_row_with_no_body_keeps_the_item_and_a_wide_mark_keeps_the_bullet ... ok
test test_lanes::a_stranger_places_every_slot_without_a_surface_registration ... ok
test test_lanes::an_unrelated_placement_does_not_move_a_pinned_panel ... ok
test test_lanes::quiet_is_a_record_flag_and_still_shows_displays_and_failures ... ok
test theme::tests::a_border_with_light_on_it_runs_from_its_own_hairline_to_the_glow ... ok
test theme::tests::a_breath_is_five_steps_of_presence_on_twenty_four_bits ... ok
test theme::tests::a_ground_answered_again_swaps_the_palette ... ok
test theme::tests::a_look_is_only_switched_for_the_thread_that_asked ... ok
test theme::tests::a_look_with_no_ground_to_follow_never_swaps ... ok
test theme::tests::a_mark_in_the_bullets_place_is_the_bullets_width ... ok
test theme::tests::a_named_look_outranks_what_the_terminal_answered ... ok
test theme::tests::a_notice_arrives_out_of_dim_and_leaves_into_it ... ok
test theme::tests::a_ramp_on_twenty_four_bits_passes_between_its_ends ... ok
test theme::tests::a_ramp_the_eight_colours_cannot_draw_takes_its_two_ends ... ok
test theme::tests::a_ramp_towards_the_ink_ends_on_the_terminals_own ... ok
test theme::tests::a_swap_leaves_the_glyphs_where_they_were ... ok
test theme::tests::a_terminal_that_answers_light_gets_the_light_palette ... ok
test theme::tests::an_osc_eleven_reply_says_which_ground_the_terminal_has ... ok
test theme::tests::each_palette_reads_against_its_own_ground ... ok
test theme::tests::every_token_is_spent_where_the_design_says ... ok
test theme::tests::light_and_dark_differ_only_in_what_each_token_is_worth ... ok
test theme::tests::no_colour_beats_a_terminal_that_answered_light ... ok
test theme::tests::no_colour_keeps_the_weights_and_drops_the_hues ... ok
test theme::tests::no_view_names_a_colour_of_its_own ... ok
test theme::tests::nothing_that_moves_spends_a_colour_under_no_colour ... ok
test theme::tests::the_ascii_table_spells_every_glyph_in_one_cell ... ok
test theme::tests::the_environment_picks_the_look_and_no_colour_wins ... ok
test theme::tests::the_ground_is_light_from_the_middle_grey_up ... ok
test theme::tests::the_ground_survives_the_number_it_is_kept_in ... ok
test theme::tests::the_ink_and_the_dim_are_the_terminals_own_in_every_look ... ok
test theme::tests::the_sparkle_walks_its_four_frames_on_the_clock ... ok
test theme::tests::what_is_not_an_answer_to_that_question_changes_no_look ... ok
test thoughts::tests::a_run_is_the_thoughts_adjacent_before_the_one_asked_about ... ok
test thoughts::tests::a_run_says_its_texts_in_order_with_a_break_between_them ... ok
test thoughts::tests::a_run_took_as_long_as_its_finished_thoughts_together ... ok
test thoughts::tests::a_thought_is_a_reasoning_item_that_is_not_a_call ... ok
test thoughts::tests::anything_between_two_thoughts_breaks_the_run ... ok
test thoughts::tests::only_the_last_thought_of_a_run_ends_it ... ok
test thoughts::tests::the_run_of_an_item_holds_every_thought_adjacent_to_it ... ok
test transcript::output::tests::a_break_above_the_last_sentence_is_not_a_row_of_the_tail ... ok
test transcript::output::tests::a_paragraph_break_does_not_shrink_the_count ... ok
test transcript::output::tests::a_text_shorter_than_the_cut_is_kept_whole ... ok
test transcript::output::tests::a_text_that_ends_blank_tails_to_the_rows_before_the_blank ... ok
test transcript::output::tests::a_wide_paragraph_is_cut_by_cells ... ok
test transcript::output::tests::nothing_arrived_yet_is_no_rows_at_all ... ok
test transcript::output::tests::one_long_paragraph_is_cut_to_the_rows_it_ends_on ... ok
test transcript::output::tests::wrapping_the_tail_again_at_its_own_width_changes_nothing ... ok
test transcript::pictured::tests::a_block_kept_between_frames_keeps_its_picture ... ok
test transcript::pictured::tests::a_destination_not_yet_read_in_is_the_chip_and_a_word_to_the_run ... ok
test transcript::pictured::tests::a_destination_that_failed_says_why_after_the_name ... ok
test transcript::pictured::tests::a_nested_item_and_a_quote_stand_one_construct_further_in ... ok
test transcript::pictured::tests::a_picture_a_person_pasted_adds_no_chip ... ok
test transcript::pictured::tests::a_picture_a_person_pasted_stands_in_a_band_above_their_line ... ok
test transcript::pictured::tests::a_picture_an_answer_named_draws_under_its_own_chip ... ok
test transcript::pictured::tests::a_picture_in_a_bullet_stands_at_the_bullets_text_column ... ok
test transcript::pictured::tests::a_picture_in_a_paragraph_stands_at_the_words_column ... ok
test transcript::pictured::tests::a_picture_no_decoder_reads_is_the_chip_on_any_terminal ... ok
test transcript::pictured::tests::a_pictures_cells_are_not_printed_back_into_the_shell ... ok
test transcript::pictured::tests::a_terminal_that_draws_no_pictures_draws_the_chip ... ok
test transcript::pictured::tests::a_terminal_that_draws_no_pictures_draws_the_chip_the_words_carry ... ok
test transcript::pictured::tests::a_tool_that_answered_with_a_picture_draws_its_cells ... ok
test transcript::pictured::tests::every_cell_carries_the_picture_it_belongs_to ... ok
test transcript::pictured::tests::six_pictures_show_four_and_count_the_rest ... ok
test transcript::pictured::tests::the_fold_hides_the_band_and_never_grows_it ... ok
test transcript::pictured::tests::the_fold_opens_a_picture_the_words_named ... ok
test transcript::pictured::tests::the_fold_says_how_much_of_a_picture_is_shown ... ok
test transcript::pictured::tests::the_frame_draws_the_band_above_the_persons_row ... ok
test transcript::pictured::tests::the_frame_draws_the_cells_and_names_the_picture ... ok
test transcript::pictured::tests::the_frame_draws_the_cells_of_a_picture_the_words_named ... ok
test transcript::pictured::tests::the_same_destination_named_twice_is_one_picture ... ok
test transcript::pictured::tests::three_pictures_stand_side_by_side_on_one_band ... ok
test transcript::pictured::tests::two_pictures_in_one_answer_hang_from_their_own_chips ... ok
test transcript::ran::tests::a_clean_exit_adds_no_row_and_a_shut_block_has_none ... ok
test transcript::ran::tests::a_failing_line_keeps_its_code_however_much_output_was_folded_away ... ok
test transcript::ran::tests::a_line_that_reached_no_code_shows_only_what_it_wrote ... ok
test transcript::said::voice_contract::stamped_voice_wins_and_legacy_words_are_frozen ... ok
test transcript::slots::fixtures::a_stranger_places_a_row_with_a_title_badge_and_action ... ok
test transcript::tests::a_call_between_two_thoughts_keeps_them_two_blocks ... ok
test transcript::tests::a_call_on_a_memory_file_is_drawn_as_remembering ... ok
test transcript::tests::a_command_the_catalogue_calls_no_skill_is_the_line_that_was_typed ... ok
test transcript::tests::a_failed_call_wears_the_row_a_failed_tool_wears ... ok
test transcript::tests::a_failed_turn_is_a_bullet_of_its_own ... ok
test transcript::tests::a_finished_job_reads_as_a_tool_row_does ... ok
test transcript::tests::a_finished_thought_holds_its_tail_then_opens_whole_then_shuts ... ok
test transcript::tests::a_long_notice_folds_without_promising_a_key ... ok
test transcript::tests::a_long_result_says_how_much_it_folded_away_and_what_opens_it ... ok
test transcript::tests::a_long_thought_holds_the_two_rows_it_ended_on ... ok
test transcript::tests::a_members_transcript_draws_none_of_its_rooms_machinery ... ok
test transcript::tests::a_receipt_joins_the_row_that_asked_for_it ... ok
test transcript::tests::a_receipt_with_no_call_above_it_names_its_own_tool ... ok
test transcript::tests::a_room_reads_as_a_conversation_does ... ok
test transcript::tests::a_run_of_minutes_reads_as_minutes_and_seconds ... ok
test transcript::tests::a_run_of_thoughts_draws_as_one_thought ... ok
test transcript::tests::a_run_whose_last_thought_is_still_being_had_is_thinking ... ok
test transcript::tests::a_running_tool_shows_the_last_rows_of_its_tail ... ok
test transcript::tests::a_running_tools_long_lines_are_cut_to_rows ... ok
test transcript::tests::a_second_line_of_yours_stays_on_the_bar_under_the_first ... ok
test transcript::tests::a_skill_call_that_names_nothing_is_drawn_as_a_tool_row ... ok
test transcript::tests::a_skill_with_no_arguments_is_the_signature_alone ... ok
test transcript::tests::a_streaming_thought_never_changes_the_height_of_its_block ... ok
test transcript::tests::a_tab_in_a_result_runs_to_the_next_stop_of_eight ... ok
test transcript::tests::a_task_call_draws_no_row_unless_the_list_cannot_say_what_it_did ... ok
test transcript::tests::a_thin_mark_from_an_older_build_still_draws_a_row ... ok
test transcript::tests::a_thought_being_had_streams_two_rows_of_its_newest_text ... ok
test transcript::tests::a_thought_of_one_long_paragraph_still_streams_two_rows ... ok
test transcript::tests::a_thought_that_closes_holds_the_rows_it_was_streaming ... ok
test transcript::tests::a_thought_with_nothing_in_it_yet_is_the_row_alone ... ok
test transcript::tests::a_thought_without_the_mark_is_untouched ... ok
test transcript::tests::a_tool_row_names_the_call_and_hangs_its_result_under_it ... ok
test transcript::tests::a_typed_skill_reads_as_the_run_it_is ... ok
test transcript::tests::a_wake_is_a_line_on_the_bar_under_a_word_of_its_own ... ok
test transcript::tests::an_agents_call_still_running_reads_as_the_text_it_is ... ok
test transcript::tests::an_agents_message_names_the_agent_on_the_marked_line ... ok
test transcript::tests::an_agents_own_call_draws_as_a_tool_row_and_not_as_a_thought ... ok
test transcript::tests::an_empty_thought_is_the_row_and_nothing_else ... ok
test transcript::tests::an_opaque_item_draws_no_terminal_cells ... ok
test transcript::tests::an_opened_result_shows_every_line ... ok
test transcript::tests::an_unsigned_post_is_the_holders_and_says_so ... ok
test transcript::tests::every_quiet_surface_is_a_marked_line_and_nothing_else_is ... ok
test transcript::tests::internal_context_does_not_supply_a_session_brief_or_rewind_prompt ... ok
test transcript::tests::internal_context_never_appears_as_a_transcript_row ... ok
test transcript::tests::only_the_rooms_own_origins_are_kept_out_of_a_member ... ok
test transcript::tests::prose_fills_the_width_until_a_person_asks_for_less ... ok
test transcript::tests::the_model_speaks_after_a_bold_bullet_and_keeps_its_indent ... ok
test transcript::tests::the_models_own_call_draws_the_same_row ... ok
test transcript::tests::thinking_streams_and_then_holds_the_rows_it_streamed ... ok
test transcript::thinking::tests::how_long_it_took_reads_as_a_person_reads_a_clock ... ok
test tree::tests::a_chain_that_never_reaches_the_root_ends_all_the_same ... ok
test tree::tests::a_child_is_found_by_the_tool_call_that_spawned_it ... ok
test tree::tests::a_child_joins_the_tree_on_the_session_frame_at_the_head_of_its_stream ... ok
test tree::tests::a_child_row_says_what_it_is_doing_and_what_it_has_spent ... ok
test tree::tests::a_child_that_closed_is_stored_until_it_is_announced_again ... ok
test tree::tests::a_count_is_scoped_to_who_is_asking ... ok
test tree::tests::a_session_nothing_answers_reports_no_status_at_all ... ok
test tree::tests::a_session_that_is_both_live_and_stored_is_one_row_and_the_live_one ... ok
test tree::tests::a_session_the_tree_has_not_heard_of_is_waited_for_not_forgotten ... ok
test tree::tests::a_stored_row_is_dim_and_a_stored_room_still_answers_nothing ... ok
test tree::tests::the_brief_is_the_first_line_of_the_first_thing_a_session_was_asked ... ok
test tree::tests::the_roster_puts_the_tree_first_and_the_stored_rows_after_it_by_id ... ok
test tree::tests::the_roster_takes_only_the_sessions_whose_parent_chain_reaches_the_root ... ok
test tree::tests::the_row_under_a_call_that_opened_a_room_is_its_name_alone ... ok
test tree::tests::the_rows_are_the_root_first_and_the_rest_in_id_order ... ok
test tree::tests::the_tally_counts_who_is_waiting_and_the_dialog_takes_the_root_first ... ok
test tree::tests::the_view_is_a_key_into_the_tree_and_falls_back_to_the_root ... ok
test ui::tests::a_notice_fades_in_over_two_frames_holds_and_fades_out ... ok
test ui::tests::a_notice_lives_its_window_and_the_frames_it_arrives_and_leaves_in ... ok
test ui::tests::a_notice_the_terminal_disproved_is_withdrawn ... ok
test ui::tests::a_still_surface_says_a_notice_at_full_strength ... ok
test ui::tests::dismissing_the_dropdown_hides_it_until_the_next_edit ... ok
test ui::tests::the_exit_arm_lapses ... ok
test ui::tests::the_next_notice_waits_until_the_first_has_left ... ok
test view::tests::a_card_comes_down_from_its_top_edge ... ok
test view::tests::a_card_hangs_under_the_row_that_asked_for_it ... ok
test view::tests::a_carried_picture_puts_a_strip_on_the_box ... ok
test view::tests::a_child_that_needs_a_person_is_counted_on_the_status_line ... ok
test view::tests::a_completed_tool_shows_the_first_lines_of_its_output ... ok
test view::tests::a_config_without_a_mode_leaves_the_status_line_as_it_was ... ok
test view::tests::a_diff_result_is_coloured_by_column ... ok
test view::tests::a_display_view_is_drawn_under_the_tool_row_that_made_it ... ok
test view::tests::a_drag_over_a_long_transcript_tints_the_cells_under_the_pointer ... ok
test view::tests::a_drag_over_a_short_transcript_tints_the_cells_under_the_pointer ... ok
test view::tests::a_drag_over_wide_glyphs_tints_the_cells_they_are_drawn_in ... ok
test view::tests::a_failed_tool_and_a_failed_turn_are_both_red ... ok
test view::tests::a_full_draw_of_a_long_transcript_reuses_the_cached_blocks ... ok
test view::tests::a_hit_on_a_short_transcript_is_marked_where_it_sits ... ok
test view::tests::a_hit_on_the_screen_is_marked_where_it_sits ... ok
test view::tests::a_line_that_will_not_steer_wears_the_tag_and_the_busy_box_says_why ... ok
test view::tests::a_rail_is_not_drawn_for_a_session_no_plugin_has_written_to ... ok
test view::tests::a_rejected_intent_becomes_a_notice ... ok
test view::tests::a_retrying_turn_says_which_attempt ... ok
test view::tests::a_room_s_board_is_drawn_in_the_room_s_view_and_not_in_its_parent_s ... ok
test view::tests::a_room_sits_under_its_own_label ... ok
test view::tests::a_room_transcript_reads_as_a_chat ... ok
test view::tests::a_rooms_prompt_keeps_its_row_under_the_strip ... ok
test view::tests::a_row_wider_than_the_card_is_cut_and_pushes_nothing_down ... ok
test view::tests::a_running_tool_shows_its_tail ... ok
test view::tests::a_screen_too_short_for_both_keeps_the_prompt_and_not_the_strip ... ok
test view::tests::a_steer_in_flight_asks_the_frame_for_no_row_at_all ... ok
test view::tests::a_stored_row_turns_live_in_place_when_its_frames_arrive ... ok
test view::tests::a_terminal_that_draws_no_pictures_gets_no_strip ... ok
test view::tests::a_terminal_too_small_for_anything_still_draws ... ok
test view::tests::a_tool_call_that_spawned_an_agent_says_what_it_is_doing ... ok
test view::tests::a_view_table_sits_above_the_composer ... ok
test view::tests::a_wide_terminal_is_filled_unless_a_person_set_a_measure ... ok
test view::tests::an_interrupted_turn_keeps_its_marker ... ok
test view::tests::confirm_dialog ... ok
test view::tests::ctrl_f_searches_the_transcript_and_esc_gives_the_status_line_back ... ok
test view::tests::ctrl_o_lifts_the_fold_then_opens_the_pager_and_esc_folds_it_back ... ok
test view::tests::ctrl_p_shows_what_the_plugins_wrote_into_the_session ... ok
test view::tests::deleting_the_token_takes_the_strip_with_it ... ok
test view::tests::dropdown ... ok
test view::tests::everything_behind_a_card_is_dim ... ok
test view::tests::five_carried_pictures_show_four_and_a_count ... ok
test view::tests::help_panel ... ok
test view::tests::idle ... ok
test view::tests::login_browser_dialog ... ok
test view::tests::login_device_dialog ... ok
test view::tests::login_paste_dialog_with_the_words_row_open ... ok
test view::tests::permission_collapsed ... ok
test view::tests::permission_expanded ... ok
test view::tests::permission_with_the_feedback_row_open ... ok
test view::tests::question_multi ... ok
test view::tests::question_single ... ok
test view::tests::streaming_assistant_text ... ok
test view::tests::the_composer_of_a_room_offers_to_post_to_it ... ok
test view::tests::the_composer_survives_a_screen_too_small_for_the_chrome ... ok
test view::tests::the_context_notice_sits_on_the_status_line_when_it_is_true ... ok
test view::tests::the_first_ctrl_c_says_how_to_leave ... ok
test view::tests::the_foot_row_costs_the_transcript_nothing ... ok
test view::tests::the_list_covers_the_transcript_and_moves_nothing_under_it ... ok
test view::tests::the_pager_scrolls_searches_and_leaves_the_frame_as_it_found_it ... ok
test view::tests::the_panel_shows_the_session_in_view_and_says_when_it_is_empty ... ok
test view::tests::the_pending_area_draws_only_what_the_person_queued ... ok
test view::tests::the_rail_holds_the_pinned_board_and_the_live_progress_card ... ok
test view::tests::the_session_picker_lists_what_can_be_resumed ... ok
test view::tests::the_status_line_names_the_mode_the_policy_published ... ok
test view::tests::the_switcher_lists_the_root_and_its_agents ... ok
test view::tests::the_transcript_scrolls_back_a_page ... ok
test view::tests::the_view_of_a_child_is_its_own_transcript_under_its_own_name ... ok
test view::tests::the_word_after_a_verb_is_drawn_under_the_verb ... ok
test view::tests::without_a_rail_the_same_cards_draw_under_the_running_rows ... ok
test viewer::tests::a_path_the_words_named_is_opened_where_it_was_written ... ok
test viewer::tests::a_picture_that_is_gone_or_unreadable_is_no_word_at_all ... ok
test viewer::tests::a_picture_that_knows_its_file_is_opened_there_while_it_is_there ... ok
test viewer::tests::an_address_is_opened_as_the_file_its_bytes_were_written_to ... ok
test viewer::tests::bytes_are_written_out_under_the_number_they_are_known_by ... ok
test viewer::tests::the_word_goes_to_the_opener_and_a_refusal_names_it ... ok
test views::progress::tests::a_bounded_fill_lights_from_the_head_and_ramps_across_what_it_lit ... ok
test views::progress::tests::a_bounded_fill_ramps_from_presence_to_its_glow ... ok
test views::progress::tests::the_sheen_walks_the_track_and_comes_back_round ... ok
test views::progress::tests::the_walk_is_free_running_and_parked_when_nothing_moves ... ok
test views::tests::a_fired_button_wears_the_mark_until_the_answer_comes_back ... ok
test views::tests::a_key_is_the_plugins_hint_else_where_the_button_sits ... ok
test views::tests::a_missing_cell_is_a_dash_and_numbers_hug_their_right_edge ... ok
test views::tests::a_row_wider_than_the_frame_is_cut_and_says_so ... ok
test views::tests::an_unbounded_bar_walks_its_sheen_along_the_track ... ok
test views::tests::an_unbounded_progress_shows_its_count_and_not_a_fraction ... ok
test views::tests::buttons_that_do_not_fit_the_width_go_on_the_next_row ... ok
test views::tests::columns_stack_below_sixty_cells ... ok
test views::tests::every_node_of_the_vocabulary_has_a_screen_of_its_own ... ok
test views::tests::the_actions_of_a_view_are_found_however_deep_they_sit ... ok
test welcome::tests::a_fresh_session_is_welcomed_in_a_box_of_its_own_width ... ok
test welcome::tests::a_newer_release_is_one_more_row_under_the_help_line ... ok
test welcome::tests::a_session_this_surface_did_not_open_is_not_welcomed ... ok
test welcome::tests::the_box_is_as_wide_as_the_transcript_it_opens ... ok
test welcome::tests::the_mark_stands_where_the_opening_walks_its_block_down_to ... ok
test welcome::tests::the_opening_plays_on_a_fresh_session_and_nowhere_else ... ok
test window::tests::a_cursor_at_the_end_keeps_the_last_row_and_marks_the_head ... ok
test window::tests::a_cursor_at_the_top_keeps_the_first_row_and_marks_the_foot ... ok
test window::tests::a_cursor_in_the_middle_marks_both_ends ... ok
test window::tests::a_list_shorter_than_its_room_is_drawn_whole ... ok
test window::tests::a_room_of_one_row_spends_it_on_the_cursor ... ok
test window::tests::a_room_of_two_rows_keeps_the_cursor_and_the_mark_that_still_fits ... ok
test window::tests::a_window_never_outgrows_its_room_and_never_loses_the_cursor ... ok
test window::tests::no_room_draws_nothing ... ok
test window::tests::onward_starts_at_the_row_the_cursor_is_on ... ok
test worked::tests::a_duration_is_two_units_at_most ... ok
test worked::tests::a_finished_turn_says_what_it_cost_under_its_answer ... ok
test worked::tests::a_new_turn_takes_the_closing_away_and_a_wait_stands_beside_it ... ok
test worked::tests::a_turn_that_answered_at_once_and_a_session_with_no_turn_say_nothing ... ok
test worked::tests::the_clock_is_the_hour_today_and_the_day_before_it_otherwise ... ok
test worked::tests::the_closing_stands_under_the_answer_and_not_over_the_composer ... ok
test worked::tests::the_verdict_picks_the_verb ... ok
test wrap::tests::a_measure_is_the_region_until_a_person_asks_for_less ... ok
test wrap::tests::a_word_wider_than_the_line_fills_the_line_it_starts_on ... ok
test wrap::tests::a_word_wider_than_the_line_is_cut ... ok
test wrap::tests::an_empty_line_stays_one_line ... ok
test wrap::tests::styles_survive_the_break ... ok
test wrap::tests::wide_glyphs_count_two_columns ... ok
test wrap::tests::words_break_at_spaces_and_drop_the_space_at_the_break ... ok

test result: ok. 1132 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 9.01s

     Running unittests src/lib.rs (target/debug/deps/bingo_tasks-163ccaec541ec70b)

running 76 tests
test board::tests::a_board_and_who_is_on_it_cost_one_walk ... ok
test board::tests::a_board_sees_the_room_s_holder_and_everything_beside_it ... ok
test board::tests::a_claim_is_stamped_with_the_caller_s_own_title ... ok
test board::tests::a_name_out_of_reach_says_which_rooms_are_here ... ok
test board::tests::a_person_writes_the_board_as_two_words_or_leaves_it_out ... ok
test board::tests::a_room_resolves_from_the_session_that_opened_it_and_from_a_member ... ok
test board::tests::a_session_that_answers_is_never_a_board ... ok
test board::tests::the_word_in_is_what_the_schema_carries ... ok
test board::tests::without_a_name_the_list_is_the_caller_s_own ... ok
test command::tests::a_room_s_board_is_the_same_table_and_names_who_is_gone ... ok
test command::tests::a_session_with_no_tasks_gets_one_line ... ok
test command::tests::an_empty_board_names_the_room_and_an_unreachable_one_is_refused ... ok
test command::tests::the_spec_runs_now_and_takes_a_board_or_nothing ... ok
test command::tests::the_table_has_a_row_per_task_in_the_columns_of_the_headers ... ok
test contributor::tests::a_host_that_cannot_be_read_is_a_notice_not_a_dead_turn ... ok
test contributor::tests::a_list_that_is_all_done_says_there_are_no_open_tasks ... ok
test contributor::tests::a_session_with_no_tasks_says_there_are_no_open_tasks ... ok
test contributor::tests::it_refreshes_at_the_start_of_each_round ... ok
test contributor::tests::task_changes_append_and_completion_clears_the_previous_snapshot ... ok
test contributor::tests::the_open_tasks_reach_the_prompt ... ok
test create::tests::a_board_nothing_answers_to_is_an_error_the_model_can_read ... ok
test create::tests::a_call_without_a_subject_is_invalid_input ... ok
test create::tests::a_task_created_in_a_room_lands_on_the_room_s_list ... ok
test create::tests::everything_a_draft_carries_reaches_the_journal ... ok
test create::tests::the_first_task_is_created_and_named_back ... ok
test create::tests::the_schema_is_a_bare_object_and_the_traits_are_the_crate_s ... ok
test get::tests::a_task_on_a_board_is_read_from_the_room_s_list ... ok
test get::tests::an_unknown_id_is_an_error_the_model_can_read ... ok
test get::tests::the_schema_asks_for_an_id ... ok
test get::tests::the_task_comes_back_as_the_json_it_was_written_as ... ok
test guide::tests::the_page_is_one_line_of_description_and_a_body_worth_reading ... ok
test guide::tests::the_page_names_every_column_the_table_shows ... ok
test guide::tests::the_page_names_every_status_a_task_may_have ... ok
test guide::tests::the_page_names_every_tool_and_command_this_plugin_registers ... ok
test guide::tests::this_plugin_claims_no_settings_for_the_page_to_name ... ok
test journal::tests::a_payload_that_is_not_a_task_list_reads_as_none ... ok
test journal::tests::a_session_that_wrote_nothing_has_no_tasks ... ok
test journal::tests::only_this_plugin_s_kind_is_the_list ... ok
test journal::tests::slots_upgrade_an_old_array_with_its_activity ... ok
test journal::tests::the_four_tools_share_one_list_and_nothing_else ... ok
test journal::tests::what_was_written_is_what_the_next_read_folds ... ok
test list::tests::a_board_marks_an_owner_no_session_here_answers_to ... ok
test list::tests::a_private_list_never_marks_an_owner ... ok
test list::tests::a_session_with_no_tasks_is_told_how_to_add_one ... ok
test list::tests::an_in_that_is_not_a_name_is_refused_rather_than_read_as_none ... ok
test list::tests::arguments_are_ignored ... ok
test list::tests::every_task_is_one_line_in_the_order_it_was_added ... ok
test list::tests::the_schema_is_an_empty_object ... ok
test plugin_tests::all_four_tools_are_quiet_only_on_the_local_board ... ok
test plugin_tests::every_tool_is_read_only_trusted_and_alone ... ok
test plugin_tests::registering_reads_nothing_and_contributes_what_the_manifest_promises ... ok
test plugin_tests::the_manifest_says_what_it_provides_and_claims_no_settings ... ok
test render::tests::a_line_names_the_id_the_status_and_the_subject ... ok
test render::tests::a_row_has_a_cell_for_every_header ... ok
test render::tests::an_empty_board_marks_everyone_and_a_private_list_marks_no_one ... ok
test render::tests::an_empty_list_tells_the_model_how_to_start_one ... ok
test render::tests::an_owner_and_what_holds_a_task_up_are_on_the_line_too ... ok
test render::tests::an_owner_nobody_here_answers_to_is_marked_gone ... ok
test render::tests::nothing_open_is_no_reminder_at_all ... ok
test render::tests::the_reminder_carries_the_open_ones_under_a_heading ... ok
test task::tests::a_change_touches_only_the_fields_it_names ... ok
test task::tests::a_new_task_is_pending_and_numbered_from_one ... ok
test task::tests::an_id_is_never_reused_once_its_task_is_completed ... ok
test task::tests::an_unknown_id_changes_nothing_and_finds_nothing ... ok
test task::tests::dependencies_accumulate_and_a_repeat_is_not_a_second_one ... ok
test task::tests::metadata_merges_by_key ... ok
test task::tests::the_json_a_host_reads_is_the_shape_it_knows ... ok
test task::tests::the_open_ones_are_everything_but_the_completed ... ok
test update::tests::a_board_write_is_gated_no_differently_than_a_private_one ... ok
test update::tests::a_claim_stamps_the_caller_s_own_name_without_it_saying_so ... ok
test update::tests::a_named_owner_is_still_an_assignment ... ok
test update::tests::a_root_claiming_is_refused_in_words_and_changes_nothing ... ok
test update::tests::a_status_change_is_reported_with_the_task ... ok
test update::tests::an_unknown_id_is_an_error_the_model_can_read ... ok
test update::tests::dependencies_and_metadata_accumulate_across_calls ... ok
test update::tests::the_schema_names_the_id_and_the_fields_that_may_change ... ok

test result: ok. 76 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

     Running unittests src/lib.rs (target/debug/deps/bingo_tool_bash-694fb85fe872dda8)

running 147 tests
test endless::tests::a_shape_the_table_is_unsure_of_is_left_alone ... ok
test endless::tests::every_reason_says_what_will_not_end ... ok
test endless::tests::the_table_answers_every_row ... ok
test idle::tests::the_reason_names_the_command_and_every_way_round_it ... ok
test idle::tests::the_table_answers_every_row ... ok
test jobs::tests::a_job_reads_its_id_back_out_of_the_log_it_writes ... ok
test jobs::tests::a_kill_is_asked_for_and_the_job_s_own_task_does_it ... ok
test jobs::tests::a_long_command_is_clipped_to_its_first_line ... ok
test jobs::tests::a_minted_id_is_short_prefixed_lowercase_and_unique ... ok
test jobs::tests::a_prefix_names_one_job_and_an_exact_id_beats_the_ids_it_prefixes ... ok
test jobs::tests::a_started_job_is_shown_as_its_own_kind_with_a_fold_for_everyone_else ... ok
test jobs::tests::a_state_says_how_the_job_ended ... ok
test jobs::tests::an_age_reads_the_way_a_person_says_it ... ok
test jobs::tests::an_empty_set_is_a_null_payload_which_is_what_removes_the_card ... ok
test jobs::tests::an_id_nobody_has_is_answered_with_the_ones_there_are ... ok
test jobs::tests::only_the_running_jobs_of_this_session_are_on_the_rail ... ok
test jobs::tests::slots_place_the_running_count_and_the_call_row ... ok
test jobs::tests::waiting_on_a_job_returns_the_moment_it_ends ... ok
test kill::tests::a_group_that_leaves_on_its_own_terms_reports_its_code ... ok
test kill::tests::a_job_that_had_already_ended_is_said_so_and_nothing_is_signalled ... ok
test kill::tests::a_kill_asks_the_job_to_die_and_reports_how_it_went ... ok
test kill::tests::an_id_nobody_has_is_an_error_result ... ok
test kill::tests::killing_is_trusted_not_read_only_and_destroys_nothing_durable ... ok
test log::tests::a_broken_byte_is_one_replacement_and_the_cursor_moves_past_it ... ok
test log::tests::a_character_split_across_two_windows_waits_for_the_second ... ok
test log::tests::a_cursor_reads_on_from_where_the_last_one_stopped ... ok
test log::tests::a_log_is_the_id_under_the_bash_directory ... ok
test log::tests::a_note_is_written_even_once_the_cap_has_bitten ... ok
test log::tests::a_window_says_when_there_is_more_waiting ... ok
test log::tests::the_cap_stops_the_log_and_says_so_in_it ... ok
test log::tests::what_was_written_is_what_the_window_reads_back ... ok
test notify::tests::a_call_that_asked_for_nothing_watches_nothing ... ok
test notify::tests::a_completion_is_delivered_quietly ... ok
test notify::tests::a_completion_names_the_job_its_state_and_where_to_read_it ... ok
test notify::tests::a_condition_hit_says_the_job_is_still_going ... ok
test notify::tests::a_condition_matched_at_the_end_rides_the_completion ... ok
test notify::tests::a_duration_is_said_the_way_a_person_reads_it ... ok
test notify::tests::a_held_notice_folds_the_next_window_into_a_line_and_a_count ... ok
test notify::tests::a_pattern_is_matched_too_and_a_bad_one_is_refused ... ok
test notify::tests::a_pinned_window_needs_an_ongoing_watch ... ok
test notify::tests::a_substring_is_matched_on_the_line_that_carries_it ... ok
test notify::tests::a_tally_the_window_held_rides_the_completion_as_a_count ... ok
test notify::tests::a_window_is_tallied_as_a_count_and_the_last_line_that_answered ... ok
test notify::tests::an_ongoing_notice_says_its_count_its_span_and_its_next ... ok
test notify::tests::an_ongoing_watch_with_nothing_to_watch_for_is_refused ... ok
test notify::tests::the_first_hit_read_is_the_tally_stopped_at_its_first_match ... ok
test output::tests::a_clean_exit_is_not_an_error ... ok
test output::tests::a_command_that_wrote_nothing_keeps_the_shape ... ok
test output::tests::a_line_with_no_end_in_sight_is_clipped ... ok
test output::tests::a_non_zero_exit_is_an_error_and_says_the_code ... ok
test output::tests::a_partial_line_is_still_the_tail ... ok
test output::tests::a_person_hears_about_an_ending_only_when_there_was_no_exit_code ... ok
test output::tests::a_timeout_says_how_long_it_waited ... ok
test output::tests::an_interrupt_keeps_the_output_and_says_it_was_cut_short ... ok
test output::tests::many_small_writes_are_one_output ... ok
test output::tests::output_past_the_cap_keeps_the_head_and_the_tail ... ok
test output::tests::output_that_fits_comes_back_whole ... ok
test output::tests::seconds_read_the_way_a_person_says_them ... ok
test output::tests::the_cap_counts_characters_and_never_splits_one ... ok
test output::tests::the_marker_counts_exactly_what_was_dropped ... ok
test output::tests::the_tail_is_the_last_lines ... ok
test promote::tests::a_call_nobody_is_running_is_not_promoted ... ok
test promote::tests::a_call_that_has_ended_can_no_longer_be_promoted ... ok
test promote::tests::a_listening_call_is_promoted_by_name ... ok
test promote::tests::the_command_flips_the_call_it_names ... ok
test promote::tests::the_command_runs_during_a_turn_and_takes_the_call_id ... ok
test promote::tests::the_command_says_why_it_promoted_nothing ... ok
test promote::tests::two_calls_are_promoted_apart ... ok
test read::tests::a_cursor_reads_on_and_never_repeats_itself ... ok
test read::tests::a_finished_job_still_answers_and_says_how_it_ended ... ok
test read::tests::a_job_with_nothing_new_says_so_and_leaves_the_cursor_alone ... ok
test read::tests::a_log_that_has_gone_is_an_error_result_naming_it ... ok
test read::tests::a_prefix_is_enough_and_an_id_nobody_has_is_an_error_result ... ok
test read::tests::a_read_says_the_command_the_state_and_where_to_read_on ... ok
test read::tests::the_tool_is_free_to_call_and_bounds_its_own_answer ... ok
test reject::tests::a_refusal_offers_the_way_round_it ... ok
test reject::tests::an_unbalanced_quote_still_reaches_the_base_command ... ok
test reject::tests::every_refusal_says_why_and_names_it ... ok
test reject::tests::quoted_words_are_one_word ... ok
test reject::tests::the_interactive_table_answers_every_row ... ok
test run::tests::a_character_split_across_two_reads_is_decoded_once ... ok
test run::tests::a_command_comes_back_with_what_it_wrote ... ok
test run::tests::a_command_nobody_watches_still_runs_where_it_was_told ... ok
test run::tests::a_directory_that_is_not_there_fails_to_spawn ... ok
test run::tests::a_promoted_wait_hands_the_running_command_back_untouched ... ok
test run::tests::bytes_that_are_not_text_become_one_replacement_each ... ok
test run::tests::letting_go_of_a_run_takes_the_whole_process_group_with_it ... ok
test run::tests::multibyte_output_survives_the_pipe ... ok
test run::tests::output_past_the_cap_is_truncated_in_the_middle ... ok
test run::tests::stderr_arrives_with_stdout ... ok
test run::tests::the_command_runs_in_the_call_s_working_directory ... ok
test run::tests::the_exit_code_is_the_command_s_own ... ok
test run::tests::the_timeout_kills_the_whole_group_and_says_so ... ok
test shell::tests::a_failing_line_carries_its_exit_code_beside_its_output ... ok
test shell::tests::a_line_records_what_it_wrote ... ok
test shell::tests::a_line_the_tool_would_refuse_runs_for_the_person_who_typed_it ... ok
test shell::tests::an_empty_line_is_nothing_to_run ... ok
test shell::tests::an_ending_that_reached_no_exit_code_gets_its_own_last_line ... ok
test shell::tests::the_line_runs_in_the_session_s_directory ... ok
test shell::tests::the_spec_runs_now_and_takes_the_rest_of_the_line ... ok
test sink::tests::a_buffer_answers_the_call_that_is_waiting ... ok
test sink::tests::a_file_sink_keeps_nothing_in_memory ... ok
test sink::tests::promoting_a_sink_that_is_already_a_file_keeps_writing ... ok
test sink::tests::promotion_carries_what_the_buffer_held_into_the_log ... ok
test supervise::tests::a_burst_buys_quiet_and_the_window_ending_flushes_what_it_held ... ok
test supervise::tests::a_job_nobody_asked_about_is_never_read ... ok
test supervise::tests::a_log_that_is_not_there_is_not_a_failure ... ok
test supervise::tests::a_pinned_window_holds_a_burst_and_flushes_at_its_end ... ok
test supervise::tests::a_scan_reads_on_from_where_it_stopped_and_fires_once ... ok
test supervise::tests::a_signalled_process_has_no_code_of_its_own ... ok
test supervise::tests::a_slow_stream_wakes_on_every_line ... ok
test supervise::tests::a_wake_buys_quiet_in_proportion_to_the_lines_it_folded ... ok
test supervise::tests::the_first_hit_past_the_window_carries_the_count_and_resets_it ... ok
test supervise::tests::what_the_window_held_rides_the_end_of_the_job ... ok
test tail::tests::a_command_that_has_written_nothing_has_no_tail ... ok
test tail::tests::an_output_that_has_not_moved_is_not_sent_again ... ok
test tail::tests::only_the_last_lines_go_out ... ok
test tail::tests::the_tail_is_the_output_so_far ... ok
test tests::a_background_call_answers_at_once_with_a_job_and_its_log ... ok
test tests::a_bare_sleep_is_refused_in_the_foreground_and_a_timer_in_the_background ... ok
test tests::a_call_bounds_itself_within_the_tool_s_ceiling ... ok
test tests::a_call_nobody_promotes_leaves_no_listener_behind ... ok
test tests::a_command_that_could_never_finish_is_backgrounded_with_the_reason ... ok
test tests::a_dropped_call_answers_nothing ... ok
test tests::a_job_that_ends_wakes_the_session_and_clears_the_rail ... ok
test tests::a_job_that_only_writes_wakes_nobody ... ok
test tests::a_job_whose_session_is_gone_writes_that_into_its_log ... ok
test tests::a_line_the_call_asked_about_wakes_the_session_before_the_end ... ok
test tests::a_missing_shell_withholds_tools_and_warns_the_session ... ok
test tests::a_pattern_that_cannot_compile_is_an_error_result_and_runs_nothing ... ok
test tests::a_person_moves_a_running_command_into_the_background ... ok
test tests::a_timeout_kills_the_command_and_says_how_long_it_waited ... ok
test tests::an_interactive_command_is_refused_without_being_run ... ok
test tests::an_ordinary_command_is_still_waited_for ... ok
test tests::arguments_that_do_not_match_the_schema_are_invalid_input ... ok
test tests::explicit_background_returns_a_job_whose_completed_result_can_be_read ... ok
test tests::nothing_about_a_command_is_a_decision_only_a_person_may_take ... ok
test tests::notify_all_with_no_condition_is_an_error_result_and_runs_nothing ... ok
test tests::notify_quiet_without_notify_all_is_an_error_result_and_runs_nothing ... ok
test tests::omitted_or_false_background_returns_completed_output_and_status ... ok
test tests::omitted_or_false_background_returns_failure_output_and_status ... ok
test tests::stopping_the_plugin_ends_every_job_it_had_running ... ok
test tests::the_plugin_registers_three_verbs_and_two_commands ... ok
test tests::the_spec_advertises_the_arguments_the_real_shell_and_the_async_way ... ok
test tests::the_spec_chooses_waiting_by_dependency_not_duration ... ok
test tests::the_subject_is_the_command_a_rule_would_match ... ok
test tests::the_traits_fail_closed_except_for_trust_and_running_together ... ok

test result: ok. 147 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.93s

     Running unittests src/lib.rs (target/debug/deps/bingo_tool_fs-f3356776ee646339)

running 80 tests
test ask::tests::a_form_answered_with_the_wrong_count_fails ... ok
test ask::tests::a_host_that_cannot_ask_fails_the_call ... ok
test ask::tests::a_malformed_set_of_questions_is_refused_before_anyone_is_asked ... ok
test ask::tests::a_multi_select_question_reports_every_label_chosen ... ok
test ask::tests::a_question_becomes_an_interaction_and_its_answer_a_label ... ok
test ask::tests::a_question_left_alone_reads_as_skipped ... ok
test ask::tests::a_set_ticked_and_typed_on_reads_back_as_both ... ok
test ask::tests::an_answer_naming_no_option_fails ... ok
test ask::tests::declining_is_an_error_result_that_says_so ... ok
test ask::tests::every_question_is_asked_at_once_and_answered_by_its_header ... ok
test ask::tests::the_spec_advertises_the_argument_schema ... ok
test ask::tests::words_alone_answer_a_choice_and_blank_words_do_not ... ok
test edit::tests::a_file_that_cannot_be_read_is_not_edited ... ok
test edit::tests::a_string_that_is_not_there_is_refused ... ok
test edit::tests::a_string_that_is_not_unique_is_refused_with_its_count ... ok
test edit::tests::a_unique_string_is_replaced_and_the_result_says_how_many ... ok
test edit::tests::an_edit_that_changes_nothing_is_refused ... ok
test edit::tests::line_endings_and_the_trailing_newline_survive_the_edit ... ok
test edit::tests::replace_all_moves_every_occurrence_and_counts_them ... ok
test edit::tests::the_result_carries_the_diff_the_preview_showed ... ok
test edit::tests::the_spec_advertises_the_argument_schema ... ok
test edit::tests::the_subject_is_the_resolved_path ... ok
test glob::tests::a_broken_pattern_is_invalid_input ... ok
test glob::tests::a_result_over_the_bounds_is_truncated_and_counts_what_is_missing ... ok
test glob::tests::a_root_that_is_not_a_directory_fails_by_name ... ok
test glob::tests::gitignored_and_hidden_files_are_not_matched ... ok
test glob::tests::matches_are_absolute_paths_newest_first ... ok
test glob::tests::nothing_matching_says_so_rather_than_returning_nothing ... ok
test glob::tests::the_path_argument_moves_the_root_and_the_pattern_with_it ... ok
test glob::tests::the_spec_advertises_the_argument_schema ... ok
test glob::tests::the_subject_is_the_search_root ... ok
test grep::tests::a_broken_pattern_or_type_is_invalid_input ... ok
test grep::tests::a_missing_root_fails_by_name ... ok
test grep::tests::a_single_file_can_be_the_search_root ... ok
test grep::tests::case_insensitive_matching_is_opt_in ... ok
test grep::tests::content_mode_numbers_its_lines_only_when_asked ... ok
test grep::tests::context_lines_come_back_with_a_dash_separator ... ok
test grep::tests::count_mode_gives_one_number_per_file ... ok
test grep::tests::gitignored_hidden_and_binary_files_are_skipped ... ok
test grep::tests::head_limit_caps_the_result ... ok
test grep::tests::multiline_lets_a_pattern_cross_a_line_break ... ok
test grep::tests::the_default_mode_names_the_files_that_matched ... ok
test grep::tests::the_glob_and_type_filters_narrow_the_files_searched ... ok
test grep::tests::the_spec_advertises_the_argument_schema ... ok
test grep::tests::the_subject_is_the_search_root ... ok
test output::tests::everything_fits_when_it_fits ... ok
test output::tests::the_character_cap_counts_what_it_dropped ... ok
test output::tests::the_entry_cap_counts_what_it_dropped ... ok
test path::tests::a_broken_pattern_is_an_error_not_a_panic ... ok
test path::tests::a_pattern_with_a_slash_anchors_at_the_root ... ok
test path::tests::a_pattern_without_a_slash_matches_at_any_depth ... ok
test path::tests::a_relative_path_resolves_against_the_working_directory ... ok
test read::tests::a_binary_file_that_is_not_an_image_fails_as_invalid_utf8 ... ok
test read::tests::a_bounded_picture_says_what_the_file_is_and_how_to_ask_for_it ... ok
test read::tests::a_directory_is_not_a_file ... ok
test read::tests::a_file_named_a_picture_that_is_not_one_says_so ... ok
test read::tests::a_file_over_the_file_cap_fails_by_size ... ok
test read::tests::a_long_file_is_truncated_and_says_how_many_lines_are_missing ... ok
test read::tests::a_missing_file_fails_by_name ... ok
test read::tests::a_photograph_over_the_wire_cap_is_read_and_bounded ... ok
test read::tests::a_relative_path_resolves_against_the_working_directory ... ok
test read::tests::a_text_file_comes_back_numbered_like_cat_n ... ok
test read::tests::an_image_comes_back_as_an_image_part ... ok
test read::tests::arguments_that_do_not_match_the_schema_are_invalid_input ... ok
test read::tests::offset_and_limit_cut_a_window_keeping_the_real_line_numbers ... ok
test read::tests::original_above_the_journals_cap_is_refused_by_size ... ok
test read::tests::original_answers_with_the_file_as_it_is ... ok
test read::tests::original_on_a_text_file_reads_the_text ... ok
test read::tests::the_description_names_every_extension_a_picture_may_arrive_under ... ok
test read::tests::the_description_names_the_flag_and_the_caps_it_is_bounded_by ... ok
test read::tests::the_spec_advertises_the_argument_schema ... ok
test read::tests::the_subject_is_the_resolved_path ... ok
test tests::the_plugin_registers_every_tool_its_manifest_promises ... ok
test write::tests::a_directory_in_the_way_is_a_refusal_not_a_write ... ok
test write::tests::a_file_that_cannot_be_read_is_not_overwritten ... ok
test write::tests::a_new_file_brings_its_directories_and_has_nothing_to_diff ... ok
test write::tests::arguments_that_do_not_match_the_schema_are_invalid_input ... ok
test write::tests::overwriting_carries_the_diff_the_preview_showed ... ok
test write::tests::the_spec_advertises_the_argument_schema ... ok
test write::tests::the_subject_is_the_resolved_path ... ok

test result: ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.75s

     Running unittests src/lib.rs (target/debug/deps/bingo_tool_web-d7b9cf277ef37661)

running 92 tests
test approved::tests::a_host_entry_matches_any_path_under_it ... ok
test approved::tests::a_host_entry_matches_its_subdomains ... ok
test approved::tests::a_host_that_is_not_on_the_list_is_not_documentation ... ok
test approved::tests::a_path_prefix_entry_matches_on_segment_boundaries_only ... ok
test body::tests::a_body_of_no_stated_type_is_read_as_text ... ok
test body::tests::a_picture_comes_back_as_the_picture ... ok
test body::tests::anything_else_is_refused_by_name ... ok
test body::tests::html_reaches_the_model_as_markdown ... ok
test body::tests::text_and_json_come_back_as_they_are ... ok
test body::tests::what_is_served_as_a_picture_is_read_as_one_only_if_it_is_one ... ok
test brave::tests::a_result_without_a_description_keeps_its_title_and_url ... ok
test brave::tests::an_answer_that_is_not_the_documented_shape_says_so ... ok
test brave::tests::an_answer_with_no_web_section_holds_no_hits ... ok
test brave::tests::every_web_result_becomes_a_hit ... ok
test brave::tests::the_key_never_reaches_a_debug_line ... ok
test cache::tests::a_page_older_than_the_ttl_is_a_miss ... ok
test cache::tests::a_page_stored_within_the_ttl_comes_back ... ok
test cache::tests::a_page_that_was_never_stored_is_a_miss ... ok
test cache::tests::a_second_store_replaces_the_first ... ok
test cache::tests::the_oldest_page_goes_when_the_total_would_pass_the_budget ... ok
test canonical::tests::a_host_without_a_dot_is_refused ... ok
test canonical::tests::a_loopback_address_keeps_the_scheme_it_was_given ... ok
test canonical::tests::a_url_longer_than_the_cap_is_refused_before_it_is_parsed ... ok
test canonical::tests::credentials_in_a_url_are_refused ... ok
test canonical::tests::http_is_upgraded_to_https ... ok
test canonical::tests::only_http_and_https_are_fetched ... ok
test canonical::tests::the_canonical_form_is_what_the_cache_and_the_gate_both_see ... ok
test canonical::tests::what_is_not_a_url_says_so ... ok
test duckduckgo::tests::a_challenge_page_is_a_failure_and_not_an_empty_result ... ok
test duckduckgo::tests::a_link_that_is_not_a_redirect_is_left_alone ... ok
test duckduckgo::tests::a_page_with_no_results_and_no_challenge_is_empty ... ok
test duckduckgo::tests::every_result_block_on_the_page_becomes_a_hit ... ok
test duckduckgo::tests::the_redirect_wrapper_is_unwrapped_to_the_page_it_points_at ... ok
test duckduckgo::tests::titles_and_snippets_arrive_as_text ... ok
test fetch::tests::a_body_over_the_cap_fails_and_nothing_is_kept ... ok
test fetch::tests::a_body_served_as_a_picture_that_is_not_one_is_refused ... ok
test fetch::tests::a_content_type_that_is_not_text_is_refused_by_name ... ok
test fetch::tests::a_fetch_is_trusted_cancellable_and_safe_beside_other_calls ... ok
test fetch::tests::a_page_comes_back_as_markdown ... ok
test fetch::tests::a_picture_comes_back_as_the_picture_and_is_never_cached ... ok
test fetch::tests::a_second_fetch_within_the_ttl_never_reaches_the_server ... ok
test fetch::tests::a_status_that_is_not_a_success_names_the_status ... ok
test fetch::tests::a_url_that_does_not_parse_names_no_subject ... ok
test fetch::tests::a_url_that_fails_validation_never_reaches_the_network ... ok
test fetch::tests::arguments_that_do_not_match_the_schema_are_invalid_input ... ok
test fetch::tests::only_the_documentation_list_is_claimed_read_only ... ok
test fetch::tests::the_request_asks_for_pictures_after_the_documents ... ok
test fetch::tests::the_spec_advertises_one_url_argument ... ok
test fetch::tests::the_subject_and_the_preview_are_the_canonical_url ... ok
test hits::tests::a_block_list_drops_the_domain_and_its_subdomains ... ok
test hits::tests::a_hit_with_no_snippet_is_one_line ... ok
test hits::tests::an_allow_list_keeps_the_domain_and_its_subdomains_only ... ok
test hits::tests::an_allow_list_settles_it_when_both_are_given ... ok
test hits::tests::no_more_than_eight_results_are_returned ... ok
test hits::tests::nothing_found_says_so ... ok
test hits::tests::results_are_a_numbered_list_of_title_url_and_snippet ... ok
test hits::tests::with_no_lists_everything_is_kept ... ok
test html_text::tests::an_ampersand_that_is_not_an_entity_survives ... ok
test html_text::tests::named_entities_are_resolved ... ok
test html_text::tests::numeric_entities_are_resolved_in_both_bases ... ok
test html_text::tests::tags_go_and_whitespace_collapses ... ok
test output::tests::the_cut_falls_on_a_character_boundary_not_a_byte_one ... ok
test output::tests::what_does_not_fit_is_cut_and_counted ... ok
test output::tests::what_fits_is_returned_unchanged ... ok
test page::tests::a_machine_with_no_browser_fails_at_once_with_the_url ... ok
test page::tests::a_page_holds_the_turn_and_is_never_waved_through ... ok
test page::tests::a_page_nobody_answers_times_out_naming_the_url ... ok
test page::tests::a_page_the_user_dismisses_answers_nothing_and_says_so ... ok
test page::tests::a_wait_the_model_names_is_bounded_at_both_ends ... ok
test page::tests::an_interrupted_turn_drops_the_page ... ok
test page::tests::arguments_that_do_not_match_the_schema_are_invalid_input ... ok
test page::tests::the_description_tells_the_model_how_a_page_answers ... ok
test page::tests::the_spec_advertises_a_title_a_page_and_a_bounded_wait ... ok
test page::tests::what_the_page_submits_is_the_result ... ok
test readable::tests::a_document_with_no_article_in_it_is_still_converted ... ok
test readable::tests::headings_and_links_survive_the_conversion ... ok
test readable::tests::the_navigation_and_the_scripts_do_not ... ok
test search::tests::a_backend_that_does_not_answer_is_given_twenty_seconds ... ok
test search::tests::a_query_too_short_to_mean_anything_is_invalid_input ... ok
test search::tests::a_search_reads_and_changes_nothing ... ok
test search::tests::finding_nothing_is_an_answer_and_not_an_error ... ok
test search::tests::results_reach_the_model_as_a_numbered_list ... ok
test search::tests::the_domain_lists_are_honoured ... ok
test search::tests::the_spec_advertises_the_query_and_the_two_domain_lists ... ok
test search::tests::what_the_backend_failed_at_is_what_the_model_is_told ... ok
test tests::a_key_that_is_not_claimed_is_a_startup_failure ... ok
test tests::an_empty_slice_searches_without_a_key ... ok
test tests::brave_with_a_key_is_the_backend_that_answers ... ok
test tests::brave_without_a_key_says_where_to_put_one ... ok
test tests::the_backend_is_named_in_the_settings_as_the_model_of_it_is_written ... ok
test tests::the_environment_beats_the_settings_and_blank_counts_as_absent ... ok
test tests::the_plugin_registers_every_tool_its_manifest_promises ... ok

test result: ok. 92 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.18s

     Running unittests src/lib.rs (target/debug/deps/bingo_update-4371c155b5fcd72f)

running 47 tests
test api::tests::a_build_names_itself_and_the_way_round_names_a_tag ... ok
test api::tests::the_address_is_the_repositorys_own_latest_release ... ok
test asset::tests::the_binary_inside_carries_the_platforms_own_suffix ... ok
test asset::tests::the_four_names_are_the_ones_the_release_publishes ... ok
test asset::tests::this_build_is_one_of_them_or_none_of_them ... ok
test check::tests::a_data_directory_that_cannot_be_written_costs_the_answer_nothing ... ok
test check::tests::a_day_later_it_asks_again ... ok
test check::tests::a_failed_ask_leaves_what_was_heard_before_where_it_was ... ok
test check::tests::a_failed_ask_says_nothing_new_and_still_waits_a_day ... ok
test check::tests::a_first_run_asks_and_says_what_it_was_told ... ok
test check::tests::a_stamp_younger_than_a_day_is_answered_without_asking ... ok
test check::tests::an_answer_that_is_not_a_release_is_silence ... ok
test check::tests::the_newest_release_being_this_build_says_nothing ... ok
test checksums::tests::a_binary_mark_and_a_single_space_read_the_same ... ok
test checksums::tests::a_line_that_is_not_a_digest_and_a_name_is_skipped ... ok
test checksums::tests::a_name_the_list_does_not_carry_has_no_digest ... ok
test checksums::tests::every_asset_in_the_list_has_its_own_digest ... ok
test checksums::tests::the_digest_the_list_carries_is_the_one_the_bytes_hash_to ... ok
test install::tests::a_binary_left_behind_by_an_earlier_update_is_swept ... ok
test install::tests::a_directory_nobody_may_write_says_so_and_names_the_way_round ... ok
test install::tests::a_target_that_is_not_there_fails_and_leaves_the_staged_one_alone ... ok
test install::tests::a_zip_is_not_unpacked_as_a_gzip ... ok
test install::tests::an_archive_that_is_not_one_fails_and_says_what_tar_said ... ok
test install::tests::the_archive_gives_up_the_binary_it_holds ... ok
test install::tests::the_two_renames_replace_the_target_and_leave_nothing_beside_it ... ok
test install::tests::what_is_staged_may_be_run ... ok
test release::tests::a_release_with_no_assets_still_reads ... ok
test release::tests::an_answer_that_is_not_a_release_is_an_error_and_not_a_panic ... ok
test release::tests::an_asset_the_release_does_not_carry_is_none ... ok
test release::tests::every_archive_the_release_matrix_uploads_is_an_asset ... ok
test release::tests::the_tag_is_the_version_without_its_v ... ok
test settings::tests::a_check_is_on_until_it_is_turned_off ... ok
test settings::tests::a_neighbouring_key_is_not_this_ones_business ... ok
test settings::tests::a_slice_that_is_not_this_shape_leaves_the_check_where_it_was ... ok
test settings::tests::an_unknown_key_inside_the_slice_is_not_a_setting ... ok
test stamp::tests::a_machine_that_has_never_asked_is_due ... ok
test stamp::tests::a_stamp_from_the_future_is_due_rather_than_forever ... ok
test stamp::tests::a_stamp_that_is_not_a_stamp_reads_as_none ... ok
test stamp::tests::a_stamp_written_before_the_answer_carries_no_version ... ok
test stamp::tests::a_stamp_younger_than_the_interval_is_not_due_and_one_older_is ... ok
test stamp::tests::what_was_written_is_what_is_read_back ... ok
test version::tests::a_higher_number_anywhere_is_newer ... ok
test version::tests::a_pre_release_is_behind_the_release_of_its_own_numbers ... ok
test version::tests::build_metadata_takes_no_part ... ok
test version::tests::nothing_unreadable_is_ever_newer ... ok
test version::tests::the_same_version_and_an_older_one_are_not_newer ... ok
test version::tests::the_tags_leading_v_is_read_on_either_side ... ok

test result: ok. 47 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

   Doc-tests bingo_agents

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_alarm

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_auth_oauth

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_channel_adapter

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_channel_feishu
mbx[progress]: 300s elapsed; 0 hits, 0 misses, 5 bypassed, 0 not looked up; ~0.0s compiler work saved

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_channels

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_checkpoints

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_claim

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_collab

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_context

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_core

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_demo_ui

running 1 test
test crates/bingo-demo-ui/src/lib.rs - DesignDoc (line 426) ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

all doctests ran in 0.36s; merged doctests compilation took 0.36s
   Doc-tests bingo_digest

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_experience

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_gateway

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_hooks_shell

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_http

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_jsonrpc

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_loopback

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_mcp

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_permissions

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_persona

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_pictures

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_plugin_rpc

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_process

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_provider_acp

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_provider_anthropic

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_provider_fake

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_provider_openai

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_rooms

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_schedule

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_schema_ts

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_sdk

running 3 tests
test crates/bingo-sdk/src/event/record.rs - event::record::UnknownRecord (line 13) - compile fail ... ok
test crates/bingo-sdk/src/intercept.rs - intercept::Next (line 72) - compile fail ... ok
test crates/bingo-sdk/src/service.rs - service::ServiceKey (line 29) - compile fail ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s

   Doc-tests bingo_services

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_shelf

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_skills

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_store_jsonl

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_surface_print

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_surface_rpc

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_surface_tui

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_tasks

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_tool_bash

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_tool_fs

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_tool_web

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests bingo_update

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

mbx[cache]: 0 hits, 0 misses, 24 incremental, 45 bypassed; 0 B downloaded, 0 B uploaded, 0 B stored locally
mbx[savings]: your deleted worktrees left 1.0 TiB logical behind. left. past tense.
```

</details>

<details>
<summary>acp-alone — exit 0</summary>

```text
$ cargo test -p bingo --test cli acp::bridge::a_call_with_no_turn_in_flight_is_refused_with_a_reason --locked -- --exact --nocapture
      ┌─────────┐  mr boxington
     ╱ ═══════ ╱│
    ╱─────────╱ │
    │ •    (◉)│≡│
    │  ◟▄▄▄◞  │ ╱
    │   ◀◆▶   │╱
    └─────────┘
   Compiling bingo-core v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-core)
   Compiling bingo-gateway v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-gateway)
   Compiling bingo v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 5.72s
     Running tests/cli/main.rs (target/debug/deps/cli-872aca5a28330dac)

running 1 test
test acp::bridge::a_call_with_no_turn_in_flight_is_refused_with_a_reason ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 252 filtered out; finished in 0.39s

mbx[cache]: 0 hits, 0 misses, 4 incremental; 0 B downloaded, 0 B uploaded, 0 B stored locally
mbx[savings]: 1.0 TiB logical of outputs had survived their checkouts. an unnatural state, now corrected.
```

</details>

<details>
<summary>discipline — exit 0</summary>

```text
$ scripts/check_discipline.sh
dependency direction ok
warn crates/bingo-plugin-rpc/examples/stub_plugin/main.rs: 709 non-test lines (>700)
warn crates/bingo-plugin-rpc/src/wire.rs: 860 non-test lines (>700)
warn crates/bingo-channel-feishu/src/feishu/tests.rs: 894 non-test lines (>700)
warn crates/bingo-surface-tui/src/ui.rs: 731 non-test lines (>700)
warn crates/bingo-surface-tui/src/transcript.rs: 949 non-test lines (>700)
warn crates/bingo-surface-tui/src/test_support.rs: 957 non-test lines (>700)
warn crates/bingo-surface-tui/src/run.rs: 982 non-test lines (>700)
warn crates/bingo-surface-tui/src/screens.rs: 1000 non-test lines (>700)
warn crates/bingo-surface-tui/src/input.rs: 898 non-test lines (>700)
warn crates/bingo-surface-tui/src/view.rs: 804 non-test lines (>700)
warn crates/bingo-surface-tui/src/form.rs: 846 non-test lines (>700)
warn crates/bingo-surface-tui/src/motion.rs: 941 non-test lines (>700)
warn crates/bingo-mcp/tests/server.rs: 771 non-test lines (>700)
warn crates/bingo-provider-fake/src/lib.rs: 703 non-test lines (>700)
warn crates/bingo-agents/src/tests.rs: 758 non-test lines (>700)
warn crates/bingo-channels/src/host/tests.rs: 947 non-test lines (>700)
warn crates/bingo-channels/src/deliver/tests.rs: 798 non-test lines (>700)
warn crates/bingo-channels/src/runner.rs: 707 non-test lines (>700)
warn crates/bingo-core/src/session.rs: 934 non-test lines (>700)
warn crates/bingo-core/src/host.rs: 995 non-test lines (>700)
warn crates/bingo-core/src/host/tests/resume.rs: 996 non-test lines (>700)
warn crates/bingo-core/src/host/tests.rs: 736 non-test lines (>700)
warn crates/bingo-core/src/turn.rs: 983 non-test lines (>700)
warn crates/bingo-core/src/test_support.rs: 740 non-test lines (>700)
warn crates/bingo-core/src/turn/tests.rs: 984 non-test lines (>700)
warn crates/bingo-core/src/session/tests.rs: 704 non-test lines (>700)
warn crates/bingo-surface-rpc/tests/wire/bounded.rs: 902 non-test lines (>700)
warn crates/bingo-surface-rpc/tests/wire/main.rs: 811 non-test lines (>700)
warn crates/bingo-surface-rpc/src/server.rs: 738 non-test lines (>700)
warn crates/bingo/tests/pty/main.rs: 965 non-test lines (>700)
warn crates/bingo/tests/cli/agents.rs: 762 non-test lines (>700)
warn crates/bingo/tests/cli/rooms.rs: 978 non-test lines (>700)
warn crates/bingo/tests/cli/gateway.rs: 744 non-test lines (>700)
warn crates/bingo/tests/cli/main.rs: 831 non-test lines (>700)
warn crates/bingo/tests/cli/acp.rs: 818 non-test lines (>700)
warn crates/bingo/tests/channels.rs: 742 non-test lines (>700)
warn crates/bingo/tests/rpc.rs: 991 non-test lines (>700)
warn crates/bingo/src/main.rs: 963 non-test lines (>700)
warn crates/bingo-provider-acp/src/session.rs: 755 non-test lines (>700)
kernel names no tool
shared service declarations ok
TUI names no plugin or tool
cohesion ok
warn crates/bingo-context/src/compact.rs:24 fn compact is 61 lines (>60)
warn crates/bingo-core/src/chain/mod.rs:165 fn invoke is 75 lines (>60)
warn crates/bingo-core/src/host/registry.rs:142 fn add is 76 lines (>60)
warn crates/bingo-core/src/session.rs:137 fn dispatch is 98 lines (>60)
warn crates/bingo-core/src/turn/stream.rs:25 fn stream is 76 lines (>60)
warn crates/bingo-core/src/turn.rs:164 fn run_turn is 63 lines (>60)
warn crates/bingo-sdk/src/testing/calls.rs:85 fn fmt is 109 lines (>60)
warn crates/bingo-surface-print/src/render.rs:161 fn text is 61 lines (>60)
warn crates/bingo-surface-tui/src/test_lanes.rs:227 fn task_payload is 74 lines (>60)
warn docs/plans/M136-the-event-catalogue.md: 166 lines (>150)
warn docs/plans/M26-the-bridge-learns-context-and-compaction.md: 152 lines (>150)
warn docs/plans/M27-the-bridge-learns-providers.md: 201 lines (>150)
warn docs/plans/M28-wire-services.md: 225 lines (>150)
warn docs/plans/M29-hooks-cross-the-bridge.md: 230 lines (>150)
warn docs/plans/M30-the-allowance.md: 230 lines (>150)
warn docs/plans/M31-stored-children-in-sight.md: 192 lines (>150)
warn docs/plans/M32-the-picker-says-what-a-session-is.md: 203 lines (>150)
warn docs/plans/M34-the-surface-answers-the-person.md: 179 lines (>150)
warn docs/plans/M35-the-endpoint-says-what-it-serves.md: 185 lines (>150)
warn docs/plans/M36-the-roster-stands-up.md: 168 lines (>150)
warn docs/plans/M37-the-room-is-read.md: 171 lines (>150)
warn docs/plans/M38-an-agent-answers-as-a-model.md: 227 lines (>150)
warn docs/plans/M39-shared-tools-cross-as-mcp.md: 255 lines (>150)
warn docs/plans/M43-a-question-reaches-the-person.md: 169 lines (>150)
warn docs/plans/M45-a-picture-beside-the-words.md: 232 lines (>150)
warn docs/plans/M46-the-picture-is-seen.md: 280 lines (>150)
warn docs/plans/M47-the-picture-from-anywhere.md: 235 lines (>150)
warn docs/plans/M48-the-draft-shows-its-pictures.md: 325 lines (>150)
warn docs/plans/M49-the-picture-through-tmux.md: 288 lines (>150)
warn docs/plans/M50-the-model-knows-the-picture-is-seen.md: 230 lines (>150)
warn docs/plans/M51-the-picture-in-the-words.md: 315 lines (>150)
warn docs/plans/M52-the-beat-on-the-persons-side.md: 363 lines (>150)
warn docs/plans/M53-the-questions-asked-together.md: 300 lines (>150)
warn docs/plans/M54-the-page-that-answers-back.md: 219 lines (>150)
warn docs/plans/M55-the-list-that-is-searched.md: 250 lines (>150)
warn docs/plans/M56-the-picture-where-the-eye-is.md: 225 lines (>150)
warn docs/plans/M57-the-form-card-compared.md: 275 lines (>150)
warn docs/plans/M58-the-list-reads-the-line.md: 279 lines (>150)
warn docs/plans/M59-the-form-holds-still.md: 240 lines (>150)
warn docs/plans/M60-the-late-answer.md: 326 lines (>150)
warn docs/plans/M61-the-picture-that-was-already-here.md: 331 lines (>150)
warn docs/plans/M62-the-band-above-the-words.md: 211 lines (>150)
warn docs/plans/M63-the-binary-that-replaces-itself.md: 196 lines (>150)
warn docs/plans/M64-one-fact-one-file.md: 154 lines (>150)
warn docs/plans/M65-the-bang-line.md: 191 lines (>150)
warn docs/plans/M67-the-turn-that-can-be-undone.md: 164 lines (>150)
warn docs/plans/M68-the-line-that-waits.md: 174 lines (>150)
warn docs/plans/M69-the-opening-shot.md: 232 lines (>150)
warn docs/plans/M70-the-box-that-opens.md: 277 lines (>150)
warn docs/plans/M71-the-look-that-follows.md: 232 lines (>150)
warn docs/plans/M72-the-line-that-draws-itself.md: 288 lines (>150)
warn docs/plans/M73-the-ink-the-terminal-owns.md: 203 lines (>150)
warn docs/plans/M84-worked-for-recap-and-memory-rows.md: 191 lines (>150)
warn docs/plans/M85-mcp-servers-that-need-signing-in.md: 304 lines (>150)
warn docs/plans/M86-the-chat-carries-more-than-words.md: 158 lines (>150)
warn docs/plans/M91-the-context-an-agent-holds.md: 157 lines (>150)
warn docs/plans/M97-a-plugin-is-switched.md: 220 lines (>150)
warn docs/plans/M98-the-settings-file-keeps-its-comments.md: 199 lines (>150)
warn docs/plans/M99-the-agent-has-a-view-of-its-own.md: 158 lines (>150)
warn crates/bingo-provider-acp/tests/harness/prompt_host.rs:19: use bingo_sdk::testing::TestHost or explain the retained host
test host and wall-clock advisory checks complete
catalogue entry declarations ok
discipline ok
```

</details>

<details>
<summary>budget — exit 0</summary>

```text
$ scripts/budget.sh
dependencies (unique, normal): 355 (max  355)
warm cargo check -p bingo-core: 0s (max  20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
target/debug: 7 GB (soft max  5)
warn: target/debug exceeds the soft limit
test binaries: 108
budget ok
```

</details>

<details>
<summary>deny — exit 0</summary>

```text
$ cargo deny check
warning[duplicate]: found 2 duplicate entries for crate 'base64'
   ┌─ /Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/Cargo.lock:21:1
   │
21 │ ╭ base64 0.22.1 registry+https://github.com/rust-lang/crates.io-index
22 │ │ base64 0.23.1 registry+https://github.com/rust-lang/crates.io-index
   │ ╰───────────────────────────────────────────────────────────────────┘ lock entries
   │
   ├ base64 v0.22.1
     ├── hyper-util v0.1.20
     │   ├── hyper-rustls v0.27.9
     │   │   └── reqwest v0.13.4
     │   │       ├── bingo v0.6.6
     │   │       ├── bingo-auth-oauth v0.6.6
     │   │       │   ├── bingo v0.6.6 (*)
     │   │       │   ├── bingo-channel-adapter v0.6.6
     │   │       │   │   ├── bingo v0.6.6 (*)
     │   │       │   │   ├── bingo-channel-feishu v0.6.6
     │   │       │   │   │   ├── bingo v0.6.6 (*)
     │   │       │   │   │   └── (dev) bingo-gateway v0.6.6
     │   │       │   │   │       └── bingo v0.6.6 (*)
     │   │       │   │   ├── bingo-channels v0.6.6
     │   │       │   │   │   ├── bingo v0.6.6 (*)
     │   │       │   │   │   └── bingo-gateway v0.6.6 (*)
     │   │       │   │   └── bingo-gateway v0.6.6 (*)
     │   │       │   ├── bingo-mcp v0.6.6
     │   │       │   │   └── bingo v0.6.6 (*)
     │   │       │   ├── bingo-provider-anthropic v0.6.6
     │   │       │   │   └── bingo v0.6.6 (*)
     │   │       │   └── bingo-provider-openai v0.6.6
     │   │       │       └── bingo v0.6.6 (*)
     │   │       ├── bingo-channel-feishu v0.6.6 (*)
     │   │       ├── bingo-http v0.6.6
     │   │       │   ├── bingo v0.6.6 (*)
     │   │       │   ├── (dev) bingo-auth-oauth v0.6.6 (*)
     │   │       │   ├── bingo-channel-feishu v0.6.6 (*)
     │   │       │   ├── bingo-mcp v0.6.6 (*)
     │   │       │   ├── bingo-pictures v0.6.6
     │   │       │   │   ├── (dev) bingo v0.6.6 (*)
     │   │       │   │   ├── bingo-channel-feishu v0.6.6 (*)
     │   │       │   │   ├── bingo-mcp v0.6.6 (*)
     │   │       │   │   ├── bingo-plugin-rpc v0.6.6
     │   │       │   │   │   └── bingo v0.6.6 (*)
     │   │       │   │   ├── bingo-surface-print v0.6.6
     │   │       │   │   │   └── bingo v0.6.6 (*)
     │   │       │   │   ├── bingo-surface-tui v0.6.6
     │   │       │   │   │   └── bingo v0.6.6 (*)
     │   │       │   │   ├── bingo-tool-fs v0.6.6
     │   │       │   │   │   └── bingo v0.6.6 (*)
     │   │       │   │   └── bingo-tool-web v0.6.6
     │   │       │   │       └── bingo v0.6.6 (*)
     │   │       │   ├── bingo-provider-anthropic v0.6.6 (*)
     │   │       │   ├── bingo-provider-openai v0.6.6 (*)
     │   │       │   ├── bingo-surface-tui v0.6.6 (*)
     │   │       │   └── bingo-tool-web v0.6.6 (*)
     │   │       ├── bingo-mcp v0.6.6 (*)
     │   │       ├── bingo-pictures v0.6.6 (*)
     │   │       ├── bingo-provider-anthropic v0.6.6 (*)
     │   │       ├── bingo-provider-openai v0.6.6 (*)
     │   │       ├── bingo-surface-tui v0.6.6 (*)
     │   │       ├── bingo-tool-web v0.6.6 (*)
     │   │       └── rmcp v3.1.4
     │   │           ├── (dev) bingo v0.6.6 (*)
     │   │           ├── bingo-mcp v0.6.6 (*)
     │   │           └── bingo-provider-acp v0.6.6
     │   │               └── bingo v0.6.6 (*)
     │   ├── reqwest v0.13.4 (*)
     │   └── wiremock v0.6.5
     │       ├── (dev) bingo v0.6.6 (*)
     │       ├── (dev) bingo-auth-oauth v0.6.6 (*)
     │       ├── (dev) bingo-channel-feishu v0.6.6 (*)
     │       ├── (dev) bingo-mcp v0.6.6 (*)
     │       ├── (dev) bingo-pictures v0.6.6 (*)
     │       ├── (dev) bingo-provider-anthropic v0.6.6 (*)
     │       ├── (dev) bingo-provider-openai v0.6.6 (*)
     │       ├── (dev) bingo-surface-tui v0.6.6 (*)
     │       └── (dev) bingo-tool-web v0.6.6 (*)
     ├── reqwest v0.13.4 (*)
     └── wiremock v0.6.5 (*)
   ├ base64 v0.23.1
     ├── (dev) bingo v0.6.6
     ├── bingo-auth-oauth v0.6.6
     │   ├── bingo v0.6.6 (*)
     │   ├── bingo-channel-adapter v0.6.6
     │   │   ├── bingo v0.6.6 (*)
     │   │   ├── bingo-channel-feishu v0.6.6
     │   │   │   ├── bingo v0.6.6 (*)
     │   │   │   └── (dev) bingo-gateway v0.6.6
     │   │   │       └── bingo v0.6.6 (*)
     │   │   ├── bingo-channels v0.6.6
     │   │   │   ├── bingo v0.6.6 (*)
     │   │   │   └── bingo-gateway v0.6.6 (*)
     │   │   └── bingo-gateway v0.6.6 (*)
     │   ├── bingo-mcp v0.6.6
     │   │   └── bingo v0.6.6 (*)
     │   ├── bingo-provider-anthropic v0.6.6
     │   │   └── bingo v0.6.6 (*)
     │   └── bingo-provider-openai v0.6.6
     │       └── bingo v0.6.6 (*)
     ├── bingo-channel-adapter v0.6.6 (*)
     ├── bingo-channel-feishu v0.6.6 (*)
     ├── bingo-loopback v0.6.6
     │   ├── bingo-auth-oauth v0.6.6 (*)
     │   ├── bingo-provider-openai v0.6.6 (*)
     │   ├── bingo-surface-tui v0.6.6
     │   │   └── bingo v0.6.6 (*)
     │   └── bingo-tool-web v0.6.6
     │       └── bingo v0.6.6 (*)
     ├── bingo-pictures v0.6.6
     │   ├── (dev) bingo v0.6.6 (*)
     │   ├── bingo-channel-feishu v0.6.6 (*)
     │   ├── bingo-mcp v0.6.6 (*)
     │   ├── bingo-plugin-rpc v0.6.6
     │   │   └── bingo v0.6.6 (*)
     │   ├── bingo-surface-print v0.6.6
     │   │   └── bingo v0.6.6 (*)
     │   ├── bingo-surface-tui v0.6.6 (*)
     │   ├── bingo-tool-fs v0.6.6
     │   │   └── bingo v0.6.6 (*)
     │   └── bingo-tool-web v0.6.6 (*)
     ├── bingo-provider-acp v0.6.6
     │   └── bingo v0.6.6 (*)
     ├── bingo-provider-openai v0.6.6 (*)
     ├── bingo-sdk v0.6.6
     │   ├── bingo v0.6.6 (*)
     │   ├── bingo-agents v0.6.6
     │   │   └── bingo v0.6.6 (*)
     │   ├── bingo-alarm v0.6.6
     │   │   ├── bingo-agents v0.6.6 (*)
     │   │   ├── bingo-rooms v0.6.6
     │   │   │   └── bingo v0.6.6 (*)
     │   │   └── bingo-schedule v0.6.6
     │   │       ├── bingo v0.6.6 (*)
     │   │       └── bingo-gateway v0.6.6 (*)
     │   ├── bingo-auth-oauth v0.6.6 (*)
     │   ├── bingo-channel-adapter v0.6.6 (*)
     │   ├── bingo-channel-feishu v0.6.6 (*)
     │   ├── bingo-channels v0.6.6 (*)
     │   ├── bingo-checkpoints v0.6.6
     │   │   └── bingo v0.6.6 (*)
     │   ├── bingo-collab v0.6.6
     │   │   ├── (dev) bingo v0.6.6 (*)
     │   │   ├── bingo-agents v0.6.6 (*)
     │   │   ├── bingo-rooms v0.6.6 (*)
     │   │   └── bingo-surface-tui v0.6.6 (*)
     │   ├── bingo-context v0.6.6
     │   │   └── bingo v0.6.6 (*)
     │   ├── bingo-core v0.6.6
     │   │   ├── bingo v0.6.6 (*)
     │   │   └── bingo-gateway v0.6.6 (*)
     │   ├── bingo-demo-ui v0.6.6
     │   │   └── bingo v0.6.6 (*)
     │   ├── bingo-experience v0.6.6
     │   │   └── bingo v0.6.6 (*)
     │   ├── bingo-gateway v0.6.6 (*)
     │   ├── bingo-hooks-shell v0.6.6
     │   │   └── bingo v0.6.6 (*)
     │   ├── bingo-http v0.6.6
     │   │   ├── bingo v0.6.6 (*)
     │   │   ├── (dev) bingo-auth-oauth v0.6.6 (*)
     │   │   ├── bingo-channel-feishu v0.6.6 (*)
     │   │   ├── bingo-mcp v0.6.6 (*)
     │   │   ├── bingo-pictures v0.6.6 (*)
     │   │   ├── bingo-provider-anthropic v0.6.6 (*)
     │   │   ├── bingo-provider-openai v0.6.6 (*)
     │   │   ├── bingo-surface-tui v0.6.6 (*)
     │   │   └── bingo-tool-web v0.6.6 (*)
     │   ├── bingo-mcp v0.6.6 (*)
     │   ├── bingo-permissions v0.6.6
     │   │   └── bingo v0.6.6 (*)
     │   ├── bingo-persona v0.6.6
     │   │   └── bingo v0.6.6 (*)
     │   ├── bingo-pictures v0.6.6 (*)
     │   ├── bingo-plugin-rpc v0.6.6 (*)
     │   ├── bingo-provider-acp v0.6.6 (*)
     │   ├── bingo-provider-anthropic v0.6.6 (*)
     │   ├── bingo-provider-fake v0.6.6
     │   │   └── bingo v0.6.6 (*)
     │   ├── bingo-provider-openai v0.6.6 (*)
     │   ├── bingo-rooms v0.6.6 (*)
     │   ├── bingo-schedule v0.6.6 (*)
     │   ├── bingo-services v0.6.6
     │   │   ├── (dev) bingo v0.6.6 (*)
     │   │   ├── bingo-agents v0.6.6 (*)
     │   │   ├── bingo-collab v0.6.6 (*)
     │   │   ├── bingo-mcp v0.6.6 (*)
     │   │   ├── bingo-provider-acp v0.6.6 (*)
     │   │   └── bingo-rooms v0.6.6 (*)
     │   ├── bingo-skills v0.6.6
     │   │   └── bingo v0.6.6 (*)
     │   ├── bingo-store-jsonl v0.6.6
     │   │   └── bingo v0.6.6 (*)
     │   ├── bingo-surface-print v0.6.6 (*)
     │   ├── bingo-surface-rpc v0.6.6
     │   │   └── bingo v0.6.6 (*)
     │   ├── bingo-surface-tui v0.6.6 (*)
     │   ├── bingo-tasks v0.6.6
     │   │   └── bingo v0.6.6 (*)
     │   ├── bingo-tool-bash v0.6.6
     │   │   └── bingo v0.6.6 (*)
     │   ├── bingo-tool-fs v0.6.6 (*)
     │   └── bingo-tool-web v0.6.6 (*)
     ├── bingo-store-jsonl v0.6.6 (*)
     ├── (dev) bingo-surface-print v0.6.6 (*)
     ├── bingo-surface-tui v0.6.6 (*)
     └── rmcp v3.1.4
         ├── (dev) bingo v0.6.6 (*)
         ├── bingo-mcp v0.6.6 (*)
         └── bingo-provider-acp v0.6.6 (*)

warning[license-not-encountered]: license was not encountered
   ┌─ /Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/deny.toml:23:26
   │
23 │   "MPL-2.0", "BSL-1.0", "OpenSSL", "CDLA-Permissive-2.0",
   │                          ━━━━━━━ unmatched license allowance

warning[license-not-encountered]: license was not encountered
   ┌─ /Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/deny.toml:22:66
   │
22 │   "ISC", "Zlib", "CC0-1.0", "Unlicense", "0BSD", "Unicode-3.0", "Unicode-DFS-2016",
   │                                                                  ━━━━━━━━━━━━━━━━ unmatched license allowance

warning[duplicate]: found 2 duplicate entries for crate 'core-foundation'
    ┌─ /Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/Cargo.lock:99:1
    │
 99 │ ╭ core-foundation 0.9.4 registry+https://github.com/rust-lang/crates.io-index
100 │ │ core-foundation 0.10.1 registry+https://github.com/rust-lang/crates.io-index
    │ ╰────────────────────────────────────────────────────────────────────────────┘ lock entries
    │
    ├ core-foundation v0.9.4
      └── system-configuration v0.7.0
          └── hyper-util v0.1.20
              ├── hyper-rustls v0.27.9
              │   └── reqwest v0.13.4
              │       ├── bingo v0.6.6
              │       ├── bingo-auth-oauth v0.6.6
              │       │   ├── bingo v0.6.6 (*)
              │       │   ├── bingo-channel-adapter v0.6.6
              │       │   │   ├── bingo v0.6.6 (*)
              │       │   │   ├── bingo-channel-feishu v0.6.6
              │       │   │   │   ├── bingo v0.6.6 (*)
              │       │   │   │   └── (dev) bingo-gateway v0.6.6
              │       │   │   │       └── bingo v0.6.6 (*)
              │       │   │   ├── bingo-channels v0.6.6
              │       │   │   │   ├── bingo v0.6.6 (*)
              │       │   │   │   └── bingo-gateway v0.6.6 (*)
              │       │   │   └── bingo-gateway v0.6.6 (*)
              │       │   ├── bingo-mcp v0.6.6
              │       │   │   └── bingo v0.6.6 (*)
              │       │   ├── bingo-provider-anthropic v0.6.6
              │       │   │   └── bingo v0.6.6 (*)
              │       │   └── bingo-provider-openai v0.6.6
              │       │       └── bingo v0.6.6 (*)
              │       ├── bingo-channel-feishu v0.6.6 (*)
              │       ├── bingo-http v0.6.6
              │       │   ├── bingo v0.6.6 (*)
              │       │   ├── (dev) bingo-auth-oauth v0.6.6 (*)
              │       │   ├── bingo-channel-feishu v0.6.6 (*)
              │       │   ├── bingo-mcp v0.6.6 (*)
              │       │   ├── bingo-pictures v0.6.6
              │       │   │   ├── (dev) bingo v0.6.6 (*)
              │       │   │   ├── bingo-channel-feishu v0.6.6 (*)
              │       │   │   ├── bingo-mcp v0.6.6 (*)
              │       │   │   ├── bingo-plugin-rpc v0.6.6
              │       │   │   │   └── bingo v0.6.6 (*)
              │       │   │   ├── bingo-surface-print v0.6.6
              │       │   │   │   └── bingo v0.6.6 (*)
              │       │   │   ├── bingo-surface-tui v0.6.6
              │       │   │   │   └── bingo v0.6.6 (*)
              │       │   │   ├── bingo-tool-fs v0.6.6
              │       │   │   │   └── bingo v0.6.6 (*)
              │       │   │   └── bingo-tool-web v0.6.6
              │       │   │       └── bingo v0.6.6 (*)
              │       │   ├── bingo-provider-anthropic v0.6.6 (*)
              │       │   ├── bingo-provider-openai v0.6.6 (*)
              │       │   ├── bingo-surface-tui v0.6.6 (*)
              │       │   └── bingo-tool-web v0.6.6 (*)
              │       ├── bingo-mcp v0.6.6 (*)
              │       ├── bingo-pictures v0.6.6 (*)
              │       ├── bingo-provider-anthropic v0.6.6 (*)
              │       ├── bingo-provider-openai v0.6.6 (*)
              │       ├── bingo-surface-tui v0.6.6 (*)
              │       ├── bingo-tool-web v0.6.6 (*)
              │       └── rmcp v3.1.4
              │           ├── (dev) bingo v0.6.6 (*)
              │           ├── bingo-mcp v0.6.6 (*)
              │           └── bingo-provider-acp v0.6.6
              │               └── bingo v0.6.6 (*)
              ├── reqwest v0.13.4 (*)
              └── wiremock v0.6.5
                  ├── (dev) bingo v0.6.6 (*)
                  ├── (dev) bingo-auth-oauth v0.6.6 (*)
                  ├── (dev) bingo-channel-feishu v0.6.6 (*)
                  ├── (dev) bingo-mcp v0.6.6 (*)
                  ├── (dev) bingo-pictures v0.6.6 (*)
                  ├── (dev) bingo-provider-anthropic v0.6.6 (*)
                  ├── (dev) bingo-provider-openai v0.6.6 (*)
                  ├── (dev) bingo-surface-tui v0.6.6 (*)
                  └── (dev) bingo-tool-web v0.6.6 (*)
    ├ core-foundation v0.10.1
      ├── rustls-platform-verifier v0.7.0
      │   └── reqwest v0.13.4
      │       ├── bingo v0.6.6
      │       ├── bingo-auth-oauth v0.6.6
      │       │   ├── bingo v0.6.6 (*)
      │       │   ├── bingo-channel-adapter v0.6.6
      │       │   │   ├── bingo v0.6.6 (*)
      │       │   │   ├── bingo-channel-feishu v0.6.6
      │       │   │   │   ├── bingo v0.6.6 (*)
      │       │   │   │   └── (dev) bingo-gateway v0.6.6
      │       │   │   │       └── bingo v0.6.6 (*)
      │       │   │   ├── bingo-channels v0.6.6
      │       │   │   │   ├── bingo v0.6.6 (*)
      │       │   │   │   └── bingo-gateway v0.6.6 (*)
      │       │   │   └── bingo-gateway v0.6.6 (*)
      │       │   ├── bingo-mcp v0.6.6
      │       │   │   └── bingo v0.6.6 (*)
      │       │   ├── bingo-provider-anthropic v0.6.6
      │       │   │   └── bingo v0.6.6 (*)
      │       │   └── bingo-provider-openai v0.6.6
      │       │       └── bingo v0.6.6 (*)
      │       ├── bingo-channel-feishu v0.6.6 (*)
      │       ├── bingo-http v0.6.6
      │       │   ├── bingo v0.6.6 (*)
      │       │   ├── (dev) bingo-auth-oauth v0.6.6 (*)
      │       │   ├── bingo-channel-feishu v0.6.6 (*)
      │       │   ├── bingo-mcp v0.6.6 (*)
      │       │   ├── bingo-pictures v0.6.6
      │       │   │   ├── (dev) bingo v0.6.6 (*)
      │       │   │   ├── bingo-channel-feishu v0.6.6 (*)
      │       │   │   ├── bingo-mcp v0.6.6 (*)
      │       │   │   ├── bingo-plugin-rpc v0.6.6
      │       │   │   │   └── bingo v0.6.6 (*)
      │       │   │   ├── bingo-surface-print v0.6.6
      │       │   │   │   └── bingo v0.6.6 (*)
      │       │   │   ├── bingo-surface-tui v0.6.6
      │       │   │   │   └── bingo v0.6.6 (*)
      │       │   │   ├── bingo-tool-fs v0.6.6
      │       │   │   │   └── bingo v0.6.6 (*)
      │       │   │   └── bingo-tool-web v0.6.6
      │       │   │       └── bingo v0.6.6 (*)
      │       │   ├── bingo-provider-anthropic v0.6.6 (*)
      │       │   ├── bingo-provider-openai v0.6.6 (*)
      │       │   ├── bingo-surface-tui v0.6.6 (*)
      │       │   └── bingo-tool-web v0.6.6 (*)
      │       ├── bingo-mcp v0.6.6 (*)
      │       ├── bingo-pictures v0.6.6 (*)
      │       ├── bingo-provider-anthropic v0.6.6 (*)
      │       ├── bingo-provider-openai v0.6.6 (*)
      │       ├── bingo-surface-tui v0.6.6 (*)
      │       ├── bingo-tool-web v0.6.6 (*)
      │       └── rmcp v3.1.4
      │           ├── (dev) bingo v0.6.6 (*)
      │           ├── bingo-mcp v0.6.6 (*)
      │           └── bingo-provider-acp v0.6.6
      │               └── bingo v0.6.6 (*)
      └── security-framework v3.7.0
          ├── rustls-native-certs v0.8.4
          │   ├── rustls-platform-verifier v0.7.0 (*)
          │   └── tokio-tungstenite v0.30.0
          │       └── bingo-channel-feishu v0.6.6 (*)
          └── rustls-platform-verifier v0.7.0 (*)

warning[duplicate]: found 2 duplicate entries for crate 'darling'
    ┌─ /Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/Cargo.lock:114:1
    │
114 │ ╭ darling 0.23.0 registry+https://github.com/rust-lang/crates.io-index
115 │ │ darling 0.24.1 registry+https://github.com/rust-lang/crates.io-index
    │ ╰────────────────────────────────────────────────────────────────────┘ lock entries
    │
    ├ darling v0.23.0
      └── serde_with_macros v3.22.0
          └── serde_with v3.22.0
              └── agent-client-protocol-schema v1.5.0
                  └── bingo-provider-acp v0.6.6
                      └── bingo v0.6.6
    ├ darling v0.24.1
      └── instability v0.3.13
          ├── ratatui v0.30.2
          │   └── bingo-surface-tui v0.6.6
          │       └── bingo v0.6.6
          ├── ratatui-crossterm v0.1.2
          │   └── ratatui v0.30.2 (*)
          └── ratatui-widgets v0.3.2
              └── ratatui v0.30.2 (*)

warning[duplicate]: found 2 duplicate entries for crate 'darling_core'
    ┌─ /Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/Cargo.lock:116:1
    │
116 │ ╭ darling_core 0.23.0 registry+https://github.com/rust-lang/crates.io-index
117 │ │ darling_core 0.24.1 registry+https://github.com/rust-lang/crates.io-index
    │ ╰─────────────────────────────────────────────────────────────────────────┘ lock entries
    │
    ├ darling_core v0.23.0
      ├── darling v0.23.0
      │   └── serde_with_macros v3.22.0
      │       └── serde_with v3.22.0
      │           └── agent-client-protocol-schema v1.5.0
      │               └── bingo-provider-acp v0.6.6
      │                   └── bingo v0.6.6
      └── darling_macro v0.23.0
          └── darling v0.23.0 (*)
    ├ darling_core v0.24.1
      ├── darling v0.24.1
      │   └── instability v0.3.13
      │       ├── ratatui v0.30.2
      │       │   └── bingo-surface-tui v0.6.6
      │       │       └── bingo v0.6.6
      │       ├── ratatui-crossterm v0.1.2
      │       │   └── ratatui v0.30.2 (*)
      │       └── ratatui-widgets v0.3.2
      │           └── ratatui v0.30.2 (*)
      └── darling_macro v0.24.1
          └── darling v0.24.1 (*)

warning[duplicate]: found 2 duplicate entries for crate 'darling_macro'
    ┌─ /Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/Cargo.lock:118:1
    │
118 │ ╭ darling_macro 0.23.0 registry+https://github.com/rust-lang/crates.io-index
119 │ │ darling_macro 0.24.1 registry+https://github.com/rust-lang/crates.io-index
    │ ╰──────────────────────────────────────────────────────────────────────────┘ lock entries
    │
    ├ darling_macro v0.23.0
      └── darling v0.23.0
          └── serde_with_macros v3.22.0
              └── serde_with v3.22.0
                  └── agent-client-protocol-schema v1.5.0
                      └── bingo-provider-acp v0.6.6
                          └── bingo v0.6.6
    ├ darling_macro v0.24.1
      └── darling v0.24.1
          └── instability v0.3.13
              ├── ratatui v0.30.2
              │   └── bingo-surface-tui v0.6.6
              │       └── bingo v0.6.6
              ├── ratatui-crossterm v0.1.2
              │   └── ratatui v0.30.2 (*)
              └── ratatui-widgets v0.3.2
                  └── ratatui v0.30.2 (*)

warning[duplicate]: found 2 duplicate entries for crate 'hashbrown'
    ┌─ /Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/Cargo.lock:173:1
    │
173 │ ╭ hashbrown 0.16.1 registry+https://github.com/rust-lang/crates.io-index
174 │ │ hashbrown 0.17.1 registry+https://github.com/rust-lang/crates.io-index
    │ ╰──────────────────────────────────────────────────────────────────────┘ lock entries
    │
    ├ hashbrown v0.16.1
      └── kasuari v0.4.12
          └── ratatui-core v0.1.2
              ├── ratatui v0.30.2
              │   └── bingo-surface-tui v0.6.6
              │       └── bingo v0.6.6
              ├── ratatui-crossterm v0.1.2
              │   └── ratatui v0.30.2 (*)
              └── ratatui-widgets v0.3.2
                  └── ratatui v0.30.2 (*)
    ├ hashbrown v0.17.1
      ├── indexmap v2.14.0
      │   ├── h2 v0.4.19
      │   │   ├── hyper v1.11.1
      │   │   │   ├── hyper-rustls v0.27.9
      │   │   │   │   └── reqwest v0.13.4
      │   │   │   │       ├── bingo v0.6.6
      │   │   │   │       ├── bingo-auth-oauth v0.6.6
      │   │   │   │       │   ├── bingo v0.6.6 (*)
      │   │   │   │       │   ├── bingo-channel-adapter v0.6.6
      │   │   │   │       │   │   ├── bingo v0.6.6 (*)
      │   │   │   │       │   │   ├── bingo-channel-feishu v0.6.6
      │   │   │   │       │   │   │   ├── bingo v0.6.6 (*)
      │   │   │   │       │   │   │   └── (dev) bingo-gateway v0.6.6
      │   │   │   │       │   │   │       └── bingo v0.6.6 (*)
      │   │   │   │       │   │   ├── bingo-channels v0.6.6
      │   │   │   │       │   │   │   ├── bingo v0.6.6 (*)
      │   │   │   │       │   │   │   └── bingo-gateway v0.6.6 (*)
      │   │   │   │       │   │   └── bingo-gateway v0.6.6 (*)
      │   │   │   │       │   ├── bingo-mcp v0.6.6
      │   │   │   │       │   │   └── bingo v0.6.6 (*)
      │   │   │   │       │   ├── bingo-provider-anthropic v0.6.6
      │   │   │   │       │   │   └── bingo v0.6.6 (*)
      │   │   │   │       │   └── bingo-provider-openai v0.6.6
      │   │   │   │       │       └── bingo v0.6.6 (*)
      │   │   │   │       ├── bingo-channel-feishu v0.6.6 (*)
      │   │   │   │       ├── bingo-http v0.6.6
      │   │   │   │       │   ├── bingo v0.6.6 (*)
      │   │   │   │       │   ├── (dev) bingo-auth-oauth v0.6.6 (*)
      │   │   │   │       │   ├── bingo-channel-feishu v0.6.6 (*)
      │   │   │   │       │   ├── bingo-mcp v0.6.6 (*)
      │   │   │   │       │   ├── bingo-pictures v0.6.6
      │   │   │   │       │   │   ├── (dev) bingo v0.6.6 (*)
      │   │   │   │       │   │   ├── bingo-channel-feishu v0.6.6 (*)
      │   │   │   │       │   │   ├── bingo-mcp v0.6.6 (*)
      │   │   │   │       │   │   ├── bingo-plugin-rpc v0.6.6
      │   │   │   │       │   │   │   └── bingo v0.6.6 (*)
      │   │   │   │       │   │   ├── bingo-surface-print v0.6.6
      │   │   │   │       │   │   │   └── bingo v0.6.6 (*)
      │   │   │   │       │   │   ├── bingo-surface-tui v0.6.6
      │   │   │   │       │   │   │   └── bingo v0.6.6 (*)
      │   │   │   │       │   │   ├── bingo-tool-fs v0.6.6
      │   │   │   │       │   │   │   └── bingo v0.6.6 (*)
      │   │   │   │       │   │   └── bingo-tool-web v0.6.6
      │   │   │   │       │   │       └── bingo v0.6.6 (*)
      │   │   │   │       │   ├── bingo-provider-anthropic v0.6.6 (*)
      │   │   │   │       │   ├── bingo-provider-openai v0.6.6 (*)
      │   │   │   │       │   ├── bingo-surface-tui v0.6.6 (*)
      │   │   │   │       │   └── bingo-tool-web v0.6.6 (*)
      │   │   │   │       ├── bingo-mcp v0.6.6 (*)
      │   │   │   │       ├── bingo-pictures v0.6.6 (*)
      │   │   │   │       ├── bingo-provider-anthropic v0.6.6 (*)
      │   │   │   │       ├── bingo-provider-openai v0.6.6 (*)
      │   │   │   │       ├── bingo-surface-tui v0.6.6 (*)
      │   │   │   │       ├── bingo-tool-web v0.6.6 (*)
      │   │   │   │       └── rmcp v3.1.4
      │   │   │   │           ├── (dev) bingo v0.6.6 (*)
      │   │   │   │           ├── bingo-mcp v0.6.6 (*)
      │   │   │   │           └── bingo-provider-acp v0.6.6
      │   │   │   │               └── bingo v0.6.6 (*)
      │   │   │   ├── hyper-util v0.1.20
      │   │   │   │   ├── hyper-rustls v0.27.9 (*)
      │   │   │   │   ├── reqwest v0.13.4 (*)
      │   │   │   │   └── wiremock v0.6.5
      │   │   │   │       ├── (dev) bingo v0.6.6 (*)
      │   │   │   │       ├── (dev) bingo-auth-oauth v0.6.6 (*)
      │   │   │   │       ├── (dev) bingo-channel-feishu v0.6.6 (*)
      │   │   │   │       ├── (dev) bingo-mcp v0.6.6 (*)
      │   │   │   │       ├── (dev) bingo-pictures v0.6.6 (*)
      │   │   │   │       ├── (dev) bingo-provider-anthropic v0.6.6 (*)
      │   │   │   │       ├── (dev) bingo-provider-openai v0.6.6 (*)
      │   │   │   │       ├── (dev) bingo-surface-tui v0.6.6 (*)
      │   │   │   │       └── (dev) bingo-tool-web v0.6.6 (*)
      │   │   │   ├── reqwest v0.13.4 (*)
      │   │   │   └── wiremock v0.6.5 (*)
      │   │   └── reqwest v0.13.4 (*)
      │   ├── process-wrap v10.0.0
      │   │   ├── bingo-gateway v0.6.6 (*)
      │   │   └── bingo-process v0.6.6
      │   │       ├── (dev) bingo v0.6.6 (*)
      │   │       ├── bingo-hooks-shell v0.6.6
      │   │       │   └── bingo v0.6.6 (*)
      │   │       ├── bingo-mcp v0.6.6 (*)
      │   │       ├── bingo-plugin-rpc v0.6.6 (*)
      │   │       ├── bingo-provider-acp v0.6.6 (*)
      │   │       └── bingo-tool-bash v0.6.6
      │   │           └── bingo v0.6.6 (*)
      │   ├── process-wrap v9.1.0
      │   │   └── rmcp v3.1.4 (*)
      │   ├── rmcp v3.1.4 (*)
      │   ├── serde_json v1.0.151
      │   │   ├── agent-client-protocol-schema v1.5.0
      │   │   │   └── bingo-provider-acp v0.6.6 (*)
      │   │   ├── assert-json-diff v2.0.2
      │   │   │   └── wiremock v0.6.5 (*)
      │   │   ├── bingo v0.6.6 (*)
      │   │   ├── bingo-agents v0.6.6
      │   │   │   └── bingo v0.6.6 (*)
      │   │   ├── bingo-auth-oauth v0.6.6 (*)
      │   │   ├── bingo-channel-adapter v0.6.6 (*)
      │   │   ├── bingo-channel-feishu v0.6.6 (*)
      │   │   ├── bingo-channels v0.6.6 (*)
      │   │   ├── bingo-checkpoints v0.6.6
      │   │   │   └── bingo v0.6.6 (*)
      │   │   ├── bingo-collab v0.6.6
      │   │   │   ├── (dev) bingo v0.6.6 (*)
      │   │   │   ├── bingo-agents v0.6.6 (*)
      │   │   │   ├── bingo-rooms v0.6.6
      │   │   │   │   └── bingo v0.6.6 (*)
      │   │   │   └── bingo-surface-tui v0.6.6 (*)
      │   │   ├── bingo-context v0.6.6
      │   │   │   └── bingo v0.6.6 (*)
      │   │   ├── bingo-core v0.6.6
      │   │   │   ├── bingo v0.6.6 (*)
      │   │   │   └── bingo-gateway v0.6.6 (*)
      │   │   ├── bingo-demo-ui v0.6.6
      │   │   │   └── bingo v0.6.6 (*)
      │   │   ├── bingo-experience v0.6.6
      │   │   │   └── bingo v0.6.6 (*)
      │   │   ├── bingo-gateway v0.6.6 (*)
      │   │   ├── bingo-hooks-shell v0.6.6 (*)
      │   │   ├── bingo-jsonrpc v0.6.6
      │   │   │   ├── bingo-plugin-rpc v0.6.6 (*)
      │   │   │   ├── bingo-provider-acp v0.6.6 (*)
      │   │   │   └── bingo-surface-rpc v0.6.6
      │   │   │       └── bingo v0.6.6 (*)
      │   │   ├── bingo-loopback v0.6.6
      │   │   │   ├── bingo-auth-oauth v0.6.6 (*)
      │   │   │   ├── bingo-provider-openai v0.6.6 (*)
      │   │   │   ├── bingo-surface-tui v0.6.6 (*)
      │   │   │   └── bingo-tool-web v0.6.6 (*)
      │   │   ├── bingo-mcp v0.6.6 (*)
      │   │   ├── (dev) bingo-permissions v0.6.6
      │   │   │   └── bingo v0.6.6 (*)
      │   │   ├── (dev) bingo-persona v0.6.6
      │   │   │   └── bingo v0.6.6 (*)
      │   │   ├── bingo-plugin-rpc v0.6.6 (*)
      │   │   ├── bingo-provider-acp v0.6.6 (*)
      │   │   ├── bingo-provider-anthropic v0.6.6 (*)
      │   │   ├── bingo-provider-fake v0.6.6
      │   │   │   └── bingo v0.6.6 (*)
      │   │   ├── bingo-provider-openai v0.6.6 (*)
      │   │   ├── bingo-rooms v0.6.6 (*)
      │   │   ├── bingo-schedule v0.6.6
      │   │   │   ├── bingo v0.6.6 (*)
      │   │   │   └── bingo-gateway v0.6.6 (*)
      │   │   ├── bingo-schema-ts v0.6.6
      │   │   │   └── (dev) bingo-plugin-rpc v0.6.6 (*)
      │   │   ├── bingo-sdk v0.6.6
      │   │   │   ├── bingo v0.6.6 (*)
      │   │   │   ├── bingo-agents v0.6.6 (*)
      │   │   │   ├── bingo-alarm v0.6.6
      │   │   │   │   ├── bingo-agents v0.6.6 (*)
      │   │   │   │   ├── bingo-rooms v0.6.6 (*)
      │   │   │   │   └── bingo-schedule v0.6.6 (*)
      │   │   │   ├── bingo-auth-oauth v0.6.6 (*)
      │   │   │   ├── bingo-channel-adapter v0.6.6 (*)
      │   │   │   ├── bingo-channel-feishu v0.6.6 (*)
      │   │   │   ├── bingo-channels v0.6.6 (*)
      │   │   │   ├── bingo-checkpoints v0.6.6 (*)
      │   │   │   ├── bingo-collab v0.6.6 (*)
      │   │   │   ├── bingo-context v0.6.6 (*)
      │   │   │   ├── bingo-core v0.6.6 (*)
      │   │   │   ├── bingo-demo-ui v0.6.6 (*)
      │   │   │   ├── bingo-experience v0.6.6 (*)
      │   │   │   ├── bingo-gateway v0.6.6 (*)
      │   │   │   ├── bingo-hooks-shell v0.6.6 (*)
      │   │   │   ├── bingo-http v0.6.6 (*)
      │   │   │   ├── bingo-mcp v0.6.6 (*)
      │   │   │   ├── bingo-permissions v0.6.6 (*)
      │   │   │   ├── bingo-persona v0.6.6 (*)
      │   │   │   ├── bingo-pictures v0.6.6 (*)
      │   │   │   ├── bingo-plugin-rpc v0.6.6 (*)
      │   │   │   ├── bingo-provider-acp v0.6.6 (*)
      │   │   │   ├── bingo-provider-anthropic v0.6.6 (*)
      │   │   │   ├── bingo-provider-fake v0.6.6 (*)
      │   │   │   ├── bingo-provider-openai v0.6.6 (*)
      │   │   │   ├── bingo-rooms v0.6.6 (*)
      │   │   │   ├── bingo-schedule v0.6.6 (*)
      │   │   │   ├── bingo-services v0.6.6
      │   │   │   │   ├── (dev) bingo v0.6.6 (*)
      │   │   │   │   ├── bingo-agents v0.6.6 (*)
      │   │   │   │   ├── bingo-collab v0.6.6 (*)
      │   │   │   │   ├── bingo-mcp v0.6.6 (*)
      │   │   │   │   ├── bingo-provider-acp v0.6.6 (*)
      │   │   │   │   └── bingo-rooms v0.6.6 (*)
      │   │   │   ├── bingo-skills v0.6.6
      │   │   │   │   └── bingo v0.6.6 (*)
      │   │   │   ├── bingo-store-jsonl v0.6.6
      │   │   │   │   └── bingo v0.6.6 (*)
      │   │   │   ├── bingo-surface-print v0.6.6 (*)
      │   │   │   ├── bingo-surface-rpc v0.6.6 (*)
      │   │   │   ├── bingo-surface-tui v0.6.6 (*)
      │   │   │   ├── bingo-tasks v0.6.6
      │   │   │   │   └── bingo v0.6.6 (*)
      │   │   │   ├── bingo-tool-bash v0.6.6 (*)
      │   │   │   ├── bingo-tool-fs v0.6.6 (*)
      │   │   │   └── bingo-tool-web v0.6.6 (*)
      │   │   ├── bingo-services v0.6.6 (*)
      │   │   ├── bingo-shelf v0.6.6
      │   │   │   ├── bingo-experience v0.6.6 (*)
      │   │   │   ├── bingo-plugin-rpc v0.6.6 (*)
      │   │   │   ├── bingo-schedule v0.6.6 (*)
      │   │   │   └── bingo-tool-fs v0.6.6 (*)
      │   │   ├── bingo-skills v0.6.6 (*)
      │   │   ├── bingo-store-jsonl v0.6.6 (*)
      │   │   ├── bingo-surface-print v0.6.6 (*)
      │   │   ├── bingo-surface-rpc v0.6.6 (*)
      │   │   ├── bingo-surface-tui v0.6.6 (*)
      │   │   ├── bingo-tasks v0.6.6 (*)
      │   │   ├── bingo-tool-bash v0.6.6 (*)
      │   │   ├── bingo-tool-fs v0.6.6 (*)
      │   │   ├── bingo-tool-web v0.6.6 (*)
      │   │   ├── bingo-update v0.6.6
      │   │   │   ├── bingo v0.6.6 (*)
      │   │   │   └── bingo-surface-tui v0.6.6 (*)
      │   │   ├── reqwest v0.13.4 (*)
      │   │   ├── rmcp v3.1.4 (*)
      │   │   ├── schemars v1.2.2
      │   │   │   ├── agent-client-protocol-schema v1.5.0 (*)
      │   │   │   ├── bingo v0.6.6 (*)
      │   │   │   ├── bingo-agents v0.6.6 (*)
      │   │   │   ├── bingo-channels v0.6.6 (*)
      │   │   │   ├── bingo-collab v0.6.6 (*)
      │   │   │   ├── (dev) bingo-core v0.6.6 (*)
      │   │   │   ├── bingo-demo-ui v0.6.6 (*)
      │   │   │   ├── bingo-experience v0.6.6 (*)
      │   │   │   ├── bingo-hooks-shell v0.6.6 (*)
      │   │   │   ├── bingo-mcp v0.6.6 (*)
      │   │   │   ├── bingo-permissions v0.6.6 (*)
      │   │   │   ├── bingo-persona v0.6.6 (*)
      │   │   │   ├── bingo-plugin-rpc v0.6.6 (*)
      │   │   │   ├── bingo-provider-acp v0.6.6 (*)
      │   │   │   ├── bingo-provider-anthropic v0.6.6 (*)
      │   │   │   ├── bingo-provider-fake v0.6.6 (*)
      │   │   │   ├── bingo-provider-openai v0.6.6 (*)
      │   │   │   ├── bingo-rooms v0.6.6 (*)
      │   │   │   ├── bingo-schedule v0.6.6 (*)
      │   │   │   ├── bingo-sdk v0.6.6 (*)
      │   │   │   ├── bingo-services v0.6.6 (*)
      │   │   │   ├── bingo-skills v0.6.6 (*)
      │   │   │   ├── bingo-surface-rpc v0.6.6 (*)
      │   │   │   ├── bingo-surface-tui v0.6.6 (*)
      │   │   │   ├── bingo-tasks v0.6.6 (*)
      │   │   │   ├── bingo-tool-bash v0.6.6 (*)
      │   │   │   ├── bingo-tool-fs v0.6.6 (*)
      │   │   │   ├── bingo-tool-web v0.6.6 (*)
      │   │   │   ├── bingo-update v0.6.6 (*)
      │   │   │   ├── rmcp v3.1.4 (*)
      │   │   │   └── serde_with v3.22.0
      │   │   │       └── agent-client-protocol-schema v1.5.0 (*)
      │   │   ├── serde_with v3.22.0 (*)
      │   │   ├── (build) tree-sitter v0.26.13
      │   │   │   └── bingo-permissions v0.6.6 (*)
      │   │   └── wiremock v0.6.5 (*)
      │   └── toml_edit v0.23.10+spec-1.0.0
      │       ├── (dev) bingo v0.6.6 (*)
      │       └── bingo-core v0.6.6 (*)
      ├── lru v0.18.3
      │   └── ratatui-core v0.1.2
      │       ├── ratatui v0.30.2
      │       │   └── bingo-surface-tui v0.6.6 (*)
      │       ├── ratatui-crossterm v0.1.2
      │       │   └── ratatui v0.30.2 (*)
      │       └── ratatui-widgets v0.3.2
      │           └── ratatui v0.30.2 (*)
      ├── ratatui-core v0.1.2 (*)
      └── ratatui-widgets v0.3.2 (*)

warning[duplicate]: found 2 duplicate entries for crate 'html5ever'
    ┌─ /Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/Cargo.lock:179:1
    │
179 │ ╭ html5ever 0.38.0 registry+https://github.com/rust-lang/crates.io-index
180 │ │ html5ever 0.39.0 registry+https://github.com/rust-lang/crates.io-index
    │ ╰──────────────────────────────────────────────────────────────────────┘ lock entries
    │
    ├ html5ever v0.38.0
      ├── htmd v0.5.5
      │   └── bingo-tool-web v0.6.6
      │       └── bingo v0.6.6
      └── markup5ever_rcdom v0.38.0+unofficial
          └── htmd v0.5.5 (*)
    ├ html5ever v0.39.0
      └── dom_query v0.28.0
          └── dom_smoothie v0.18.0
              └── bingo-tool-web v0.6.6
                  └── bingo v0.6.6

warning[duplicate]: found 2 duplicate entries for crate 'markup5ever'
    ┌─ /Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/Cargo.lock:234:1
    │
234 │ ╭ markup5ever 0.38.0 registry+https://github.com/rust-lang/crates.io-index
235 │ │ markup5ever 0.39.0 registry+https://github.com/rust-lang/crates.io-index
    │ ╰────────────────────────────────────────────────────────────────────────┘ lock entries
    │
    ├ markup5ever v0.38.0
      ├── html5ever v0.38.0
      │   ├── htmd v0.5.5
      │   │   └── bingo-tool-web v0.6.6
      │   │       └── bingo v0.6.6
      │   └── markup5ever_rcdom v0.38.0+unofficial
      │       └── htmd v0.5.5 (*)
      ├── markup5ever_rcdom v0.38.0+unofficial (*)
      └── xml5ever v0.38.0
          └── markup5ever_rcdom v0.38.0+unofficial (*)
    ├ markup5ever v0.39.0
      └── html5ever v0.39.0
          └── dom_query v0.28.0
              └── dom_smoothie v0.18.0
                  └── bingo-tool-web v0.6.6
                      └── bingo v0.6.6

warning[duplicate]: found 2 duplicate entries for crate 'miniz_oxide'
    ┌─ /Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/Cargo.lock:240:1
    │
240 │ ╭ miniz_oxide 0.8.9 registry+https://github.com/rust-lang/crates.io-index
241 │ │ miniz_oxide 0.9.1 registry+https://github.com/rust-lang/crates.io-index
    │ ╰───────────────────────────────────────────────────────────────────────┘ lock entries
    │
    ├ miniz_oxide v0.8.9
      └── png v0.18.1
          └── image v0.25.10
              └── bingo-pictures v0.6.6
                  ├── (dev) bingo v0.6.6
                  ├── bingo-channel-feishu v0.6.6
                  │   ├── bingo v0.6.6 (*)
                  │   └── (dev) bingo-gateway v0.6.6
                  │       └── bingo v0.6.6 (*)
                  ├── bingo-mcp v0.6.6
                  │   └── bingo v0.6.6 (*)
                  ├── bingo-plugin-rpc v0.6.6
                  │   └── bingo v0.6.6 (*)
                  ├── bingo-surface-print v0.6.6
                  │   └── bingo v0.6.6 (*)
                  ├── bingo-surface-tui v0.6.6
                  │   └── bingo v0.6.6 (*)
                  ├── bingo-tool-fs v0.6.6
                  │   └── bingo v0.6.6 (*)
                  └── bingo-tool-web v0.6.6
                      └── bingo v0.6.6 (*)
    ├ miniz_oxide v0.9.1
      └── flate2 v1.1.10
          ├── png v0.18.1
          │   └── image v0.25.10
          │       └── bingo-pictures v0.6.6
          │           ├── (dev) bingo v0.6.6
          │           ├── bingo-channel-feishu v0.6.6
          │           │   ├── bingo v0.6.6 (*)
          │           │   └── (dev) bingo-gateway v0.6.6
          │           │       └── bingo v0.6.6 (*)
          │           ├── bingo-mcp v0.6.6
          │           │   └── bingo v0.6.6 (*)
          │           ├── bingo-plugin-rpc v0.6.6
          │           │   └── bingo v0.6.6 (*)
          │           ├── bingo-surface-print v0.6.6
          │           │   └── bingo v0.6.6 (*)
          │           ├── bingo-surface-tui v0.6.6
          │           │   └── bingo v0.6.6 (*)
          │           ├── bingo-tool-fs v0.6.6
          │           │   └── bingo v0.6.6 (*)
          │           └── bingo-tool-web v0.6.6
          │               └── bingo v0.6.6 (*)
          ├── syntect v5.3.0
          │   ├── bingo-surface-tui v0.6.6 (*)
          │   └── two-face v0.5.2+bat-0.26.1
          │       └── bingo-surface-tui v0.6.6 (*)
          └── tiff v0.11.3
              └── image v0.25.10 (*)

warning[duplicate]: found 2 duplicate entries for crate 'process-wrap'
    ┌─ /Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/Cargo.lock:273:1
    │
273 │ ╭ process-wrap 9.1.0 registry+https://github.com/rust-lang/crates.io-index
274 │ │ process-wrap 10.0.0 registry+https://github.com/rust-lang/crates.io-index
    │ ╰─────────────────────────────────────────────────────────────────────────┘ lock entries
    │
    ├ process-wrap v9.1.0
      └── rmcp v3.1.4
          ├── (dev) bingo v0.6.6
          ├── bingo-mcp v0.6.6
          │   └── bingo v0.6.6 (*)
          └── bingo-provider-acp v0.6.6
              └── bingo v0.6.6 (*)
    ├ process-wrap v10.0.0
      ├── bingo-gateway v0.6.6
      │   └── bingo v0.6.6
      └── bingo-process v0.6.6
          ├── (dev) bingo v0.6.6 (*)
          ├── bingo-hooks-shell v0.6.6
          │   └── bingo v0.6.6 (*)
          ├── bingo-mcp v0.6.6
          │   └── bingo v0.6.6 (*)
          ├── bingo-plugin-rpc v0.6.6
          │   └── bingo v0.6.6 (*)
          ├── bingo-provider-acp v0.6.6
          │   └── bingo v0.6.6 (*)
          └── bingo-tool-bash v0.6.6
              └── bingo v0.6.6 (*)

warning[duplicate]: found 2 duplicate entries for crate 'syn'
    ┌─ /Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/Cargo.lock:361:1
    │
361 │ ╭ syn 2.0.119 registry+https://github.com/rust-lang/crates.io-index
362 │ │ syn 3.0.4 registry+https://github.com/rust-lang/crates.io-index
    │ ╰───────────────────────────────────────────────────────────────┘ lock entries
    │
    ├ syn v2.0.119
      ├── cssparser-macros v0.7.0
      │   └── cssparser v0.37.0
      │       ├── dom_query v0.28.0
      │       │   └── dom_smoothie v0.18.0
      │       │       └── bingo-tool-web v0.6.6
      │       │           └── bingo v0.6.6
      │       └── selectors v0.38.0
      │           └── dom_query v0.28.0 (*)
      ├── darling_core v0.23.0
      │   ├── darling v0.23.0
      │   │   └── serde_with_macros v3.22.0
      │   │       └── serde_with v3.22.0
      │   │           └── agent-client-protocol-schema v1.5.0
      │   │               └── bingo-provider-acp v0.6.6
      │   │                   └── bingo v0.6.6 (*)
      │   └── darling_macro v0.23.0
      │       └── darling v0.23.0 (*)
      ├── darling_macro v0.23.0 (*)
      ├── derive_more-impl v2.1.1
      │   └── derive_more v2.1.1
      │       ├── agent-client-protocol-schema v1.5.0 (*)
      │       ├── crossterm v0.29.0
      │       │   ├── bingo-surface-tui v0.6.6
      │       │   │   └── bingo v0.6.6 (*)
      │       │   └── ratatui-crossterm v0.1.2
      │       │       └── ratatui v0.30.2
      │       │           └── bingo-surface-tui v0.6.6 (*)
      │       └── selectors v0.38.0 (*)
      ├── jni-macros v0.22.4
      │   └── jni v0.22.4
      │       └── rustls-platform-verifier v0.7.0
      │           └── reqwest v0.13.4
      │               ├── bingo v0.6.6 (*)
      │               ├── bingo-auth-oauth v0.6.6
      │               │   ├── bingo v0.6.6 (*)
      │               │   ├── bingo-channel-adapter v0.6.6
      │               │   │   ├── bingo v0.6.6 (*)
      │               │   │   ├── bingo-channel-feishu v0.6.6
      │               │   │   │   ├── bingo v0.6.6 (*)
      │               │   │   │   └── (dev) bingo-gateway v0.6.6
      │               │   │   │       └── bingo v0.6.6 (*)
      │               │   │   ├── bingo-channels v0.6.6
      │               │   │   │   ├── bingo v0.6.6 (*)
      │               │   │   │   └── bingo-gateway v0.6.6 (*)
      │               │   │   └── bingo-gateway v0.6.6 (*)
      │               │   ├── bingo-mcp v0.6.6
      │               │   │   └── bingo v0.6.6 (*)
      │               │   ├── bingo-provider-anthropic v0.6.6
      │               │   │   └── bingo v0.6.6 (*)
      │               │   └── bingo-provider-openai v0.6.6
      │               │       └── bingo v0.6.6 (*)
      │               ├── bingo-channel-feishu v0.6.6 (*)
      │               ├── bingo-http v0.6.6
      │               │   ├── bingo v0.6.6 (*)
      │               │   ├── (dev) bingo-auth-oauth v0.6.6 (*)
      │               │   ├── bingo-channel-feishu v0.6.6 (*)
      │               │   ├── bingo-mcp v0.6.6 (*)
      │               │   ├── bingo-pictures v0.6.6
      │               │   │   ├── (dev) bingo v0.6.6 (*)
      │               │   │   ├── bingo-channel-feishu v0.6.6 (*)
      │               │   │   ├── bingo-mcp v0.6.6 (*)
      │               │   │   ├── bingo-plugin-rpc v0.6.6
      │               │   │   │   └── bingo v0.6.6 (*)
      │               │   │   ├── bingo-surface-print v0.6.6
      │               │   │   │   └── bingo v0.6.6 (*)
      │               │   │   ├── bingo-surface-tui v0.6.6 (*)
      │               │   │   ├── bingo-tool-fs v0.6.6
      │               │   │   │   └── bingo v0.6.6 (*)
      │               │   │   └── bingo-tool-web v0.6.6 (*)
      │               │   ├── bingo-provider-anthropic v0.6.6 (*)
      │               │   ├── bingo-provider-openai v0.6.6 (*)
      │               │   ├── bingo-surface-tui v0.6.6 (*)
      │               │   └── bingo-tool-web v0.6.6 (*)
      │               ├── bingo-mcp v0.6.6 (*)
      │               ├── bingo-pictures v0.6.6 (*)
      │               ├── bingo-provider-anthropic v0.6.6 (*)
      │               ├── bingo-provider-openai v0.6.6 (*)
      │               ├── bingo-surface-tui v0.6.6 (*)
      │               ├── bingo-tool-web v0.6.6 (*)
      │               └── rmcp v3.1.4
      │                   ├── (dev) bingo v0.6.6 (*)
      │                   ├── bingo-mcp v0.6.6 (*)
      │                   └── bingo-provider-acp v0.6.6 (*)
      ├── jni-sys-macros v0.4.1
      │   └── jni-sys v0.4.1
      │       └── jni v0.22.4 (*)
      ├── phf_macros v0.13.1
      │   └── phf v0.13.1
      │       ├── cssparser v0.37.0 (*)
      │       ├── dom_smoothie v0.18.0 (*)
      │       ├── htmd v0.5.5
      │       │   └── bingo-tool-web v0.6.6 (*)
      │       ├── selectors v0.38.0 (*)
      │       └── web_atoms v0.2.6
      │           ├── markup5ever v0.38.0
      │           │   ├── html5ever v0.38.0
      │           │   │   ├── htmd v0.5.5 (*)
      │           │   │   └── markup5ever_rcdom v0.38.0+unofficial
      │           │   │       └── htmd v0.5.5 (*)
      │           │   ├── markup5ever_rcdom v0.38.0+unofficial (*)
      │           │   └── xml5ever v0.38.0
      │           │       └── markup5ever_rcdom v0.38.0+unofficial (*)
      │           └── markup5ever v0.39.0
      │               └── html5ever v0.39.0
      │                   └── dom_query v0.28.0 (*)
      ├── serde_with_macros v3.22.0 (*)
      ├── strum_macros v0.28.0
      │   └── strum v0.28.0
      │       ├── agent-client-protocol-schema v1.5.0 (*)
      │       ├── ratatui-core v0.1.2
      │       │   ├── ratatui v0.30.2 (*)
      │       │   ├── ratatui-crossterm v0.1.2 (*)
      │       │   └── ratatui-widgets v0.3.2
      │       │       └── ratatui v0.30.2 (*)
      │       └── ratatui-widgets v0.3.2 (*)
      ├── synstructure v0.13.2
      │   ├── yoke-derive v0.8.2
      │   │   └── yoke v0.8.3
      │   │       ├── icu_collections v2.3.0
      │   │       │   ├── icu_normalizer v2.3.0
      │   │       │   │   └── idna_adapter v1.2.2
      │   │       │   │       └── idna v1.1.0
      │   │       │   │           └── url v2.5.8
      │   │       │   │               ├── bingo-tool-web v0.6.6 (*)
      │   │       │   │               ├── reqwest v0.13.4 (*)
      │   │       │   │               ├── rmcp v3.1.4 (*)
      │   │       │   │               ├── tower-http v0.6.11
      │   │       │   │               │   └── reqwest v0.13.4 (*)
      │   │       │   │               └── wiremock v0.6.5
      │   │       │   │                   ├── (dev) bingo v0.6.6 (*)
      │   │       │   │                   ├── (dev) bingo-auth-oauth v0.6.6 (*)
      │   │       │   │                   ├── (dev) bingo-channel-feishu v0.6.6 (*)
      │   │       │   │                   ├── (dev) bingo-mcp v0.6.6 (*)
      │   │       │   │                   ├── (dev) bingo-pictures v0.6.6 (*)
      │   │       │   │                   ├── (dev) bingo-provider-anthropic v0.6.6 (*)
      │   │       │   │                   ├── (dev) bingo-provider-openai v0.6.6 (*)
      │   │       │   │                   ├── (dev) bingo-surface-tui v0.6.6 (*)
      │   │       │   │                   └── (dev) bingo-tool-web v0.6.6 (*)
      │   │       │   └── icu_properties v2.3.0
      │   │       │       └── idna_adapter v1.2.2 (*)
      │   │       ├── icu_provider v2.3.1
      │   │       │   ├── icu_normalizer v2.3.0 (*)
      │   │       │   └── icu_properties v2.3.0 (*)
      │   │       ├── zerotrie v0.2.5
      │   │       │   ├── icu_properties v2.3.0 (*)
      │   │       │   └── icu_provider v2.3.1 (*)
      │   │       └── zerovec v0.11.8
      │   │           ├── icu_collections v2.3.0 (*)
      │   │           ├── icu_locale_core v2.3.0
      │   │           │   ├── icu_properties v2.3.0 (*)
      │   │           │   └── icu_provider v2.3.1 (*)
      │   │           ├── icu_normalizer v2.3.0 (*)
      │   │           ├── icu_properties v2.3.0 (*)
      │   │           ├── icu_provider v2.3.1 (*)
      │   │           ├── potential_utf v0.1.6
      │   │           │   └── icu_collections v2.3.0 (*)
      │   │           └── tinystr v0.8.4
      │   │               └── icu_locale_core v2.3.0 (*)
      │   └── zerofrom-derive v0.1.7
      │       └── zerofrom v0.1.8
      │           ├── icu_collections v2.3.0 (*)
      │           ├── icu_provider v2.3.1 (*)
      │           ├── yoke v0.8.3 (*)
      │           ├── zerotrie v0.2.5 (*)
      │           └── zerovec v0.11.8 (*)
      ├── thiserror-impl v1.0.69
      │   └── thiserror v1.0.69
      │       └── filedescriptor v0.8.3
      │           └── portable-pty v0.9.0
      │               └── (dev) bingo v0.6.6 (*)
      ├── tracing-attributes v0.1.31
      │   └── tracing v0.1.44
      │       ├── bingo v0.6.6 (*)
      │       ├── bingo-agents v0.6.6
      │       │   └── bingo v0.6.6 (*)
      │       ├── bingo-auth-oauth v0.6.6 (*)
      │       ├── bingo-channel-feishu v0.6.6 (*)
      │       ├── bingo-channels v0.6.6 (*)
      │       ├── bingo-checkpoints v0.6.6
      │       │   └── bingo v0.6.6 (*)
      │       ├── bingo-context v0.6.6
      │       │   └── bingo v0.6.6 (*)
      │       ├── bingo-core v0.6.6
      │       │   ├── bingo v0.6.6 (*)
      │       │   └── bingo-gateway v0.6.6 (*)
      │       ├── bingo-gateway v0.6.6 (*)
      │       ├── bingo-hooks-shell v0.6.6
      │       │   └── bingo v0.6.6 (*)
      │       ├── bingo-mcp v0.6.6 (*)
      │       ├── bingo-permissions v0.6.6
      │       │   └── bingo v0.6.6 (*)
      │       ├── bingo-plugin-rpc v0.6.6 (*)
      │       ├── bingo-rooms v0.6.6
      │       │   └── bingo v0.6.6 (*)
      │       ├── bingo-schedule v0.6.6
      │       │   ├── bingo v0.6.6 (*)
      │       │   └── bingo-gateway v0.6.6 (*)
      │       ├── bingo-store-jsonl v0.6.6
      │       │   └── bingo v0.6.6 (*)
      │       ├── bingo-surface-rpc v0.6.6
      │       │   └── bingo v0.6.6 (*)
      │       ├── bingo-surface-tui v0.6.6 (*)
      │       ├── bingo-tasks v0.6.6
      │       │   └── bingo v0.6.6 (*)
      │       ├── bingo-tool-bash v0.6.6
      │       │   └── bingo v0.6.6 (*)
      │       ├── bingo-update v0.6.6
      │       │   ├── bingo v0.6.6 (*)
      │       │   └── bingo-surface-tui v0.6.6 (*)
      │       ├── h2 v0.4.19
      │       │   ├── hyper v1.11.1
      │       │   │   ├── hyper-rustls v0.27.9
      │       │   │   │   └── reqwest v0.13.4 (*)
      │       │   │   ├── hyper-util v0.1.20
      │       │   │   │   ├── hyper-rustls v0.27.9 (*)
      │       │   │   │   ├── reqwest v0.13.4 (*)
      │       │   │   │   └── wiremock v0.6.5 (*)
      │       │   │   ├── reqwest v0.13.4 (*)
      │       │   │   └── wiremock v0.6.5 (*)
      │       │   └── reqwest v0.13.4 (*)
      │       ├── hyper-util v0.1.20 (*)
      │       ├── process-wrap v10.0.0
      │       │   ├── bingo-gateway v0.6.6 (*)
      │       │   └── bingo-process v0.6.6
      │       │       ├── (dev) bingo v0.6.6 (*)
      │       │       ├── bingo-hooks-shell v0.6.6 (*)
      │       │       ├── bingo-mcp v0.6.6 (*)
      │       │       ├── bingo-plugin-rpc v0.6.6 (*)
      │       │       ├── bingo-provider-acp v0.6.6 (*)
      │       │       └── bingo-tool-bash v0.6.6 (*)
      │       ├── process-wrap v9.1.0
      │       │   └── rmcp v3.1.4 (*)
      │       └── rmcp v3.1.4 (*)
      ├── wasm-bindgen-macro-support v0.2.127
      │   └── wasm-bindgen-macro v0.2.127
      │       └── wasm-bindgen v0.2.127
      │           ├── iana-time-zone v0.1.65
      │           │   └── chrono v0.4.45
      │           │       ├── rmcp v3.1.4 (*)
      │           │       └── schemars v1.2.2
      │           │           ├── agent-client-protocol-schema v1.5.0 (*)
      │           │           ├── bingo v0.6.6 (*)
      │           │           ├── bingo-agents v0.6.6 (*)
      │           │           ├── bingo-channels v0.6.6 (*)
      │           │           ├── bingo-collab v0.6.6
      │           │           │   ├── (dev) bingo v0.6.6 (*)
      │           │           │   ├── bingo-agents v0.6.6 (*)
      │           │           │   ├── bingo-rooms v0.6.6 (*)
      │           │           │   └── bingo-surface-tui v0.6.6 (*)
      │           │           ├── (dev) bingo-core v0.6.6 (*)
      │           │           ├── bingo-demo-ui v0.6.6
      │           │           │   └── bingo v0.6.6 (*)
      │           │           ├── bingo-experience v0.6.6
      │           │           │   └── bingo v0.6.6 (*)
      │           │           ├── bingo-hooks-shell v0.6.6 (*)
      │           │           ├── bingo-mcp v0.6.6 (*)
      │           │           ├── bingo-permissions v0.6.6 (*)
      │           │           ├── bingo-persona v0.6.6
      │           │           │   └── bingo v0.6.6 (*)
      │           │           ├── bingo-plugin-rpc v0.6.6 (*)
      │           │           ├── bingo-provider-acp v0.6.6 (*)
      │           │           ├── bingo-provider-anthropic v0.6.6 (*)
      │           │           ├── bingo-provider-fake v0.6.6
      │           │           │   └── bingo v0.6.6 (*)
      │           │           ├── bingo-provider-openai v0.6.6 (*)
      │           │           ├── bingo-rooms v0.6.6 (*)
      │           │           ├── bingo-schedule v0.6.6 (*)
      │           │           ├── bingo-sdk v0.6.6
      │           │           │   ├── bingo v0.6.6 (*)
      │           │           │   ├── bingo-agents v0.6.6 (*)
      │           │           │   ├── bingo-alarm v0.6.6
      │           │           │   │   ├── bingo-agents v0.6.6 (*)
      │           │           │   │   ├── bingo-rooms v0.6.6 (*)
      │           │           │   │   └── bingo-schedule v0.6.6 (*)
      │           │           │   ├── bingo-auth-oauth v0.6.6 (*)
      │           │           │   ├── bingo-channel-adapter v0.6.6 (*)
      │           │           │   ├── bingo-channel-feishu v0.6.6 (*)
      │           │           │   ├── bingo-channels v0.6.6 (*)
      │           │           │   ├── bingo-checkpoints v0.6.6 (*)
      │           │           │   ├── bingo-collab v0.6.6 (*)
      │           │           │   ├── bingo-context v0.6.6 (*)
      │           │           │   ├── bingo-core v0.6.6 (*)
      │           │           │   ├── bingo-demo-ui v0.6.6 (*)
      │           │           │   ├── bingo-experience v0.6.6 (*)
      │           │           │   ├── bingo-gateway v0.6.6 (*)
      │           │           │   ├── bingo-hooks-shell v0.6.6 (*)
      │           │           │   ├── bingo-http v0.6.6 (*)
      │           │           │   ├── bingo-mcp v0.6.6 (*)
      │           │           │   ├── bingo-permissions v0.6.6 (*)
      │           │           │   ├── bingo-persona v0.6.6 (*)
      │           │           │   ├── bingo-pictures v0.6.6 (*)
      │           │           │   ├── bingo-plugin-rpc v0.6.6 (*)
      │           │           │   ├── bingo-provider-acp v0.6.6 (*)
      │           │           │   ├── bingo-provider-anthropic v0.6.6 (*)
      │           │           │   ├── bingo-provider-fake v0.6.6 (*)
      │           │           │   ├── bingo-provider-openai v0.6.6 (*)
      │           │           │   ├── bingo-rooms v0.6.6 (*)
      │           │           │   ├── bingo-schedule v0.6.6 (*)
      │           │           │   ├── bingo-services v0.6.6
      │           │           │   │   ├── (dev) bingo v0.6.6 (*)
      │           │           │   │   ├── bingo-agents v0.6.6 (*)
      │           │           │   │   ├── bingo-collab v0.6.6 (*)
      │           │           │   │   ├── bingo-mcp v0.6.6 (*)
      │           │           │   │   ├── bingo-provider-acp v0.6.6 (*)
      │           │           │   │   └── bingo-rooms v0.6.6 (*)
      │           │           │   ├── bingo-skills v0.6.6
      │           │           │   │   └── bingo v0.6.6 (*)
      │           │           │   ├── bingo-store-jsonl v0.6.6 (*)
      │           │           │   ├── bingo-surface-print v0.6.6 (*)
      │           │           │   ├── bingo-surface-rpc v0.6.6 (*)
      │           │           │   ├── bingo-surface-tui v0.6.6 (*)
      │           │           │   ├── bingo-tasks v0.6.6 (*)
      │           │           │   ├── bingo-tool-bash v0.6.6 (*)
      │           │           │   ├── bingo-tool-fs v0.6.6 (*)
      │           │           │   └── bingo-tool-web v0.6.6 (*)
      │           │           ├── bingo-services v0.6.6 (*)
      │           │           ├── bingo-skills v0.6.6 (*)
      │           │           ├── bingo-surface-rpc v0.6.6 (*)
      │           │           ├── bingo-surface-tui v0.6.6 (*)
      │           │           ├── bingo-tasks v0.6.6 (*)
      │           │           ├── bingo-tool-bash v0.6.6 (*)
      │           │           ├── bingo-tool-fs v0.6.6 (*)
      │           │           ├── bingo-tool-web v0.6.6 (*)
      │           │           ├── bingo-update v0.6.6 (*)
      │           │           ├── rmcp v3.1.4 (*)
      │           │           └── serde_with v3.22.0 (*)
      │           ├── js-sys v0.3.104
      │           │   ├── iana-time-zone v0.1.65 (*)
      │           │   ├── reqwest v0.13.4 (*)
      │           │   ├── wasm-bindgen-futures v0.4.77
      │           │   │   ├── reqwest v0.13.4 (*)
      │           │   │   └── wasm-streams v0.5.0
      │           │   │       └── reqwest v0.13.4 (*)
      │           │   ├── wasm-streams v0.5.0 (*)
      │           │   ├── web-sys v0.3.104
      │           │   │   ├── reqwest v0.13.4 (*)
      │           │   │   └── wasm-streams v0.5.0 (*)
      │           │   └── web-time v1.1.0
      │           │       └── ulid v3.0.0
      │           │           ├── bingo-sdk v0.6.6 (*)
      │           │           ├── bingo-shelf v0.6.6
      │           │           │   ├── bingo-experience v0.6.6 (*)
      │           │           │   ├── bingo-plugin-rpc v0.6.6 (*)
      │           │           │   ├── bingo-schedule v0.6.6 (*)
      │           │           │   └── bingo-tool-fs v0.6.6 (*)
      │           │           └── bingo-tool-bash v0.6.6 (*)
      │           ├── reqwest v0.13.4 (*)
      │           ├── wasm-bindgen-futures v0.4.77 (*)
      │           ├── wasm-streams v0.5.0 (*)
      │           ├── web-sys v0.3.104 (*)
      │           └── web-time v1.1.0 (*)
      ├── windows-implement v0.60.2
      │   └── windows-core v0.62.2
      │       ├── iana-time-zone v0.1.65 (*)
      │       ├── windows v0.62.2
      │       │   ├── process-wrap v10.0.0 (*)
      │       │   └── process-wrap v9.1.0 (*)
      │       ├── windows-collections v0.3.2
      │       │   └── windows v0.62.2 (*)
      │       ├── windows-future v0.3.2
      │       │   └── windows v0.62.2 (*)
      │       └── windows-numerics v0.3.1
      │           └── windows v0.62.2 (*)
      ├── windows-interface v0.59.3
      │   └── windows-core v0.62.2 (*)
      ├── yoke-derive v0.8.2 (*)
      ├── zerocopy-derive v0.8.56
      │   └── zerocopy v0.8.56
      │       ├── half v2.7.1
      │       │   └── tiff v0.11.3
      │       │       └── image v0.25.10
      │       │           └── bingo-pictures v0.6.6 (*)
      │       └── ppv-lite86 v0.2.21
      │           └── rand_chacha v0.9.0
      │               └── proptest v1.11.0
      │                   ├── (dev) bingo-context v0.6.6 (*)
      │                   ├── (dev) bingo-core v0.6.6 (*)
      │                   ├── (dev) bingo-experience v0.6.6 (*)
      │                   ├── (dev) bingo-mcp v0.6.6 (*)
      │                   ├── (dev) bingo-permissions v0.6.6 (*)
      │                   ├── (dev) bingo-schedule v0.6.6 (*)
      │                   └── (dev) bingo-sdk v0.6.6 (*)
      └── zerofrom-derive v0.1.7 (*)
    ├ syn v3.0.4
      ├── async-trait v0.1.92
      │   ├── (dev) bingo v0.6.6
      │   ├── bingo-agents v0.6.6
      │   │   └── bingo v0.6.6 (*)
      │   ├── (dev) bingo-auth-oauth v0.6.6
      │   │   ├── bingo v0.6.6 (*)
      │   │   ├── bingo-channel-adapter v0.6.6
      │   │   │   ├── bingo v0.6.6 (*)
      │   │   │   ├── bingo-channel-feishu v0.6.6
      │   │   │   │   ├── bingo v0.6.6 (*)
      │   │   │   │   └── (dev) bingo-gateway v0.6.6
      │   │   │   │       └── bingo v0.6.6 (*)
      │   │   │   ├── bingo-channels v0.6.6
      │   │   │   │   ├── bingo v0.6.6 (*)
      │   │   │   │   └── bingo-gateway v0.6.6 (*)
      │   │   │   └── bingo-gateway v0.6.6 (*)
      │   │   ├── bingo-mcp v0.6.6
      │   │   │   └── bingo v0.6.6 (*)
      │   │   ├── bingo-provider-anthropic v0.6.6
      │   │   │   └── bingo v0.6.6 (*)
      │   │   └── bingo-provider-openai v0.6.6
      │   │       └── bingo v0.6.6 (*)
      │   ├── bingo-channel-adapter v0.6.6 (*)
      │   ├── bingo-channel-feishu v0.6.6 (*)
      │   ├── bingo-channels v0.6.6 (*)
      │   ├── bingo-checkpoints v0.6.6
      │   │   └── bingo v0.6.6 (*)
      │   ├── bingo-context v0.6.6
      │   │   └── bingo v0.6.6 (*)
      │   ├── bingo-core v0.6.6
      │   │   ├── bingo v0.6.6 (*)
      │   │   └── bingo-gateway v0.6.6 (*)
      │   ├── bingo-demo-ui v0.6.6
      │   │   └── bingo v0.6.6 (*)
      │   ├── bingo-experience v0.6.6
      │   │   └── bingo v0.6.6 (*)
      │   ├── bingo-hooks-shell v0.6.6
      │   │   └── bingo v0.6.6 (*)
      │   ├── bingo-mcp v0.6.6 (*)
      │   ├── bingo-permissions v0.6.6
      │   │   └── bingo v0.6.6 (*)
      │   ├── bingo-persona v0.6.6
      │   │   └── bingo v0.6.6 (*)
      │   ├── bingo-plugin-rpc v0.6.6
      │   │   └── bingo v0.6.6 (*)
      │   ├── bingo-provider-acp v0.6.6
      │   │   └── bingo v0.6.6 (*)
      │   ├── bingo-provider-anthropic v0.6.6 (*)
      │   ├── bingo-provider-fake v0.6.6
      │   │   └── bingo v0.6.6 (*)
      │   ├── bingo-provider-openai v0.6.6 (*)
      │   ├── bingo-rooms v0.6.6
      │   │   └── bingo v0.6.6 (*)
      │   ├── bingo-schedule v0.6.6
      │   │   ├── bingo v0.6.6 (*)
      │   │   └── bingo-gateway v0.6.6 (*)
      │   ├── bingo-sdk v0.6.6
      │   │   ├── bingo v0.6.6 (*)
      │   │   ├── bingo-agents v0.6.6 (*)
      │   │   ├── bingo-alarm v0.6.6
      │   │   │   ├── bingo-agents v0.6.6 (*)
      │   │   │   ├── bingo-rooms v0.6.6 (*)
      │   │   │   └── bingo-schedule v0.6.6 (*)
      │   │   ├── bingo-auth-oauth v0.6.6 (*)
      │   │   ├── bingo-channel-adapter v0.6.6 (*)
      │   │   ├── bingo-channel-feishu v0.6.6 (*)
      │   │   ├── bingo-channels v0.6.6 (*)
      │   │   ├── bingo-checkpoints v0.6.6 (*)
      │   │   ├── bingo-collab v0.6.6
      │   │   │   ├── (dev) bingo v0.6.6 (*)
      │   │   │   ├── bingo-agents v0.6.6 (*)
      │   │   │   ├── bingo-rooms v0.6.6 (*)
      │   │   │   └── bingo-surface-tui v0.6.6
      │   │   │       └── bingo v0.6.6 (*)
      │   │   ├── bingo-context v0.6.6 (*)
      │   │   ├── bingo-core v0.6.6 (*)
      │   │   ├── bingo-demo-ui v0.6.6 (*)
      │   │   ├── bingo-experience v0.6.6 (*)
      │   │   ├── bingo-gateway v0.6.6 (*)
      │   │   ├── bingo-hooks-shell v0.6.6 (*)
      │   │   ├── bingo-http v0.6.6
      │   │   │   ├── bingo v0.6.6 (*)
      │   │   │   ├── (dev) bingo-auth-oauth v0.6.6 (*)
      │   │   │   ├── bingo-channel-feishu v0.6.6 (*)
      │   │   │   ├── bingo-mcp v0.6.6 (*)
      │   │   │   ├── bingo-pictures v0.6.6
      │   │   │   │   ├── (dev) bingo v0.6.6 (*)
      │   │   │   │   ├── bingo-channel-feishu v0.6.6 (*)
      │   │   │   │   ├── bingo-mcp v0.6.6 (*)
      │   │   │   │   ├── bingo-plugin-rpc v0.6.6 (*)
      │   │   │   │   ├── bingo-surface-print v0.6.6
      │   │   │   │   │   └── bingo v0.6.6 (*)
      │   │   │   │   ├── bingo-surface-tui v0.6.6 (*)
      │   │   │   │   ├── bingo-tool-fs v0.6.6
      │   │   │   │   │   └── bingo v0.6.6 (*)
      │   │   │   │   └── bingo-tool-web v0.6.6
      │   │   │   │       └── bingo v0.6.6 (*)
      │   │   │   ├── bingo-provider-anthropic v0.6.6 (*)
      │   │   │   ├── bingo-provider-openai v0.6.6 (*)
      │   │   │   ├── bingo-surface-tui v0.6.6 (*)
      │   │   │   └── bingo-tool-web v0.6.6 (*)
      │   │   ├── bingo-mcp v0.6.6 (*)
      │   │   ├── bingo-permissions v0.6.6 (*)
      │   │   ├── bingo-persona v0.6.6 (*)
      │   │   ├── bingo-pictures v0.6.6 (*)
      │   │   ├── bingo-plugin-rpc v0.6.6 (*)
      │   │   ├── bingo-provider-acp v0.6.6 (*)
      │   │   ├── bingo-provider-anthropic v0.6.6 (*)
      │   │   ├── bingo-provider-fake v0.6.6 (*)
      │   │   ├── bingo-provider-openai v0.6.6 (*)
      │   │   ├── bingo-rooms v0.6.6 (*)
      │   │   ├── bingo-schedule v0.6.6 (*)
      │   │   ├── bingo-services v0.6.6
      │   │   │   ├── (dev) bingo v0.6.6 (*)
      │   │   │   ├── bingo-agents v0.6.6 (*)
      │   │   │   ├── bingo-collab v0.6.6 (*)
      │   │   │   ├── bingo-mcp v0.6.6 (*)
      │   │   │   ├── bingo-provider-acp v0.6.6 (*)
      │   │   │   └── bingo-rooms v0.6.6 (*)
      │   │   ├── bingo-skills v0.6.6
      │   │   │   └── bingo v0.6.6 (*)
      │   │   ├── bingo-store-jsonl v0.6.6
      │   │   │   └── bingo v0.6.6 (*)
      │   │   ├── bingo-surface-print v0.6.6 (*)
      │   │   ├── bingo-surface-rpc v0.6.6
      │   │   │   └── bingo v0.6.6 (*)
      │   │   ├── bingo-surface-tui v0.6.6 (*)
      │   │   ├── bingo-tasks v0.6.6
      │   │   │   └── bingo v0.6.6 (*)
      │   │   ├── bingo-tool-bash v0.6.6
      │   │   │   └── bingo v0.6.6 (*)
      │   │   ├── bingo-tool-fs v0.6.6 (*)
      │   │   └── bingo-tool-web v0.6.6 (*)
      │   ├── bingo-services v0.6.6 (*)
      │   ├── bingo-skills v0.6.6 (*)
      │   ├── bingo-store-jsonl v0.6.6 (*)
      │   ├── bingo-surface-print v0.6.6 (*)
      │   ├── bingo-surface-rpc v0.6.6 (*)
      │   ├── bingo-surface-tui v0.6.6 (*)
      │   ├── bingo-tasks v0.6.6 (*)
      │   ├── bingo-tool-bash v0.6.6 (*)
      │   ├── bingo-tool-fs v0.6.6 (*)
      │   └── bingo-tool-web v0.6.6 (*)
      ├── clap_derive v4.6.4
      │   └── clap v4.6.6
      │       ├── bingo v0.6.6 (*)
      │       └── bingo-gateway v0.6.6 (*)
      ├── darling_core v0.24.1
      │   ├── darling v0.24.1
      │   │   └── instability v0.3.13
      │   │       ├── ratatui v0.30.2
      │   │       │   └── bingo-surface-tui v0.6.6 (*)
      │   │       ├── ratatui-crossterm v0.1.2
      │   │       │   └── ratatui v0.30.2 (*)
      │   │       └── ratatui-widgets v0.3.2
      │   │           └── ratatui v0.30.2 (*)
      │   └── darling_macro v0.24.1
      │       └── darling v0.24.1 (*)
      ├── darling_macro v0.24.1 (*)
      ├── displaydoc v0.2.7
      │   ├── icu_collections v2.3.0
      │   │   ├── icu_normalizer v2.3.0
      │   │   │   └── idna_adapter v1.2.2
      │   │   │       └── idna v1.1.0
      │   │   │           └── url v2.5.8
      │   │   │               ├── bingo-tool-web v0.6.6 (*)
      │   │   │               ├── reqwest v0.13.4
      │   │   │               │   ├── bingo v0.6.6 (*)
      │   │   │               │   ├── bingo-auth-oauth v0.6.6 (*)
      │   │   │               │   ├── bingo-channel-feishu v0.6.6 (*)
      │   │   │               │   ├── bingo-http v0.6.6 (*)
      │   │   │               │   ├── bingo-mcp v0.6.6 (*)
      │   │   │               │   ├── bingo-pictures v0.6.6 (*)
      │   │   │               │   ├── bingo-provider-anthropic v0.6.6 (*)
      │   │   │               │   ├── bingo-provider-openai v0.6.6 (*)
      │   │   │               │   ├── bingo-surface-tui v0.6.6 (*)
      │   │   │               │   ├── bingo-tool-web v0.6.6 (*)
      │   │   │               │   └── rmcp v3.1.4
      │   │   │               │       ├── (dev) bingo v0.6.6 (*)
      │   │   │               │       ├── bingo-mcp v0.6.6 (*)
      │   │   │               │       └── bingo-provider-acp v0.6.6 (*)
      │   │   │               ├── rmcp v3.1.4 (*)
      │   │   │               ├── tower-http v0.6.11
      │   │   │               │   └── reqwest v0.13.4 (*)
      │   │   │               └── wiremock v0.6.5
      │   │   │                   ├── (dev) bingo v0.6.6 (*)
      │   │   │                   ├── (dev) bingo-auth-oauth v0.6.6 (*)
      │   │   │                   ├── (dev) bingo-channel-feishu v0.6.6 (*)
      │   │   │                   ├── (dev) bingo-mcp v0.6.6 (*)
      │   │   │                   ├── (dev) bingo-pictures v0.6.6 (*)
      │   │   │                   ├── (dev) bingo-provider-anthropic v0.6.6 (*)
      │   │   │                   ├── (dev) bingo-provider-openai v0.6.6 (*)
      │   │   │                   ├── (dev) bingo-surface-tui v0.6.6 (*)
      │   │   │                   └── (dev) bingo-tool-web v0.6.6 (*)
      │   │   └── icu_properties v2.3.0
      │   │       └── idna_adapter v1.2.2 (*)
      │   ├── icu_locale_core v2.3.0
      │   │   ├── icu_properties v2.3.0 (*)
      │   │   └── icu_provider v2.3.1
      │   │       ├── icu_normalizer v2.3.0 (*)
      │   │       └── icu_properties v2.3.0 (*)
      │   ├── icu_properties v2.3.0 (*)
      │   ├── icu_provider v2.3.1 (*)
      │   ├── tinystr v0.8.4
      │   │   └── icu_locale_core v2.3.0 (*)
      │   └── zerotrie v0.2.5
      │       ├── icu_properties v2.3.0 (*)
      │       └── icu_provider v2.3.1 (*)
      ├── futures-macro v0.3.34
      │   └── futures-util v0.3.34
      │       ├── futures v0.3.34
      │       │   ├── (dev) bingo v0.6.6 (*)
      │       │   ├── bingo-agents v0.6.6 (*)
      │       │   ├── bingo-channel-feishu v0.6.6 (*)
      │       │   ├── bingo-channels v0.6.6 (*)
      │       │   ├── (dev) bingo-checkpoints v0.6.6 (*)
      │       │   ├── bingo-context v0.6.6 (*)
      │       │   ├── bingo-core v0.6.6 (*)
      │       │   ├── (dev) bingo-demo-ui v0.6.6 (*)
      │       │   ├── bingo-http v0.6.6 (*)
      │       │   ├── bingo-jsonrpc v0.6.6
      │       │   │   ├── bingo-plugin-rpc v0.6.6 (*)
      │       │   │   ├── bingo-provider-acp v0.6.6 (*)
      │       │   │   └── bingo-surface-rpc v0.6.6 (*)
      │       │   ├── bingo-pictures v0.6.6 (*)
      │       │   ├── bingo-plugin-rpc v0.6.6 (*)
      │       │   ├── bingo-provider-acp v0.6.6 (*)
      │       │   ├── bingo-provider-anthropic v0.6.6 (*)
      │       │   ├── bingo-provider-fake v0.6.6 (*)
      │       │   ├── bingo-provider-openai v0.6.6 (*)
      │       │   ├── (dev) bingo-rooms v0.6.6 (*)
      │       │   ├── bingo-sdk v0.6.6 (*)
      │       │   ├── bingo-surface-print v0.6.6 (*)
      │       │   ├── bingo-surface-rpc v0.6.6 (*)
      │       │   ├── bingo-surface-tui v0.6.6 (*)
      │       │   ├── (dev) bingo-tasks v0.6.6 (*)
      │       │   ├── bingo-tool-web v0.6.6 (*)
      │       │   ├── process-wrap v10.0.0
      │       │   │   ├── bingo-gateway v0.6.6 (*)
      │       │   │   └── bingo-process v0.6.6
      │       │   │       ├── (dev) bingo v0.6.6 (*)
      │       │   │       ├── bingo-hooks-shell v0.6.6 (*)
      │       │   │       ├── bingo-mcp v0.6.6 (*)
      │       │   │       ├── bingo-plugin-rpc v0.6.6 (*)
      │       │   │       ├── bingo-provider-acp v0.6.6 (*)
      │       │   │       └── bingo-tool-bash v0.6.6 (*)
      │       │   ├── process-wrap v9.1.0
      │       │   │   └── rmcp v3.1.4 (*)
      │       │   ├── rmcp v3.1.4 (*)
      │       │   └── wiremock v0.6.5 (*)
      │       ├── futures-executor v0.3.34
      │       │   └── futures v0.3.34 (*)
      │       ├── hyper-util v0.1.20
      │       │   ├── hyper-rustls v0.27.9
      │       │   │   └── reqwest v0.13.4 (*)
      │       │   ├── reqwest v0.13.4 (*)
      │       │   └── wiremock v0.6.5 (*)
      │       ├── js-sys v0.3.104
      │       │   ├── iana-time-zone v0.1.65
      │       │   │   └── chrono v0.4.45
      │       │   │       ├── rmcp v3.1.4 (*)
      │       │   │       └── schemars v1.2.2
      │       │   │           ├── agent-client-protocol-schema v1.5.0
      │       │   │           │   └── bingo-provider-acp v0.6.6 (*)
      │       │   │           ├── bingo v0.6.6 (*)
      │       │   │           ├── bingo-agents v0.6.6 (*)
      │       │   │           ├── bingo-channels v0.6.6 (*)
      │       │   │           ├── bingo-collab v0.6.6 (*)
      │       │   │           ├── (dev) bingo-core v0.6.6 (*)
      │       │   │           ├── bingo-demo-ui v0.6.6 (*)
      │       │   │           ├── bingo-experience v0.6.6 (*)
      │       │   │           ├── bingo-hooks-shell v0.6.6 (*)
      │       │   │           ├── bingo-mcp v0.6.6 (*)
      │       │   │           ├── bingo-permissions v0.6.6 (*)
      │       │   │           ├── bingo-persona v0.6.6 (*)
      │       │   │           ├── bingo-plugin-rpc v0.6.6 (*)
      │       │   │           ├── bingo-provider-acp v0.6.6 (*)
      │       │   │           ├── bingo-provider-anthropic v0.6.6 (*)
      │       │   │           ├── bingo-provider-fake v0.6.6 (*)
      │       │   │           ├── bingo-provider-openai v0.6.6 (*)
      │       │   │           ├── bingo-rooms v0.6.6 (*)
      │       │   │           ├── bingo-schedule v0.6.6 (*)
      │       │   │           ├── bingo-sdk v0.6.6 (*)
      │       │   │           ├── bingo-services v0.6.6 (*)
      │       │   │           ├── bingo-skills v0.6.6 (*)
      │       │   │           ├── bingo-surface-rpc v0.6.6 (*)
      │       │   │           ├── bingo-surface-tui v0.6.6 (*)
      │       │   │           ├── bingo-tasks v0.6.6 (*)
      │       │   │           ├── bingo-tool-bash v0.6.6 (*)
      │       │   │           ├── bingo-tool-fs v0.6.6 (*)
      │       │   │           ├── bingo-tool-web v0.6.6 (*)
      │       │   │           ├── bingo-update v0.6.6
      │       │   │           │   ├── bingo v0.6.6 (*)
      │       │   │           │   └── bingo-surface-tui v0.6.6 (*)
      │       │   │           ├── rmcp v3.1.4 (*)
      │       │   │           └── serde_with v3.22.0
      │       │   │               └── agent-client-protocol-schema v1.5.0 (*)
      │       │   ├── reqwest v0.13.4 (*)
      │       │   ├── wasm-bindgen-futures v0.4.77
      │       │   │   ├── reqwest v0.13.4 (*)
      │       │   │   └── wasm-streams v0.5.0
      │       │   │       └── reqwest v0.13.4 (*)
      │       │   ├── wasm-streams v0.5.0 (*)
      │       │   ├── web-sys v0.3.104
      │       │   │   ├── reqwest v0.13.4 (*)
      │       │   │   └── wasm-streams v0.5.0 (*)
      │       │   └── web-time v1.1.0
      │       │       └── ulid v3.0.0
      │       │           ├── bingo-sdk v0.6.6 (*)
      │       │           ├── bingo-shelf v0.6.6
      │       │           │   ├── bingo-experience v0.6.6 (*)
      │       │           │   ├── bingo-plugin-rpc v0.6.6 (*)
      │       │           │   ├── bingo-schedule v0.6.6 (*)
      │       │           │   └── bingo-tool-fs v0.6.6 (*)
      │       │           └── bingo-tool-bash v0.6.6 (*)
      │       ├── reqwest v0.13.4 (*)
      │       ├── sse-stream v0.2.5
      │       │   └── rmcp v3.1.4 (*)
      │       ├── tokio-tungstenite v0.30.0
      │       │   └── bingo-channel-feishu v0.6.6 (*)
      │       ├── tokio-util v0.7.19
      │       │   ├── bingo-core v0.6.6 (*)
      │       │   ├── bingo-jsonrpc v0.6.6 (*)
      │       │   ├── bingo-provider-acp v0.6.6 (*)
      │       │   ├── bingo-sdk v0.6.6 (*)
      │       │   ├── bingo-surface-rpc v0.6.6 (*)
      │       │   ├── h2 v0.4.19
      │       │   │   ├── hyper v1.11.1
      │       │   │   │   ├── hyper-rustls v0.27.9 (*)
      │       │   │   │   ├── hyper-util v0.1.20 (*)
      │       │   │   │   ├── reqwest v0.13.4 (*)
      │       │   │   │   └── wiremock v0.6.5 (*)
      │       │   │   └── reqwest v0.13.4 (*)
      │       │   ├── reqwest v0.13.4 (*)
      │       │   └── rmcp v3.1.4 (*)
      │       ├── tower v0.5.3
      │       │   ├── reqwest v0.13.4 (*)
      │       │   └── tower-http v0.6.11 (*)
      │       ├── tower-http v0.6.11 (*)
      │       └── wasm-streams v0.5.0 (*)
      ├── instability v0.3.13 (*)
      ├── ref-cast-impl v1.0.27
      │   └── ref-cast v1.0.27
      │       └── schemars v1.2.2 (*)
      ├── schemars_derive v1.2.2
      │   └── schemars v1.2.2 (*)
      ├── serde_derive v1.0.229
      │   ├── serde v1.0.229
      │   │   ├── agent-client-protocol-schema v1.5.0 (*)
      │   │   ├── assert-json-diff v2.0.2
      │   │   │   └── wiremock v0.6.5 (*)
      │   │   ├── bincode v1.3.3
      │   │   │   └── syntect v5.3.0
      │   │   │       ├── bingo-surface-tui v0.6.6 (*)
      │   │   │       └── two-face v0.5.2+bat-0.26.1
      │   │   │           └── bingo-surface-tui v0.6.6 (*)
      │   │   ├── bingo v0.6.6 (*)
      │   │   ├── bingo-agents v0.6.6 (*)
      │   │   ├── bingo-auth-oauth v0.6.6 (*)
      │   │   ├── bingo-channel-adapter v0.6.6 (*)
      │   │   ├── bingo-channel-feishu v0.6.6 (*)
      │   │   ├── bingo-channels v0.6.6 (*)
      │   │   ├── bingo-collab v0.6.6 (*)
      │   │   ├── bingo-context v0.6.6 (*)
      │   │   ├── bingo-core v0.6.6 (*)
      │   │   ├── bingo-demo-ui v0.6.6 (*)
      │   │   ├── bingo-experience v0.6.6 (*)
      │   │   ├── bingo-gateway v0.6.6 (*)
      │   │   ├── bingo-hooks-shell v0.6.6 (*)
      │   │   ├── bingo-jsonrpc v0.6.6 (*)
      │   │   ├── bingo-mcp v0.6.6 (*)
      │   │   ├── bingo-permissions v0.6.6 (*)
      │   │   ├── bingo-persona v0.6.6 (*)
      │   │   ├── bingo-plugin-rpc v0.6.6 (*)
      │   │   ├── bingo-provider-acp v0.6.6 (*)
      │   │   ├── bingo-provider-anthropic v0.6.6 (*)
      │   │   ├── bingo-provider-fake v0.6.6 (*)
      │   │   ├── bingo-provider-openai v0.6.6 (*)
      │   │   ├── bingo-rooms v0.6.6 (*)
      │   │   ├── bingo-schedule v0.6.6 (*)
      │   │   ├── bingo-sdk v0.6.6 (*)
      │   │   ├── bingo-services v0.6.6 (*)
      │   │   ├── bingo-shelf v0.6.6 (*)
      │   │   ├── bingo-skills v0.6.6 (*)
      │   │   ├── bingo-store-jsonl v0.6.6 (*)
      │   │   ├── bingo-surface-rpc v0.6.6 (*)
      │   │   ├── bingo-surface-tui v0.6.6 (*)
      │   │   ├── bingo-tasks v0.6.6 (*)
      │   │   ├── bingo-tool-bash v0.6.6 (*)
      │   │   ├── bingo-tool-fs v0.6.6 (*)
      │   │   ├── bingo-tool-web v0.6.6 (*)
      │   │   ├── bingo-update v0.6.6 (*)
      │   │   ├── chrono v0.4.45 (*)
      │   │   ├── insta v1.48.0
      │   │   │   ├── (dev) bingo-channel-adapter v0.6.6 (*)
      │   │   │   ├── (dev) bingo-channel-feishu v0.6.6 (*)
      │   │   │   ├── (dev) bingo-context v0.6.6 (*)
      │   │   │   ├── (dev) bingo-core v0.6.6 (*)
      │   │   │   ├── (dev) bingo-hooks-shell v0.6.6 (*)
      │   │   │   ├── (dev) bingo-mcp v0.6.6 (*)
      │   │   │   ├── (dev) bingo-provider-anthropic v0.6.6 (*)
      │   │   │   ├── (dev) bingo-provider-openai v0.6.6 (*)
      │   │   │   ├── (dev) bingo-schedule v0.6.6 (*)
      │   │   │   ├── (dev) bingo-sdk v0.6.6 (*)
      │   │   │   ├── (dev) bingo-skills v0.6.6 (*)
      │   │   │   ├── (dev) bingo-surface-print v0.6.6 (*)
      │   │   │   └── (dev) bingo-surface-tui v0.6.6 (*)
      │   │   ├── jsonc-parser v0.33.1
      │   │   │   └── bingo-core v0.6.6 (*)
      │   │   ├── reqwest v0.13.4 (*)
      │   │   ├── rmcp v3.1.4 (*)
      │   │   ├── schemars v1.2.2 (*)
      │   │   ├── string_cache v0.9.0
      │   │   │   └── web_atoms v0.2.6
      │   │   │       ├── markup5ever v0.38.0
      │   │   │       │   ├── html5ever v0.38.0
      │   │   │       │   │   ├── htmd v0.5.5
      │   │   │       │   │   │   └── bingo-tool-web v0.6.6 (*)
      │   │   │       │   │   └── markup5ever_rcdom v0.38.0+unofficial
      │   │   │       │   │       └── htmd v0.5.5 (*)
      │   │   │       │   ├── markup5ever_rcdom v0.38.0+unofficial (*)
      │   │   │       │   └── xml5ever v0.38.0
      │   │   │       │       └── markup5ever_rcdom v0.38.0+unofficial (*)
      │   │   │       └── markup5ever v0.39.0
      │   │   │           └── html5ever v0.39.0
      │   │   │               └── dom_query v0.28.0
      │   │   │                   └── dom_smoothie v0.18.0
      │   │   │                       └── bingo-tool-web v0.6.6 (*)
      │   │   ├── syntect v5.3.0 (*)
      │   │   ├── two-face v0.5.2+bat-0.26.1 (*)
      │   │   ├── ulid v3.0.0 (*)
      │   │   └── wiremock v0.6.5 (*)
      │   ├── syntect v5.3.0 (*)
      │   └── two-face v0.5.2+bat-0.26.1 (*)
      ├── serde_derive_internals v0.30.0
      │   └── schemars_derive v1.2.2 (*)
      ├── thiserror-impl v2.0.20
      │   └── thiserror v2.0.20
      │       ├── bingo-auth-oauth v0.6.6 (*)
      │       ├── bingo-channel-adapter v0.6.6 (*)
      │       ├── bingo-channel-feishu v0.6.6 (*)
      │       ├── bingo-channels v0.6.6 (*)
      │       ├── bingo-claim v0.6.6
      │       │   ├── bingo-channels v0.6.6 (*)
      │       │   ├── bingo-gateway v0.6.6 (*)
      │       │   ├── bingo-schedule v0.6.6 (*)
      │       │   └── bingo-store-jsonl v0.6.6 (*)
      │       ├── bingo-context v0.6.6 (*)
      │       ├── bingo-core v0.6.6 (*)
      │       ├── bingo-experience v0.6.6 (*)
      │       ├── bingo-hooks-shell v0.6.6 (*)
      │       ├── bingo-http v0.6.6 (*)
      │       ├── bingo-jsonrpc v0.6.6 (*)
      │       ├── bingo-loopback v0.6.6
      │       │   ├── bingo-auth-oauth v0.6.6 (*)
      │       │   ├── bingo-provider-openai v0.6.6 (*)
      │       │   ├── bingo-surface-tui v0.6.6 (*)
      │       │   └── bingo-tool-web v0.6.6 (*)
      │       ├── bingo-permissions v0.6.6 (*)
      │       ├── bingo-pictures v0.6.6 (*)
      │       ├── bingo-process v0.6.6 (*)
      │       ├── bingo-provider-acp v0.6.6 (*)
      │       ├── bingo-provider-fake v0.6.6 (*)
      │       ├── bingo-rooms v0.6.6 (*)
      │       ├── bingo-schedule v0.6.6 (*)
      │       ├── bingo-sdk v0.6.6 (*)
      │       ├── bingo-shelf v0.6.6 (*)
      │       ├── bingo-surface-tui v0.6.6 (*)
      │       ├── bingo-tool-fs v0.6.6 (*)
      │       ├── bingo-tool-web v0.6.6 (*)
      │       ├── bingo-update v0.6.6 (*)
      │       ├── dom_smoothie v0.18.0 (*)
      │       ├── jni v0.22.4
      │       │   └── rustls-platform-verifier v0.7.0
      │       │       └── reqwest v0.13.4 (*)
      │       ├── kasuari v0.4.12
      │       │   └── ratatui-core v0.1.2
      │       │       ├── ratatui v0.30.2 (*)
      │       │       ├── ratatui-crossterm v0.1.2 (*)
      │       │       └── ratatui-widgets v0.3.2 (*)
      │       ├── ratatui-core v0.1.2 (*)
      │       ├── rmcp v3.1.4 (*)
      │       ├── syntect v5.3.0 (*)
      │       └── tungstenite v0.30.0
      │           └── tokio-tungstenite v0.30.0 (*)
      ├── tokio-macros v2.7.2
      │   └── tokio v1.53.1
      │       ├── (dev) bingo v0.6.6 (*)
      │       ├── bingo-agents v0.6.6 (*)
      │       ├── (dev) bingo-alarm v0.6.6 (*)
      │       ├── (dev) bingo-auth-oauth v0.6.6 (*)
      │       ├── bingo-channel-adapter v0.6.6 (*)
      │       ├── (dev) bingo-channel-feishu v0.6.6 (*)
      │       ├── (dev) bingo-channels v0.6.6 (*)
      │       ├── (dev) bingo-checkpoints v0.6.6 (*)
      │       ├── bingo-context v0.6.6 (*)
      │       ├── (dev) bingo-core v0.6.6 (*)
      │       ├── (dev) bingo-demo-ui v0.6.6 (*)
      │       ├── (dev) bingo-experience v0.6.6 (*)
      │       ├── bingo-gateway v0.6.6 (*)
      │       ├── bingo-hooks-shell v0.6.6 (*)
      │       ├── (dev) bingo-http v0.6.6 (*)
      │       ├── bingo-jsonrpc v0.6.6 (*)
      │       ├── (dev) bingo-loopback v0.6.6 (*)
      │       ├── (dev) bingo-mcp v0.6.6 (*)
      │       ├── (dev) bingo-permissions v0.6.6 (*)
      │       ├── (dev) bingo-persona v0.6.6 (*)
      │       ├── bingo-pictures v0.6.6 (*)
      │       ├── (dev) bingo-plugin-rpc v0.6.6 (*)
      │       ├── bingo-process v0.6.6 (*)
      │       ├── (dev) bingo-provider-acp v0.6.6 (*)
      │       ├── (dev) bingo-provider-anthropic v0.6.6 (*)
      │       ├── (dev) bingo-provider-fake v0.6.6 (*)
      │       ├── (dev) bingo-provider-openai v0.6.6 (*)
      │       ├── (dev) bingo-rooms v0.6.6 (*)
      │       ├── (dev) bingo-schedule v0.6.6 (*)
      │       ├── bingo-sdk v0.6.6 (*)
      │       ├── (dev) bingo-services v0.6.6 (*)
      │       ├── (dev) bingo-skills v0.6.6 (*)
      │       ├── bingo-store-jsonl v0.6.6 (*)
      │       ├── bingo-surface-print v0.6.6 (*)
      │       ├── bingo-surface-rpc v0.6.6 (*)
      │       ├── (dev) bingo-surface-tui v0.6.6 (*)
      │       ├── (dev) bingo-tasks v0.6.6 (*)
      │       ├── (dev) bingo-tool-bash v0.6.6 (*)
      │       ├── bingo-tool-fs v0.6.6 (*)
      │       ├── (dev) bingo-tool-web v0.6.6 (*)
      │       ├── (dev) bingo-update v0.6.6 (*)
      │       ├── deadpool v0.12.3
      │       │   └── wiremock v0.6.5 (*)
      │       ├── h2 v0.4.19 (*)
      │       ├── hyper v1.11.1 (*)
      │       ├── hyper-rustls v0.27.9 (*)
      │       ├── hyper-util v0.1.20 (*)
      │       ├── process-wrap v10.0.0 (*)
      │       ├── process-wrap v9.1.0 (*)
      │       ├── reqwest v0.13.4 (*)
      │       ├── rmcp v3.1.4 (*)
      │       ├── tokio-rustls v0.26.4
      │       │   ├── hyper-rustls v0.27.9 (*)
      │       │   ├── reqwest v0.13.4 (*)
      │       │   └── tokio-tungstenite v0.30.0 (*)
      │       ├── tokio-stream v0.1.19
      │       │   └── rmcp v3.1.4 (*)
      │       ├── tokio-tungstenite v0.30.0 (*)
      │       ├── tokio-util v0.7.19 (*)
      │       ├── tower v0.5.3 (*)
      │       └── wiremock v0.6.5 (*)
      └── zerovec-derive v0.11.6
          └── zerovec v0.11.8
              ├── icu_collections v2.3.0 (*)
              ├── icu_locale_core v2.3.0 (*)
              ├── icu_normalizer v2.3.0 (*)
              ├── icu_properties v2.3.0 (*)
              ├── icu_provider v2.3.1 (*)
              ├── potential_utf v0.1.6
              │   └── icu_collections v2.3.0 (*)
              └── tinystr v0.8.4 (*)

warning[duplicate]: found 2 duplicate entries for crate 'untrusted'
    ┌─ /Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/Cargo.lock:412:1
    │
412 │ ╭ untrusted 0.7.1 registry+https://github.com/rust-lang/crates.io-index
413 │ │ untrusted 0.9.0 registry+https://github.com/rust-lang/crates.io-index
    │ ╰─────────────────────────────────────────────────────────────────────┘ lock entries
    │
    ├ untrusted v0.7.1
      └── aws-lc-rs v1.18.0
          ├── bingo-auth-oauth v0.6.6
          │   ├── bingo v0.6.6
          │   ├── bingo-channel-adapter v0.6.6
          │   │   ├── bingo v0.6.6 (*)
          │   │   ├── bingo-channel-feishu v0.6.6
          │   │   │   ├── bingo v0.6.6 (*)
          │   │   │   └── (dev) bingo-gateway v0.6.6
          │   │   │       └── bingo v0.6.6 (*)
          │   │   ├── bingo-channels v0.6.6
          │   │   │   ├── bingo v0.6.6 (*)
          │   │   │   └── bingo-gateway v0.6.6 (*)
          │   │   └── bingo-gateway v0.6.6 (*)
          │   ├── bingo-mcp v0.6.6
          │   │   └── bingo v0.6.6 (*)
          │   ├── bingo-provider-anthropic v0.6.6
          │   │   └── bingo v0.6.6 (*)
          │   └── bingo-provider-openai v0.6.6
          │       └── bingo v0.6.6 (*)
          ├── rustls v0.23.45
          │   ├── hyper-rustls v0.27.9
          │   │   └── reqwest v0.13.4
          │   │       ├── bingo v0.6.6 (*)
          │   │       ├── bingo-auth-oauth v0.6.6 (*)
          │   │       ├── bingo-channel-feishu v0.6.6 (*)
          │   │       ├── bingo-http v0.6.6
          │   │       │   ├── bingo v0.6.6 (*)
          │   │       │   ├── (dev) bingo-auth-oauth v0.6.6 (*)
          │   │       │   ├── bingo-channel-feishu v0.6.6 (*)
          │   │       │   ├── bingo-mcp v0.6.6 (*)
          │   │       │   ├── bingo-pictures v0.6.6
          │   │       │   │   ├── (dev) bingo v0.6.6 (*)
          │   │       │   │   ├── bingo-channel-feishu v0.6.6 (*)
          │   │       │   │   ├── bingo-mcp v0.6.6 (*)
          │   │       │   │   ├── bingo-plugin-rpc v0.6.6
          │   │       │   │   │   └── bingo v0.6.6 (*)
          │   │       │   │   ├── bingo-surface-print v0.6.6
          │   │       │   │   │   └── bingo v0.6.6 (*)
          │   │       │   │   ├── bingo-surface-tui v0.6.6
          │   │       │   │   │   └── bingo v0.6.6 (*)
          │   │       │   │   ├── bingo-tool-fs v0.6.6
          │   │       │   │   │   └── bingo v0.6.6 (*)
          │   │       │   │   └── bingo-tool-web v0.6.6
          │   │       │   │       └── bingo v0.6.6 (*)
          │   │       │   ├── bingo-provider-anthropic v0.6.6 (*)
          │   │       │   ├── bingo-provider-openai v0.6.6 (*)
          │   │       │   ├── bingo-surface-tui v0.6.6 (*)
          │   │       │   └── bingo-tool-web v0.6.6 (*)
          │   │       ├── bingo-mcp v0.6.6 (*)
          │   │       ├── bingo-pictures v0.6.6 (*)
          │   │       ├── bingo-provider-anthropic v0.6.6 (*)
          │   │       ├── bingo-provider-openai v0.6.6 (*)
          │   │       ├── bingo-surface-tui v0.6.6 (*)
          │   │       ├── bingo-tool-web v0.6.6 (*)
          │   │       └── rmcp v3.1.4
          │   │           ├── (dev) bingo v0.6.6 (*)
          │   │           ├── bingo-mcp v0.6.6 (*)
          │   │           └── bingo-provider-acp v0.6.6
          │   │               └── bingo v0.6.6 (*)
          │   ├── reqwest v0.13.4 (*)
          │   ├── rustls-platform-verifier v0.7.0
          │   │   └── reqwest v0.13.4 (*)
          │   ├── tokio-rustls v0.26.4
          │   │   ├── hyper-rustls v0.27.9 (*)
          │   │   ├── reqwest v0.13.4 (*)
          │   │   └── tokio-tungstenite v0.30.0
          │   │       └── bingo-channel-feishu v0.6.6 (*)
          │   ├── tokio-tungstenite v0.30.0 (*)
          │   └── tungstenite v0.30.0
          │       └── tokio-tungstenite v0.30.0 (*)
          └── rustls-webpki v0.103.15
              ├── rustls v0.23.45 (*)
              └── rustls-platform-verifier v0.7.0 (*)
    ├ untrusted v0.9.0
      └── rustls-webpki v0.103.15
          ├── rustls v0.23.45
          │   ├── hyper-rustls v0.27.9
          │   │   └── reqwest v0.13.4
          │   │       ├── bingo v0.6.6
          │   │       ├── bingo-auth-oauth v0.6.6
          │   │       │   ├── bingo v0.6.6 (*)
          │   │       │   ├── bingo-channel-adapter v0.6.6
          │   │       │   │   ├── bingo v0.6.6 (*)
          │   │       │   │   ├── bingo-channel-feishu v0.6.6
          │   │       │   │   │   ├── bingo v0.6.6 (*)
          │   │       │   │   │   └── (dev) bingo-gateway v0.6.6
          │   │       │   │   │       └── bingo v0.6.6 (*)
          │   │       │   │   ├── bingo-channels v0.6.6
          │   │       │   │   │   ├── bingo v0.6.6 (*)
          │   │       │   │   │   └── bingo-gateway v0.6.6 (*)
          │   │       │   │   └── bingo-gateway v0.6.6 (*)
          │   │       │   ├── bingo-mcp v0.6.6
          │   │       │   │   └── bingo v0.6.6 (*)
          │   │       │   ├── bingo-provider-anthropic v0.6.6
          │   │       │   │   └── bingo v0.6.6 (*)
          │   │       │   └── bingo-provider-openai v0.6.6
          │   │       │       └── bingo v0.6.6 (*)
          │   │       ├── bingo-channel-feishu v0.6.6 (*)
          │   │       ├── bingo-http v0.6.6
          │   │       │   ├── bingo v0.6.6 (*)
          │   │       │   ├── (dev) bingo-auth-oauth v0.6.6 (*)
          │   │       │   ├── bingo-channel-feishu v0.6.6 (*)
          │   │       │   ├── bingo-mcp v0.6.6 (*)
          │   │       │   ├── bingo-pictures v0.6.6
          │   │       │   │   ├── (dev) bingo v0.6.6 (*)
          │   │       │   │   ├── bingo-channel-feishu v0.6.6 (*)
          │   │       │   │   ├── bingo-mcp v0.6.6 (*)
          │   │       │   │   ├── bingo-plugin-rpc v0.6.6
          │   │       │   │   │   └── bingo v0.6.6 (*)
          │   │       │   │   ├── bingo-surface-print v0.6.6
          │   │       │   │   │   └── bingo v0.6.6 (*)
          │   │       │   │   ├── bingo-surface-tui v0.6.6
          │   │       │   │   │   └── bingo v0.6.6 (*)
          │   │       │   │   ├── bingo-tool-fs v0.6.6
          │   │       │   │   │   └── bingo v0.6.6 (*)
          │   │       │   │   └── bingo-tool-web v0.6.6
          │   │       │   │       └── bingo v0.6.6 (*)
          │   │       │   ├── bingo-provider-anthropic v0.6.6 (*)
          │   │       │   ├── bingo-provider-openai v0.6.6 (*)
          │   │       │   ├── bingo-surface-tui v0.6.6 (*)
          │   │       │   └── bingo-tool-web v0.6.6 (*)
          │   │       ├── bingo-mcp v0.6.6 (*)
          │   │       ├── bingo-pictures v0.6.6 (*)
          │   │       ├── bingo-provider-anthropic v0.6.6 (*)
          │   │       ├── bingo-provider-openai v0.6.6 (*)
          │   │       ├── bingo-surface-tui v0.6.6 (*)
          │   │       ├── bingo-tool-web v0.6.6 (*)
          │   │       └── rmcp v3.1.4
          │   │           ├── (dev) bingo v0.6.6 (*)
          │   │           ├── bingo-mcp v0.6.6 (*)
          │   │           └── bingo-provider-acp v0.6.6
          │   │               └── bingo v0.6.6 (*)
          │   ├── reqwest v0.13.4 (*)
          │   ├── rustls-platform-verifier v0.7.0
          │   │   └── reqwest v0.13.4 (*)
          │   ├── tokio-rustls v0.26.4
          │   │   ├── hyper-rustls v0.27.9 (*)
          │   │   ├── reqwest v0.13.4 (*)
          │   │   └── tokio-tungstenite v0.30.0
          │   │       └── bingo-channel-feishu v0.6.6 (*)
          │   ├── tokio-tungstenite v0.30.0 (*)
          │   └── tungstenite v0.30.0
          │       └── tokio-tungstenite v0.30.0 (*)
          └── rustls-platform-verifier v0.7.0 (*)

warning[duplicate]: found 2 duplicate entries for crate 'winnow'
    ┌─ /Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/Cargo.lock:453:1
    │
453 │ ╭ winnow 0.7.15 registry+https://github.com/rust-lang/crates.io-index
454 │ │ winnow 1.0.4 registry+https://github.com/rust-lang/crates.io-index
    │ ╰──────────────────────────────────────────────────────────────────┘ lock entries
    │
    ├ winnow v0.7.15
      └── toml_edit v0.23.10+spec-1.0.0
          ├── (dev) bingo v0.6.6
          └── bingo-core v0.6.6
              ├── bingo v0.6.6 (*)
              └── bingo-gateway v0.6.6
                  └── bingo v0.6.6 (*)
    ├ winnow v1.0.4
      └── toml_parser v1.1.3+spec-1.1.0
          └── toml_edit v0.23.10+spec-1.0.0
              ├── (dev) bingo v0.6.6
              └── bingo-core v0.6.6
                  ├── bingo v0.6.6 (*)
                  └── bingo-gateway v0.6.6
                      └── bingo v0.6.6 (*)

advisories ok, bans ok, licenses ok, sources ok
```

</details>

<details>
<summary>windows-sdk — exit 0</summary>

```text
$ cargo check -p bingo-sdk --all-targets --target x86_64-pc-windows-msvc --locked
      ┌─────────┐  mr boxington
     ╱ ═══════ ╱│
    ╱─────────╱ │
    │ •    (◉)│≡│
    │  ◟▄▄▄◞  │ ╱
    │   ◀◆▶   │╱
    └─────────┘
    Checking cfg-if v1.0.4
    Checking windows-link v0.2.1
    Checking rand_core v0.10.1
   Compiling getrandom v0.4.3
   Compiling serde_core v1.0.229
    Checking memchr v2.8.3
    Checking windows-sys v0.61.2
   Compiling getrandom v0.3.4
    Checking pin-project-lite v0.2.17
    Checking futures-sink v0.3.34
    Checking futures-core v0.3.34
   Compiling serde v1.0.229
    Checking slab v0.4.12
    Checking futures-channel v0.3.34
    Checking futures-io v0.3.34
    Checking futures-task v0.3.34
   Compiling zerocopy v0.8.56
    Checking rand_core v0.9.5
   Compiling zmij v1.0.23
    Checking equivalent v1.0.2
    Checking futures-util v0.3.34
    Checking fastrand v2.5.0
    Checking hashbrown v0.17.1
    Checking once_cell v1.21.4
    Checking cpufeatures v0.3.1
   Compiling serde_json v1.0.151
    Checking indexmap v2.14.0
    Checking tempfile v3.27.0
    Checking chacha20 v0.10.2
   Compiling ref-cast v1.0.27
   Compiling libc v0.2.189
    Checking mio v1.2.2
    Checking itoa v1.0.18
    Checking bytes v1.12.1
    Checking jiff-tzdb v0.1.8
    Checking ppv-lite86 v0.2.21
   Compiling thiserror v2.0.20
    Checking tokio v1.53.1
    Checking jiff-tzdb-platform v0.1.3
    Checking rand v0.10.2
    Checking futures-executor v0.3.34
   Compiling num-traits v0.2.19
    Checking fnv v1.0.7
    Checking wait-timeout v0.2.1
    Checking bit-vec v0.8.0
    Checking jiff-core v0.1.0
    Checking quick-error v1.2.3
    Checking dyn-clone v1.0.20
    Checking encode_unicode v1.0.0
    Checking bit-set v0.8.0
    Checking rusty-fork v0.3.1
    Checking futures v0.3.34
    Checking console v0.16.4
    Checking tokio-util v0.7.19
    Checking rand_chacha v0.9.0
    Checking rand v0.9.5
    Checking rand_xorshift v0.4.0
    Checking regex-syntax v0.8.11
    Checking base64 v0.23.1
    Checking unarray v0.1.4
    Checking bitflags v2.13.1
    Checking similar v2.7.0
    Checking proptest v1.11.0
    Checking jiff v0.2.35
    Checking ulid v3.0.0
    Checking schemars v1.2.2
    Checking insta v1.48.0
    Checking bingo-sdk v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-sdk)
mbx[warning]: a flight prediction was not restored: failed to read compiler input /Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-sdk/src/agent.rs: No such file or directory (os error 2)
mbx[warning]: a flight prediction was not restored: failed to read compiler input /Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-sdk/fixtures/agent/edit.json: No such file or directory (os error 2)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.55s
mbx[cache]: 74 hits, 8 misses, 2 not looked up; 0 B downloaded, 0 B uploaded, 5.0 MiB stored locally
mbx[savings]: rustc arrived to find 74 compilations already done. awkward. (38s saved)
```

</details>

<details>
<summary>windows-core — exit 0</summary>

```text
$ cargo check -p bingo-core --all-targets --target x86_64-pc-windows-msvc --locked
      ┌─────────┐  mr boxington
     ╱ ═══════ ╱│
    ╱─────────╱ │
    │ •    (◉)│≡│
    │  ◟▄▄▄◞  │ ╱
    │   ◀◆▶   │╱
    └─────────┘
   Compiling serde_core v1.0.229
    Checking tokio v1.53.1
    Checking winnow v1.0.4
    Checking tracing-core v0.1.36
    Checking toml_writer v1.1.2+spec-1.1.0
    Checking winnow v0.7.15
    Checking tracing v0.1.44
    Checking toml_parser v1.1.3+spec-1.1.0
    Checking tokio-util v0.7.19
    Checking serde v1.0.229
    Checking serde_json v1.0.151
    Checking toml_datetime v0.7.5+spec-1.1.0
    Checking jiff v0.2.35
    Checking serde_spanned v1.1.1
    Checking toml_edit v0.23.10+spec-1.0.0
    Checking schemars v1.2.2
    Checking ulid v3.0.0
    Checking jsonc-parser v0.33.1
    Checking insta v1.48.0
    Checking bingo-sdk v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-sdk)
    Checking bingo-core v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-core)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.70s
mbx[cache]: 18 hits, 5 misses; 0 B downloaded, 0 B uploaded, 9.3 MiB stored locally
mbx[savings]: cleaned up 1.0 TiB logical after checkouts that left without saying goodbye.
```

</details>

<details>
<summary>windows-services — exit 0</summary>

```text
$ cargo check -p bingo-services --all-targets --target x86_64-pc-windows-msvc --locked
      ┌─────────┐  mr boxington
     ╱ ═══════ ╱│
    ╱─────────╱ │
    │ •    (◉)│≡│
    │  ◟▄▄▄◞  │ ╱
    │   ◀◆▶   │╱
    └─────────┘
    Checking windows-sys v0.61.2
    Checking mio v1.2.2
    Checking tokio v1.53.1
    Checking tokio-util v0.7.19
    Checking bingo-sdk v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-sdk)
mbx[warning]: a flight prediction was not restored: failed to read compiler input /Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-sdk/src/agent.rs: No such file or directory (os error 2)
    Checking bingo-services v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-services)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.30s
mbx[cache]: 4 hits, 0 misses, 3 not looked up; 0 B downloaded, 0 B uploaded, 5.4 MiB stored locally
mbx[savings]: worktrees come and go. their 1.0 TiB logical now goes with them.
```

</details>

<details>
<summary>windows-plugin-rpc — exit 101</summary>

```text
$ cargo check -p bingo-plugin-rpc --all-targets --target x86_64-pc-windows-msvc --locked
      ┌─────────┐  mr boxington
     ╱ ═══════ ╱│
    ╱─────────╱ │
    │ •    (◉)│≡│
    │  ◟▄▄▄◞  │ ╱
    │   ◀◆▶   │╱
    └─────────┘
    Checking windows-sys v0.61.2
    Checking futures-channel v0.3.34
    Checking slab v0.4.12
    Checking stable_deref_trait v1.2.1
    Checking zerofrom v0.1.8
    Checking windows-result v0.4.1
    Checking windows-strings v0.5.1
   Compiling aws-lc-sys v0.44.0
    Checking futures-util v0.3.34
    Checking yoke v0.8.3
    Checking litemap v0.8.3
    Checking socket2 v0.6.5
    Checking mio v1.2.2
    Checking zerovec v0.11.8
    Checking writeable v0.6.4
    Checking smallvec v1.15.2
    Checking zerotrie v0.2.5
    Checking tokio v1.53.1
    Checking http v1.5.0
    Checking tinystr v0.8.4
    Checking potential_utf v0.1.6
    Checking utf8_iter v1.0.4
    Checking zeroize v1.9.0
    Checking icu_locale_core v2.3.0
   Compiling icu_normalizer_data v2.3.0
    Checking icu_collections v2.3.0
   Compiling icu_properties_data v2.3.0
    Checking tracing v0.1.44
    Checking tokio-util v0.7.19
    Checking icu_provider v2.3.1
    Checking serde v1.0.229
    Checking rustls-pki-types v1.15.1
    Checking http-body v1.1.0
    Checking untrusted v0.9.0
    Checking percent-encoding v2.3.2
    Checking simd-adler32 v0.3.10
   Compiling httparse v1.10.1
    Checking icu_properties v2.3.0
    Checking icu_normalizer v2.3.0
    Checking futures-executor v0.3.34
   Compiling windows-interface v0.59.3
   Compiling windows-implement v0.60.2
    Checking tower-service v0.3.3
    Checking try-lock v0.2.5
    Checking subtle v2.6.1
    Checking atomic-waker v1.1.2
    Checking adler2 v2.0.1
    Checking h2 v0.4.19
   Compiling crc32fast v1.5.1
    Checking want v0.3.1
    Checking futures v0.3.34
    Checking idna_adapter v1.2.2
    Checking thiserror v2.0.20
    Checking windows-core v0.62.2
   Compiling zerocopy v0.8.56
    Checking hyper v1.11.1
    Checking idna v1.1.0
    Checking miniz_oxide v0.9.1
    Checking form_urlencoded v1.2.2
    Checking windows-registry v0.6.1
    Checking sync_wrapper v1.0.2
    Checking tower-layer v0.3.3
    Checking base64 v0.22.1
    Checking ipnet v2.12.1
    Checking tower v0.5.3
    Checking url v2.5.8
    Checking hyper-util v0.1.20
    Checking ref-cast v1.0.27
    Checking flate2 v1.1.10
   Compiling num-traits v0.2.19
    Checking windows-threading v0.2.1
    Checking zune-core v0.5.3
    Checking weezl v0.1.12
    Checking log v0.4.34
    Checking quick-error v2.0.1
    Checking zune-jpeg v0.5.15
    Checking windows-future v0.3.2
    Checking ulid v3.0.0
    Checking half v2.7.1
    Checking tower-http v0.6.11
    Checking windows-collections v0.3.2
    Checking windows-numerics v0.3.1
    Checking miniz_oxide v0.8.9
    Checking fdeflate v0.3.7
    Checking http-body-util v0.1.5
    Checking encoding_rs v0.8.35
    Checking mime v0.3.17
    Checking fax v0.2.7
    Checking color_quant v1.1.0
    Checking byteorder-lite v0.1.0
    Checking pxfm v0.1.30
    Checking schemars v1.2.2
    Checking bytemuck v1.25.2
    Checking gif v0.14.2
    Checking image-webp v0.2.4
    Checking tiff v0.11.3
    Checking qoi v0.4.1
    Checking bingo-sdk v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-sdk)
    Checking png v0.18.1
    Checking moxcms v0.8.1
    Checking windows v0.62.2
    Checking similar v3.2.0
    Checking bingo-jsonrpc v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-jsonrpc)
    Checking bingo-shelf v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-shelf)
    Checking process-wrap v10.0.0
    Checking image v0.25.10
    Checking tempfile v3.27.0
    Checking bingo-schema-ts v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-schema-ts)
    Checking bingo-process v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-process)
warning: aws-lc-sys@0.44.0: Environment Variable found 'CARGO_ENCODED_RUSTFLAGS': ''
warning: aws-lc-sys@0.44.0: Emitting configuration: cargo:rustc-cfg=universal
warning: aws-lc-sys@0.44.0: Building with: CC
warning: aws-lc-sys@0.44.0: Symbol Prefix: Some("aws_lc_0_44_0")
warning: aws-lc-sys@0.44.0: Target platform: 'x86_64-pc-windows-msvc'
warning: aws-lc-sys@0.44.0: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/tests/compiler_features_tests/c11.c:7:10: fatal error: 'stdlib.h' file not found
warning: aws-lc-sys@0.44.0:     7 | #include <stdlib.h>
warning: aws-lc-sys@0.44.0:       |          ^~~~~~~~~~
warning: aws-lc-sys@0.44.0: 1 error generated.
warning: aws-lc-sys@0.44.0: Compilation of 'c11.c' failed - Err(Error { kind: ToolExecError, message: "command did not execute successfully (status code exit status: 1): LC_ALL=\"C\" \"cc\" \"-O0\" \"-ffunction-sections\" \"-fdata-sections\" \"-m64\" \"--target=x86_64-pc-windows-msvc\" \"-Werror\" \"-Wall\" \"-Wextra\" \"-Wno-unused-parameter\" \"-o\" \"/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/out-c11/7dfda64fdf5a526c-c11.o\" \"-c\" \"/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/tests/compiler_features_tests/c11.c\"" }).
warning: aws-lc-sys@0.44.0: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/tests/compiler_features_tests/stdalign_check.c:10:10: fatal error: 'stdlib.h' file not found
warning: aws-lc-sys@0.44.0:    10 | #include <stdlib.h>
warning: aws-lc-sys@0.44.0:       |          ^~~~~~~~~~
warning: aws-lc-sys@0.44.0: 1 error generated.
warning: aws-lc-sys@0.44.0: Compilation of 'stdalign_check.c' failed - Err(Error { kind: ToolExecError, message: "command did not execute successfully (status code exit status: 1): LC_ALL=\"C\" \"cc\" \"-O0\" \"-ffunction-sections\" \"-fdata-sections\" \"-m64\" \"--target=x86_64-pc-windows-msvc\" \"-Werror\" \"-Wall\" \"-Wextra\" \"-Wno-unused-parameter\" \"-o\" \"/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/out-stdalign_check/7dfda64fdf5a526c-stdalign_check.o\" \"-c\" \"/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/tests/compiler_features_tests/stdalign_check.c\"" }).
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-base.c:33:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-gcd.h:23:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
warning: aws-lc-sys@0.44.0: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
warning: aws-lc-sys@0.44.0:    49 | #include <windows.h>
warning: aws-lc-sys@0.44.0:       |          ^~~~~~~~~~~
warning: aws-lc-sys@0.44.0: 1 error generated.
warning: aws-lc-sys@0.44.0: ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-base.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-base.c"cargo:rerun-if-env-changed=VCINSTALLDIR
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-gcd.c:22:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-gcd.h:23:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
warning: aws-lc-sys@0.44.0: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
warning: aws-lc-sys@0.44.0:    49 | #include <windows.h>
warning: aws-lc-sys@0.44.0:       |          ^~~~~~~~~~~
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-health.c:22:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-health.h:23:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
warning: aws-lc-sys@0.44.0: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
warning: aws-lc-sys@0.44.0:    49 | #include <windows.h>
warning: aws-lc-sys@0.44.0:       |          ^~~~~~~~~~~
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-noise.c:21:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-noise.h:23:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
warning: aws-lc-sys@0.44.0: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
warning: aws-lc-sys@0.44.0:    49 | #include <windows.h>
warning: aws-lc-sys@0.44.0:       |          ^~~~~~~~~~~
warning: aws-lc-sys@0.44.0: 1 error generated.
warning: aws-lc-sys@0.44.0: ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-gcd.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-gcd.c"cargo:warning=1 error generated.
warning: aws-lc-sys@0.44.0: ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-health.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-health.c"cargo:warning=1 error generated.
warning: aws-lc-sys@0.44.0: ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-noise.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-noise.c"cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-sha3.c:21:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-sha3.h:23:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
warning: aws-lc-sys@0.44.0: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
warning: aws-lc-sys@0.44.0:    49 | #include <windows.h>
warning: aws-lc-sys@0.44.0:       |          ^~~~~~~~~~~
warning: aws-lc-sys@0.44.0: 1 error generated.
warning: aws-lc-sys@0.44.0: ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-sha3.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-sha3.c"cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-timer.c:22:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-timer.h:23:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
warning: aws-lc-sys@0.44.0: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
warning: aws-lc-sys@0.44.0:    49 | #include <windows.h>
warning: aws-lc-sys@0.44.0:       |          ^~~~~~~~~~~
warning: aws-lc-sys@0.44.0: 1 error generated.
warning: aws-lc-sys@0.44.0: ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-timer.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-timer.c"
error: failed to run custom build command for `aws-lc-sys v0.44.0`
note: To improve backtraces for build dependencies, set the CARGO_PROFILE_DEV_BUILD_OVERRIDE_DEBUG=true environment variable to enable debug information generation.

Caused by:
  process didn't exit successfully: `/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/debug/build/aws-lc-sys-6ad9a570e9d6c662/build-script-main` (exit status: 1)
  --- stdout
  cargo:rerun-if-env-changed=AWS_LC_SYS_NO_PREFIX_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_NO_PREFIX
  cargo:rerun-if-env-changed=AWS_LC_SYS_PREGENERATING_BINDINGS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_PREGENERATING_BINDINGS
  cargo:rerun-if-env-changed=AWS_LC_SYS_EXTERNAL_BINDGEN_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_EXTERNAL_BINDGEN
  cargo:rerun-if-env-changed=AWS_LC_SYS_NO_ASM_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_NO_ASM
  cargo:rerun-if-env-changed=AWS_LC_SYS_PREBUILT_NASM_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_PREBUILT_NASM
  cargo:rerun-if-env-changed=AWS_LC_SYS_C_STD_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_C_STD
  cargo:rerun-if-env-changed=AWS_LC_SYS_CMAKE_BUILDER_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_CMAKE_BUILDER
  cargo:rerun-if-env-changed=AWS_LC_SYS_NO_PREGENERATED_SRC_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_NO_PREGENERATED_SRC
  cargo:rerun-if-env-changed=AWS_LC_SYS_SMALL_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_SMALL
  cargo:rerun-if-env-changed=AWS_LC_SYS_EFFECTIVE_TARGET_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_EFFECTIVE_TARGET
  cargo:rerun-if-env-changed=AWS_LC_SYS_NO_JITTER_ENTROPY_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_NO_JITTER_ENTROPY
  cargo:rerun-if-env-changed=AWS_LC_SYS_NO_U1_BINDINGS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_NO_U1_BINDINGS
  cargo:rerun-if-env-changed=AWS_LC_SYS_INCLUDES_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_INCLUDES
  cargo:rerun-if-env-changed=AWS_LC_SYS_SANITIZER_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_SANITIZER
  cargo:rerun-if-env-changed=AWS_LC_SYS_LINK_WHOLE_ARCHIVE_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_LINK_WHOLE_ARCHIVE
  cargo:rerun-if-env-changed=AWS_LC_SYS_STATIC_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_STATIC
  cargo:rerun-if-env-changed=AWS_LC_SYS_SYSTEM_DIR_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_SYSTEM_DIR
  cargo:rerun-if-env-changed=AWS_LC_SYS_USE_SYSTEM_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_USE_SYSTEM
  cargo:rerun-if-env-changed=AWS_LC_SYS_SYSTEM_BINDINGS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_SYSTEM_BINDINGS
  cargo:rerun-if-env-changed=AWS_LC_SYS_SYSTEM_SKIP_VERSION_CHECK_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_SYSTEM_SKIP_VERSION_CHECK
  cargo:rerun-if-env-changed=CARGO_ENCODED_RUSTFLAGS
  cargo:warning=Environment Variable found 'CARGO_ENCODED_RUSTFLAGS': ''
  cargo:warning=Emitting configuration: cargo:rustc-cfg=universal
  cargo:rustc-cfg=universal
  cargo:rerun-if-env-changed=OPENSSL_DIR_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=OPENSSL_DIR
  cargo:rerun-if-env-changed=OPENSSL_INCLUDE_DIR_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=OPENSSL_INCLUDE_DIR
  cargo:rerun-if-env-changed=OPENSSL_LIB_DIR_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=OPENSSL_LIB_DIR
  cargo:rerun-if-env-changed=OPENSSL_NO_PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG_ALLOW_CROSS
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS
  cargo:rerun-if-env-changed=PKG_CONFIG_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG_SYSROOT_DIR
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR
  cargo:rerun-if-env-changed=AWS_LC_NO_PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG_ALLOW_CROSS
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS
  cargo:rerun-if-env-changed=PKG_CONFIG_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG_SYSROOT_DIR
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR
  cargo:rerun-if-env-changed=LIBCRYPTO_NO_PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG_ALLOW_CROSS
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS
  cargo:rerun-if-env-changed=PKG_CONFIG_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG_SYSROOT_DIR
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR
  cargo:rerun-if-env-changed=LIBCRYPTO_AWSLC_NO_PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG_ALLOW_CROSS
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS
  cargo:rerun-if-env-changed=PKG_CONFIG_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG_SYSROOT_DIR
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR
  cargo:rerun-if-env-changed=CARGO_FEATURE_SSL
  cargo:warning=Building with: CC
  cargo:warning=Symbol Prefix: Some("aws_lc_0_44_0")
  cargo:rerun-if-env-changed=CARGO_FEATURE_SSL
  cargo:warning=Target platform: 'x86_64-pc-windows-msvc'
  cargo:root=/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out
  cargo:rerun-if-env-changed=AWS_LC_SYS_TARGET_CFLAGS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_TARGET_CFLAGS
  cargo:rerun-if-env-changed=TARGET_CFLAGS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  cargo:rerun-if-env-changed=AWS_LC_SYS_CFLAGS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_CFLAGS
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=CFLAGS
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CC_ENABLE_DEBUG_OUTPUT
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CC_ENABLE_DEBUG_OUTPUT
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CC_ENABLE_DEBUG_OUTPUT
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_FORCE_DISABLE
  CC_FORCE_DISABLE = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:warning=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/tests/compiler_features_tests/c11.c:7:10: fatal error: 'stdlib.h' file not found
  cargo:warning=    7 | #include <stdlib.h>
  cargo:warning=      |          ^~~~~~~~~~
  cargo:warning=1 error generated.
  cargo:warning=Compilation of 'c11.c' failed - Err(Error { kind: ToolExecError, message: "command did not execute successfully (status code exit status: 1): LC_ALL=\"C\" \"cc\" \"-O0\" \"-ffunction-sections\" \"-fdata-sections\" \"-m64\" \"--target=x86_64-pc-windows-msvc\" \"-Werror\" \"-Wall\" \"-Wextra\" \"-Wno-unused-parameter\" \"-o\" \"/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/out-c11/7dfda64fdf5a526c-c11.o\" \"-c\" \"/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/tests/compiler_features_tests/c11.c\"" }).
  cargo:rerun-if-env-changed=AWS_LC_SYS_TARGET_CC_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_TARGET_CC
  cargo:rerun-if-env-changed=TARGET_CC_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_CC
  cargo:rerun-if-env-changed=AWS_LC_SYS_CC_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_CC
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=CC
  cargo:rerun-if-env-changed=AWS_LC_SYS_TARGET_CXX_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_TARGET_CXX
  cargo:rerun-if-env-changed=TARGET_CXX_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_CXX
  cargo:rerun-if-env-changed=AWS_LC_SYS_CXX_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_CXX
  cargo:rerun-if-env-changed=CXX_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=CXX
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CC_ENABLE_DEBUG_OUTPUT
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_FORCE_DISABLE
  CC_FORCE_DISABLE = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:warning=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/tests/compiler_features_tests/stdalign_check.c:10:10: fatal error: 'stdlib.h' file not found
  cargo:warning=   10 | #include <stdlib.h>
  cargo:warning=      |          ^~~~~~~~~~
  cargo:warning=1 error generated.
  cargo:warning=Compilation of 'stdalign_check.c' failed - Err(Error { kind: ToolExecError, message: "command did not execute successfully (status code exit status: 1): LC_ALL=\"C\" \"cc\" \"-O0\" \"-ffunction-sections\" \"-fdata-sections\" \"-m64\" \"--target=x86_64-pc-windows-msvc\" \"-Werror\" \"-Wall\" \"-Wextra\" \"-Wno-unused-parameter\" \"-o\" \"/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/out-stdalign_check/7dfda64fdf5a526c-stdalign_check.o\" \"-c\" \"/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/tests/compiler_features_tests/stdalign_check.c\"" }).
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CC_ENABLE_DEBUG_OUTPUT
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_FORCE_DISABLE
  CC_FORCE_DISABLE = None
  cargo:rerun-if-env-changed=CC_FORCE_DISABLE
  CC_FORCE_DISABLE = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-base.c:33:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-gcd.h:23:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
  cargo:warning=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
  cargo:warning=   49 | #include <windows.h>
  cargo:warning=      |          ^~~~~~~~~~~
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:warning=1 error generated.
  exit status: 1
  cargo:warning=ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-base.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-base.c"cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-gcd.c:22:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-gcd.h:23:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
  cargo:warning=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
  cargo:warning=   49 | #include <windows.h>
  cargo:warning=      |          ^~~~~~~~~~~
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-health.c:22:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-health.h:23:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
  cargo:warning=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
  cargo:warning=   49 | #include <windows.h>
  cargo:warning=      |          ^~~~~~~~~~~
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-noise.c:21:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-noise.h:23:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
  cargo:warning=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
  cargo:warning=   49 | #include <windows.h>
  cargo:warning=      |          ^~~~~~~~~~~
  cargo:warning=1 error generated.

  exit status: 1
  cargo:warning=ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-gcd.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-gcd.c"cargo:warning=1 error generated.

  exit status: 1
  cargo:warning=ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-health.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-health.c"cargo:warning=1 error generated.

  exit status: 1
  cargo:warning=ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-noise.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-noise.c"cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-sha3.c:21:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-sha3.h:23:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
  cargo:warning=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
  cargo:warning=   49 | #include <windows.h>
  cargo:warning=      |          ^~~~~~~~~~~
  cargo:warning=1 error generated.

  exit status: 1
  cargo:warning=ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-sha3.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-sha3.c"cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-timer.c:22:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-timer.h:23:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
  cargo:warning=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
  cargo:warning=   49 | #include <windows.h>
  cargo:warning=      |          ^~~~~~~~~~~
  cargo:warning=1 error generated.

  exit status: 1
  cargo:warning=ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-timer.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-timer.c"

  --- stderr


  error occurred in cc-rs: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-timer.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-timer.c"


warning: build failed, waiting for other jobs to finish...
mbx[cache]: 110 hits, 4 misses; 0 B downloaded, 0 B uploaded, 5.2 MiB stored locally
mbx[savings]: cargo asked for 110 compilations; mbx had them filed under already-done. (54s saved)
```

</details>

<details>
<summary>windows-bin — exit 101</summary>

```text
$ cargo check -p bingo --all-targets --target x86_64-pc-windows-msvc --locked
      ┌─────────┐  mr boxington
     ╱ ═══════ ╱│
    ╱─────────╱ │
    │ •    (◉)│≡│
    │  ◟▄▄▄◞  │ ╱
    │   ◀◆▶   │╱
    └─────────┘
    Checking windows-sys v0.61.2
    Checking foldhash v0.2.0
    Checking allocator-api2 v0.2.21
    Checking serde v1.0.229
    Checking base64 v0.23.1
    Checking stable_deref_trait v1.2.1
    Checking bitflags v2.13.1
    Checking hashbrown v0.17.1
    Checking mio v1.2.2
    Checking socket2 v0.6.5
    Checking yoke v0.8.3
    Checking indexmap v2.14.0
   Compiling aws-lc-sys v0.44.0
    Checking zerovec v0.11.8
    Checking serde_json v1.0.151
    Checking tokio v1.53.1
   Compiling libc v0.2.189
    Checking tinystr v0.8.4
    Checking potential_utf v0.1.6
    Checking zerotrie v0.2.5
    Checking icu_locale_core v2.3.0
    Checking untrusted v0.7.1
    Checking tokio-util v0.7.19
    Checking icu_collections v2.3.0
    Checking icu_provider v2.3.1
    Checking httpdate v1.0.3
    Checking tower v0.5.3
    Checking icu_properties v2.3.0
    Checking icu_normalizer v2.3.0
    Checking h2 v0.4.19
   Compiling parking_lot_core v0.9.12
    Checking aho-corasick v1.1.5
    Checking idna_adapter v1.2.2
    Checking scopeguard v1.2.0
    Checking hyper v1.11.1
    Checking regex-automata v0.4.18
    Checking idna v1.1.0
    Checking lock_api v0.4.14
    Checking hyper-util v0.1.20
    Checking url v2.5.8
    Checking siphasher v1.0.3
    Checking tower-http v0.6.11
    Checking parking_lot v0.12.5
    Checking phf_shared v0.13.1
    Checking new_debug_unreachable v1.0.6
    Checking precomputed-hash v0.1.1
    Checking phf v0.13.1
    Checking unicode-width v0.2.2
   Compiling web_atoms v0.2.6
    Checking derive_more v2.1.1
    Checking tendril v0.5.1
    Checking bingo-loopback v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-loopback)
   Compiling winapi v0.3.9
    Checking anstyle v1.0.14
    Checking png v0.18.1
    Checking unicode-segmentation v1.13.3
    Checking bit-vec v0.8.0
    Checking image v0.25.10
    Checking typenum v1.20.1
    Checking bit-set v0.8.0
    Checking chrono v0.4.45
    Checking ulid v3.0.0
    Checking string_cache v0.9.0
    Checking hybrid-array v0.4.14
    Checking schemars v1.2.2
    Checking strum v0.28.0
    Checking process-wrap v10.0.0
    Checking bingo-sdk v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-sdk)
    Checking bstr v1.13.1
mbx[warning]: build-script result was not stored: local action key already has a different result
    Checking winapi-util v0.1.11
    Checking either v1.18.0
    Checking lazy_static v1.5.0
    Checking itertools v0.14.0
   Compiling tree-sitter-language v0.1.7
   Compiling crossbeam-utils v0.8.22
    Checking same-file v1.0.6
    Checking castaway v0.2.4
    Checking regex v1.13.1
    Checking encoding_rs_io v0.1.8
    Checking hashbrown v0.16.1
    Checking static_assertions v1.1.0
    Checking dtoa v1.0.11
    Checking ryu v1.0.23
    Checking dtoa-short v0.3.5
    Checking compact_str v0.9.1
   Compiling crossbeam-epoch v0.9.20
    Checking kasuari v0.4.12
    Checking walkdir v2.5.0
    Checking unicode-truncate v2.0.1
    Checking bingo-shelf v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-shelf)
    Checking bingo-process v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-process)
    Checking block-buffer v0.12.1
    Checking crypto-common v0.2.2
    Checking lru v0.18.3
    Checking arraydeque v0.5.1
    Checking critical-section v1.2.0
    Checking const-oid v0.10.2
    Checking utf8parse v0.2.2
    Checking once_cell_polyfill v1.70.2
    Checking anstyle-parse v1.0.0
    Checking anstyle-wincon v3.0.11
    Checking digest v0.11.3
    Checking ratatui-core v0.1.2
   Compiling crossbeam-deque v0.8.7
   Compiling anyhow v1.0.104
    Checking granit-parser v1.1.0
    Checking cssparser v0.37.0
   Compiling selectors v0.38.0
    Checking globset v0.4.20
    Checking annotate-snippets v0.12.16
    Checking servo_arc v0.4.3
    Checking anstyle-query v1.1.5
    Checking bingo-claim v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-claim)
    Checking rustc-hash v2.1.3
    Checking colorchoice v1.0.5
    Checking is_terminal_polyfill v1.70.2
   Compiling rmcp v3.1.4
    Checking anstream v1.0.0
    Checking markup5ever v0.38.0
    Checking markup5ever v0.39.0
    Checking xml5ever v0.38.0
    Checking html5ever v0.38.0
    Checking html5ever v0.39.0
    Checking serde-saphyr v1.1.0
    Checking sha1 v0.11.0
    Checking fancy-regex v0.16.2
    Checking bincode v1.3.3
    Checking bingo-jsonrpc v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-jsonrpc)
    Checking process-wrap v9.1.0
    Checking sse-stream v0.2.5
    Checking tokio-stream v0.1.19
    Checking line-clipping v0.3.8
    Checking schannel v0.1.29
    Checking uuid v1.26.0
    Checking grep-matcher v0.1.9
    Checking nom v8.0.0
    Checking strsim v0.11.1
    Checking data-encoding v2.11.1
    Checking bingo-digest v0.6.6 (/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/crates/bingo-digest)
    Checking clap_lex v1.1.0
   Compiling pulldown-cmark v0.13.4
    Checking clap_builder v4.6.6
    Checking toml_edit v0.23.10+spec-1.0.0
   Compiling thiserror v1.0.69
    Checking rustls-native-certs v0.8.4
   Compiling tree-sitter-bash v0.25.1
    Checking ratatui-widgets v0.3.2
    Checking syntect v5.3.0
   Compiling tree-sitter v0.26.13
    Checking crossterm_winapi v0.9.1
    Checking crossterm v0.29.0
    Checking dom_query v0.28.0
    Checking ratatui-crossterm v0.1.2
warning: tree-sitter@0.26.13: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/lib.c:1:
warning: tree-sitter@0.26.13: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/./alloc.c:1:
warning: tree-sitter@0.26.13: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/./alloc.h:9:10: fatal error: 'stdio.h' file not found
warning: tree-sitter@0.26.13:     9 | #include <stdio.h>
warning: tree-sitter@0.26.13:       |          ^~~~~~~~~
warning: tree-sitter@0.26.13: 1 error generated.
error: failed to run custom build command for `tree-sitter v0.26.13`
note: To improve backtraces for build dependencies, set the CARGO_PROFILE_DEV_BUILD_OVERRIDE_DEBUG=true environment variable to enable debug information generation.

Caused by:
  process didn't exit successfully: `/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/debug/build/tree-sitter-cb5c32b0d0e653cc/build-script-build` (exit status: 1)
  --- stdout
  cargo:rerun-if-env-changed=CARGO_FEATURE_WASM
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/error_costs.h
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/lib.c
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/subtree.h
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/reusable_node.h
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/host.h
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/point.h
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/unicode
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/stack.c
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/parser.h
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/tree_cursor.h
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/portable
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/language.h
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/wasm
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/lexer.c
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/wasm_store.h
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/tree.h
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/get_changed_ranges.c
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/alloc.c
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/unicode.h
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/array.h
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/query.c
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/stack.h
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/point.c
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/subtree.c
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/length.h
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/ts_assert.h
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/lexer.h
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/language.c
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/reduce_action.h
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/tree_cursor.c
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/parser.c
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/alloc.h
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/node.c
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/get_changed_ranges.h
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/wasm_store.c
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/tree.c
  cargo:rerun-if-changed=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/atomic.h
  cargo:rerun-if-env-changed=CC_FORCE_DISABLE
  CC_FORCE_DISABLE = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CC_ENABLE_DEBUG_OUTPUT
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/lib.c:1:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/./alloc.c:1:
  cargo:warning=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/./alloc.h:9:10: fatal error: 'stdio.h' file not found
  cargo:warning=    9 | #include <stdio.h>
  cargo:warning=      |          ^~~~~~~~~
  cargo:warning=1 error generated.

  --- stderr


  error occurred in cc-rs: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-std=c11" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/wasm" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/include" "-w" "-fvisibility=hidden" "-Wshadow" "-Wno-unused-parameter" "-Wno-incompatible-pointer-types" "-D_POSIX_C_SOURCE=200112L" "-D_DEFAULT_SOURCE" "-D_BSD_SOURCE" "-D_DARWIN_C_SOURCE" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/tree-sitter-de3d4fd4fd009015/out/98490c8781b409d2-lib.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tree-sitter-0.26.13/src/lib.c"


warning: build failed, waiting for other jobs to finish...
warning: tree-sitter-bash@0.25.1: In file included from src/parser.c:3:
warning: tree-sitter-bash@0.25.1: src/tree_sitter/parser.h:10:10: fatal error: 'stdlib.h' file not found
warning: tree-sitter-bash@0.25.1:    10 | #include <stdlib.h>
warning: tree-sitter-bash@0.25.1:       |          ^~~~~~~~~~
warning: tree-sitter-bash@0.25.1: 1 error generated.
warning: tree-sitter-bash@0.25.1: ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-std=c11" "-I" "src" "-Wall" "-Wextra" "-Wno-unused-value" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/tree-sitter-bash-bb3a62c3386af739/out/ea708c7824d36062-parser.o" "-c" "src/parser.c"cargo:warning=In file included from src/scanner.c:1:
warning: tree-sitter-bash@0.25.1: In file included from src/tree_sitter/array.h:8:
warning: tree-sitter-bash@0.25.1: src/tree_sitter/./alloc.h:9:10: fatal error: 'stdio.h' file not found
warning: tree-sitter-bash@0.25.1:     9 | #include <stdio.h>
warning: tree-sitter-bash@0.25.1:       |          ^~~~~~~~~
warning: tree-sitter-bash@0.25.1: 1 error generated.
warning: tree-sitter-bash@0.25.1: ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-std=c11" "-I" "src" "-Wall" "-Wextra" "-Wno-unused-value" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/tree-sitter-bash-bb3a62c3386af739/out/ea708c7824d36062-scanner.o" "-c" "src/scanner.c"
error: failed to run custom build command for `tree-sitter-bash v0.25.1`
note: To improve backtraces for build dependencies, set the CARGO_PROFILE_DEV_BUILD_OVERRIDE_DEBUG=true environment variable to enable debug information generation.

Caused by:
  process didn't exit successfully: `/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/debug/build/tree-sitter-bash-ef1920d386c8c4c0/build-script-build` (exit status: 1)
  --- stdout
  cargo:rerun-if-changed=src/parser.c
  cargo:rerun-if-changed=src/scanner.c
  cargo:rerun-if-env-changed=CC_FORCE_DISABLE
  CC_FORCE_DISABLE = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CC_ENABLE_DEBUG_OUTPUT
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:warning=In file included from src/parser.c:3:
  cargo:warning=src/tree_sitter/parser.h:10:10: fatal error: 'stdlib.h' file not found
  cargo:warning=   10 | #include <stdlib.h>
  cargo:warning=      |          ^~~~~~~~~~
  cargo:warning=1 error generated.
  exit status: 1
  cargo:warning=ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-std=c11" "-I" "src" "-Wall" "-Wextra" "-Wno-unused-value" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/tree-sitter-bash-bb3a62c3386af739/out/ea708c7824d36062-parser.o" "-c" "src/parser.c"cargo:warning=In file included from src/scanner.c:1:
  cargo:warning=In file included from src/tree_sitter/array.h:8:
  cargo:warning=src/tree_sitter/./alloc.h:9:10: fatal error: 'stdio.h' file not found
  cargo:warning=    9 | #include <stdio.h>
  cargo:warning=      |          ^~~~~~~~~
  cargo:warning=1 error generated.

  exit status: 1
  cargo:warning=ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-std=c11" "-I" "src" "-Wall" "-Wextra" "-Wno-unused-value" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/tree-sitter-bash-bb3a62c3386af739/out/ea708c7824d36062-scanner.o" "-c" "src/scanner.c"

  --- stderr


  error occurred in cc-rs: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-std=c11" "-I" "src" "-Wall" "-Wextra" "-Wno-unused-value" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/tree-sitter-bash-bb3a62c3386af739/out/ea708c7824d36062-scanner.o" "-c" "src/scanner.c"


warning: aws-lc-sys@0.44.0: Environment Variable found 'CARGO_ENCODED_RUSTFLAGS': ''
warning: aws-lc-sys@0.44.0: Emitting configuration: cargo:rustc-cfg=universal
warning: aws-lc-sys@0.44.0: Building with: CC
warning: aws-lc-sys@0.44.0: Symbol Prefix: Some("aws_lc_0_44_0")
warning: aws-lc-sys@0.44.0: Target platform: 'x86_64-pc-windows-msvc'
warning: aws-lc-sys@0.44.0: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/tests/compiler_features_tests/c11.c:7:10: fatal error: 'stdlib.h' file not found
warning: aws-lc-sys@0.44.0:     7 | #include <stdlib.h>
warning: aws-lc-sys@0.44.0:       |          ^~~~~~~~~~
warning: aws-lc-sys@0.44.0: 1 error generated.
warning: aws-lc-sys@0.44.0: Compilation of 'c11.c' failed - Err(Error { kind: ToolExecError, message: "command did not execute successfully (status code exit status: 1): LC_ALL=\"C\" \"cc\" \"-O0\" \"-ffunction-sections\" \"-fdata-sections\" \"-m64\" \"--target=x86_64-pc-windows-msvc\" \"-Werror\" \"-Wall\" \"-Wextra\" \"-Wno-unused-parameter\" \"-o\" \"/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/out-c11/7dfda64fdf5a526c-c11.o\" \"-c\" \"/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/tests/compiler_features_tests/c11.c\"" }).
warning: aws-lc-sys@0.44.0: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/tests/compiler_features_tests/stdalign_check.c:10:10: fatal error: 'stdlib.h' file not found
warning: aws-lc-sys@0.44.0:    10 | #include <stdlib.h>
warning: aws-lc-sys@0.44.0:       |          ^~~~~~~~~~
warning: aws-lc-sys@0.44.0: 1 error generated.
warning: aws-lc-sys@0.44.0: Compilation of 'stdalign_check.c' failed - Err(Error { kind: ToolExecError, message: "command did not execute successfully (status code exit status: 1): LC_ALL=\"C\" \"cc\" \"-O0\" \"-ffunction-sections\" \"-fdata-sections\" \"-m64\" \"--target=x86_64-pc-windows-msvc\" \"-Werror\" \"-Wall\" \"-Wextra\" \"-Wno-unused-parameter\" \"-o\" \"/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/out-stdalign_check/7dfda64fdf5a526c-stdalign_check.o\" \"-c\" \"/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/tests/compiler_features_tests/stdalign_check.c\"" }).
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-base.c:33:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-gcd.h:23:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
warning: aws-lc-sys@0.44.0: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
warning: aws-lc-sys@0.44.0:    49 | #include <windows.h>
warning: aws-lc-sys@0.44.0:       |          ^~~~~~~~~~~
warning: aws-lc-sys@0.44.0: 1 error generated.
warning: aws-lc-sys@0.44.0: ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-base.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-base.c"cargo:rerun-if-env-changed=VCINSTALLDIR
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-gcd.c:22:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-gcd.h:23:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
warning: aws-lc-sys@0.44.0: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
warning: aws-lc-sys@0.44.0:    49 | #include <windows.h>
warning: aws-lc-sys@0.44.0:       |          ^~~~~~~~~~~
warning: aws-lc-sys@0.44.0: 1 error generated.
warning: aws-lc-sys@0.44.0: ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-gcd.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-gcd.c"cargo:rerun-if-env-changed=VCINSTALLDIR
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-health.c:22:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-health.h:23:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
warning: aws-lc-sys@0.44.0: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
warning: aws-lc-sys@0.44.0:    49 | #include <windows.h>
warning: aws-lc-sys@0.44.0:       |          ^~~~~~~~~~~
warning: aws-lc-sys@0.44.0: 1 error generated.
warning: aws-lc-sys@0.44.0: ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-health.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-health.c"cargo:rerun-if-env-changed=VCINSTALLDIR
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-noise.c:21:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-noise.h:23:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
warning: aws-lc-sys@0.44.0: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
warning: aws-lc-sys@0.44.0:    49 | #include <windows.h>
warning: aws-lc-sys@0.44.0:       |          ^~~~~~~~~~~
warning: aws-lc-sys@0.44.0: 1 error generated.
warning: aws-lc-sys@0.44.0: ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-noise.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-noise.c"cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-sha3.c:21:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-sha3.h:23:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
warning: aws-lc-sys@0.44.0: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
warning: aws-lc-sys@0.44.0:    49 | #include <windows.h>
warning: aws-lc-sys@0.44.0:       |          ^~~~~~~~~~~
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-timer.c:22:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-timer.h:23:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
warning: aws-lc-sys@0.44.0: In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
warning: aws-lc-sys@0.44.0: /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
warning: aws-lc-sys@0.44.0:    49 | #include <windows.h>
warning: aws-lc-sys@0.44.0:       |          ^~~~~~~~~~~
warning: aws-lc-sys@0.44.0: 1 error generated.
warning: aws-lc-sys@0.44.0: ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-sha3.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-sha3.c"cargo:warning=1 error generated.
warning: aws-lc-sys@0.44.0: ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-timer.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-timer.c"
error: failed to run custom build command for `aws-lc-sys v0.44.0`
note: To improve backtraces for build dependencies, set the CARGO_PROFILE_DEV_BUILD_OVERRIDE_DEBUG=true environment variable to enable debug information generation.

Caused by:
  process didn't exit successfully: `/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/debug/build/aws-lc-sys-6ad9a570e9d6c662/build-script-main` (exit status: 1)
  --- stdout
  cargo:rerun-if-env-changed=AWS_LC_SYS_NO_PREFIX_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_NO_PREFIX
  cargo:rerun-if-env-changed=AWS_LC_SYS_PREGENERATING_BINDINGS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_PREGENERATING_BINDINGS
  cargo:rerun-if-env-changed=AWS_LC_SYS_EXTERNAL_BINDGEN_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_EXTERNAL_BINDGEN
  cargo:rerun-if-env-changed=AWS_LC_SYS_NO_ASM_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_NO_ASM
  cargo:rerun-if-env-changed=AWS_LC_SYS_PREBUILT_NASM_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_PREBUILT_NASM
  cargo:rerun-if-env-changed=AWS_LC_SYS_C_STD_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_C_STD
  cargo:rerun-if-env-changed=AWS_LC_SYS_CMAKE_BUILDER_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_CMAKE_BUILDER
  cargo:rerun-if-env-changed=AWS_LC_SYS_NO_PREGENERATED_SRC_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_NO_PREGENERATED_SRC
  cargo:rerun-if-env-changed=AWS_LC_SYS_SMALL_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_SMALL
  cargo:rerun-if-env-changed=AWS_LC_SYS_EFFECTIVE_TARGET_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_EFFECTIVE_TARGET
  cargo:rerun-if-env-changed=AWS_LC_SYS_NO_JITTER_ENTROPY_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_NO_JITTER_ENTROPY
  cargo:rerun-if-env-changed=AWS_LC_SYS_NO_U1_BINDINGS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_NO_U1_BINDINGS
  cargo:rerun-if-env-changed=AWS_LC_SYS_INCLUDES_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_INCLUDES
  cargo:rerun-if-env-changed=AWS_LC_SYS_SANITIZER_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_SANITIZER
  cargo:rerun-if-env-changed=AWS_LC_SYS_LINK_WHOLE_ARCHIVE_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_LINK_WHOLE_ARCHIVE
  cargo:rerun-if-env-changed=AWS_LC_SYS_STATIC_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_STATIC
  cargo:rerun-if-env-changed=AWS_LC_SYS_SYSTEM_DIR_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_SYSTEM_DIR
  cargo:rerun-if-env-changed=AWS_LC_SYS_USE_SYSTEM_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_USE_SYSTEM
  cargo:rerun-if-env-changed=AWS_LC_SYS_SYSTEM_BINDINGS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_SYSTEM_BINDINGS
  cargo:rerun-if-env-changed=AWS_LC_SYS_SYSTEM_SKIP_VERSION_CHECK_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_SYSTEM_SKIP_VERSION_CHECK
  cargo:rerun-if-env-changed=CARGO_ENCODED_RUSTFLAGS
  cargo:warning=Environment Variable found 'CARGO_ENCODED_RUSTFLAGS': ''
  cargo:warning=Emitting configuration: cargo:rustc-cfg=universal
  cargo:rustc-cfg=universal
  cargo:rerun-if-env-changed=OPENSSL_DIR_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=OPENSSL_DIR
  cargo:rerun-if-env-changed=OPENSSL_INCLUDE_DIR_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=OPENSSL_INCLUDE_DIR
  cargo:rerun-if-env-changed=OPENSSL_LIB_DIR_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=OPENSSL_LIB_DIR
  cargo:rerun-if-env-changed=OPENSSL_NO_PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG_ALLOW_CROSS
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS
  cargo:rerun-if-env-changed=PKG_CONFIG_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG_SYSROOT_DIR
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR
  cargo:rerun-if-env-changed=AWS_LC_NO_PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG_ALLOW_CROSS
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS
  cargo:rerun-if-env-changed=PKG_CONFIG_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG_SYSROOT_DIR
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR
  cargo:rerun-if-env-changed=LIBCRYPTO_NO_PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG_ALLOW_CROSS
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS
  cargo:rerun-if-env-changed=PKG_CONFIG_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG_SYSROOT_DIR
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR
  cargo:rerun-if-env-changed=LIBCRYPTO_AWSLC_NO_PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG_ALLOW_CROSS
  cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS
  cargo:rerun-if-env-changed=PKG_CONFIG_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64-pc-windows-msvc
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_PKG_CONFIG_SYSROOT_DIR
  cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR
  cargo:rerun-if-env-changed=CARGO_FEATURE_SSL
  cargo:warning=Building with: CC
  cargo:warning=Symbol Prefix: Some("aws_lc_0_44_0")
  cargo:rerun-if-env-changed=CARGO_FEATURE_SSL
  cargo:warning=Target platform: 'x86_64-pc-windows-msvc'
  cargo:root=/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out
  cargo:rerun-if-env-changed=AWS_LC_SYS_TARGET_CFLAGS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_TARGET_CFLAGS
  cargo:rerun-if-env-changed=TARGET_CFLAGS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  cargo:rerun-if-env-changed=AWS_LC_SYS_CFLAGS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_CFLAGS
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=CFLAGS
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CC_ENABLE_DEBUG_OUTPUT
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CC_ENABLE_DEBUG_OUTPUT
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CC_ENABLE_DEBUG_OUTPUT
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_FORCE_DISABLE
  CC_FORCE_DISABLE = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:warning=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/tests/compiler_features_tests/c11.c:7:10: fatal error: 'stdlib.h' file not found
  cargo:warning=    7 | #include <stdlib.h>
  cargo:warning=      |          ^~~~~~~~~~
  cargo:warning=1 error generated.
  cargo:warning=Compilation of 'c11.c' failed - Err(Error { kind: ToolExecError, message: "command did not execute successfully (status code exit status: 1): LC_ALL=\"C\" \"cc\" \"-O0\" \"-ffunction-sections\" \"-fdata-sections\" \"-m64\" \"--target=x86_64-pc-windows-msvc\" \"-Werror\" \"-Wall\" \"-Wextra\" \"-Wno-unused-parameter\" \"-o\" \"/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/out-c11/7dfda64fdf5a526c-c11.o\" \"-c\" \"/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/tests/compiler_features_tests/c11.c\"" }).
  cargo:rerun-if-env-changed=AWS_LC_SYS_TARGET_CC_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_TARGET_CC
  cargo:rerun-if-env-changed=TARGET_CC_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_CC
  cargo:rerun-if-env-changed=AWS_LC_SYS_CC_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_CC
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=CC
  cargo:rerun-if-env-changed=AWS_LC_SYS_TARGET_CXX_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_TARGET_CXX
  cargo:rerun-if-env-changed=TARGET_CXX_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=TARGET_CXX
  cargo:rerun-if-env-changed=AWS_LC_SYS_CXX_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=AWS_LC_SYS_CXX
  cargo:rerun-if-env-changed=CXX_x86_64_pc_windows_msvc
  cargo:rerun-if-env-changed=CXX
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CC_ENABLE_DEBUG_OUTPUT
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_FORCE_DISABLE
  CC_FORCE_DISABLE = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:warning=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/tests/compiler_features_tests/stdalign_check.c:10:10: fatal error: 'stdlib.h' file not found
  cargo:warning=   10 | #include <stdlib.h>
  cargo:warning=      |          ^~~~~~~~~~
  cargo:warning=1 error generated.
  cargo:warning=Compilation of 'stdalign_check.c' failed - Err(Error { kind: ToolExecError, message: "command did not execute successfully (status code exit status: 1): LC_ALL=\"C\" \"cc\" \"-O0\" \"-ffunction-sections\" \"-fdata-sections\" \"-m64\" \"--target=x86_64-pc-windows-msvc\" \"-Werror\" \"-Wall\" \"-Wextra\" \"-Wno-unused-parameter\" \"-o\" \"/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/out-stdalign_check/7dfda64fdf5a526c-stdalign_check.o\" \"-c\" \"/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/tests/compiler_features_tests/stdalign_check.c\"" }).
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CC_ENABLE_DEBUG_OUTPUT
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_FORCE_DISABLE
  CC_FORCE_DISABLE = None
  cargo:rerun-if-env-changed=CC_FORCE_DISABLE
  CC_FORCE_DISABLE = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-base.c:33:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-gcd.h:23:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
  cargo:warning=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
  cargo:warning=   49 | #include <windows.h>
  cargo:warning=      |          ^~~~~~~~~~~
  cargo:warning=1 error generated.
  exit status: 1
  cargo:warning=ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-base.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-base.c"cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-gcd.c:22:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-gcd.h:23:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
  cargo:warning=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
  cargo:warning=   49 | #include <windows.h>
  cargo:warning=      |          ^~~~~~~~~~~
  cargo:warning=1 error generated.

  exit status: 1
  cargo:warning=ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-gcd.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-gcd.c"cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-health.c:22:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-health.h:23:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
  cargo:warning=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
  cargo:warning=   49 | #include <windows.h>
  cargo:warning=      |          ^~~~~~~~~~~
  cargo:warning=1 error generated.

  exit status: 1
  cargo:warning=ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-health.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-health.c"cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=VCINSTALLDIR
  VCINSTALLDIR = None
  cargo:rerun-if-env-changed=CC_x86_64-pc-windows-msvc
  CC_x86_64-pc-windows-msvc = None
  cargo:rerun-if-env-changed=CC_x86_64_pc_windows_msvc
  CC_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=TARGET_CC
  TARGET_CC = None
  cargo:rerun-if-env-changed=CC
  CC = None
  cargo:rerun-if-env-changed=CROSS_COMPILE
  CROSS_COMPILE = None
  cargo:rerun-if-env-changed=CRATE_CC_NO_DEFAULTS
  CRATE_CC_NO_DEFAULTS = None
  cargo:rerun-if-env-changed=CFLAGS
  CFLAGS = None
  cargo:rerun-if-env-changed=TARGET_CFLAGS
  TARGET_CFLAGS = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64_pc_windows_msvc
  CFLAGS_x86_64_pc_windows_msvc = None
  cargo:rerun-if-env-changed=CFLAGS_x86_64-pc-windows-msvc
  CFLAGS_x86_64-pc-windows-msvc = None
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-noise.c:21:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-noise.h:23:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
  cargo:warning=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
  cargo:warning=   49 | #include <windows.h>
  cargo:warning=      |          ^~~~~~~~~~~
  cargo:warning=1 error generated.

  exit status: 1
  cargo:warning=ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-noise.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-noise.c"cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-sha3.c:21:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-sha3.h:23:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
  cargo:warning=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
  cargo:warning=   49 | #include <windows.h>
  cargo:warning=      |          ^~~~~~~~~~~
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-timer.c:22:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-timer.h:23:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-internal.h:45:
  cargo:warning=In file included from /Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/jitterentropy.h:166:
  cargo:warning=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/arch/jitterentropy-base-windows.h:49:10: fatal error: 'windows.h' file not found
  cargo:warning=   49 | #include <windows.h>
  cargo:warning=      |          ^~~~~~~~~~~
  cargo:warning=1 error generated.

  exit status: 1
  cargo:warning=ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-sha3.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-sha3.c"cargo:warning=1 error generated.

  exit status: 1
  cargo:warning=ToolExecError: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-timer.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-timer.c"

  --- stderr


  error occurred in cc-rs: command did not execute successfully (status code exit status: 1): LC_ALL="C" "cc" "-O0" "-ffunction-sections" "-fdata-sections" "-m64" "--target=x86_64-pc-windows-msvc" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/s2n-bignum/s2n-bignum-imported/include" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library" "-I" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src" "-Wall" "-Wextra" "-fwrapv" "--param" "ssp-buffer-size=4" "-fvisibility=hidden" "-Wcast-align" "-Wmissing-field-initializers" "-Wshadow" "-Wswitch-enum" "-Wextra" "-Wall" "-pedantic" "-O0" "-fwrapv" "-Wconversion" "-ffile-prefix-map=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0=" "-fsanitize-undefined-strip-path-components=-1" "--include=/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/generated-include/openssl/boringssl_prefix_symbols.h" "-DBORINGSSL_IMPLEMENTATION=1" "-DBORINGSSL_PREFIX=aws_lc_0_44_0" "-DWIN32_LEAN_AND_MEAN=" "-DNOMINMAX=" "-D_CRT_SECURE_NO_WARNINGS=1" "-DAWSLC=1" "-o" "/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve-worktrees/upgrade-m136/target/x86_64-pc-windows-msvc/debug/build/aws-lc-sys-3e667223b1e35892/out/2d40dbbd793ef942-jitterentropy-timer.o" "-c" "/Users/yexrob/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/aws-lc-sys-0.44.0/aws-lc/third_party/jitterentropy/jitterentropy-library/src/jitterentropy-timer.c"


mbx[cache]: 155 hits, 4 misses, 1 bypassed; 0 B downloaded, 0 B uploaded, 5.2 MiB stored locally
mbx[savings]: 253681 cache hits and counting. rustc thinks it has been busy.
```

</details>

<details>
<summary>tui-smoke — exit 0</summary>

```text
$ bash /tmp/m136-tui-smoke.sh
  a reply reaches the transcript
  esc interrupts a turn that is still waiting
  esc interrupts a turn whose request has not answered yet
  a page up releases the tail, and the foot takes it back
  the wheel scrolls the transcript
  ctrl+f searches the transcript and esc gives the status line back
  a focused block opens whole in the pager and gives the frame back
  an @ offers the paths under the session and enter takes one
  the help sheet opens on ? and closes on esc
  ctrl+g opens the one list of sessions and esc closes it
  a permission dialog answered y runs the tool
  a notice holds the status line and then leaves it
  a question on a window nobody watches notifies and nothing else
  a picture is named where the terminal does not draw pictures
  BINGO_GRAPHICS=off asks nothing and still names the picture
  BINGO_ASCII=1 and NO_COLOR leave a terminal nothing it cannot draw
  a live signal moves in the rail and leaves nothing behind
  a button on a pinned board fires its command and the table changes
tui-smoke ok
```

</details>

<details>
<summary>typescript — exit 0</summary>

```text
$ npm test --prefix sdk/typescript

> test
> node --test src/*.test.ts

✔ matchers are partial, nested, scalar, regex or alternatives (0.606958ms)
✔ patterns do not depend on the current event catalogue (0.099708ms)
✔ multiple hooks wrap once, preserving after order (0.91825ms)
✔ short circuit answers without next and cancellation rejects next (2.628708ms)
✔ an omitted matcher accepts null and scalar event inputs (0.343042ms)
✔ cancel before next rejects before and clears the chain (0.226ms)
✔ cancel while the after hook is waiting rejects after (0.187333ms)
✔ RPC pairs ids, rejects unknown requests and closes all pending calls (0.751209ms)
✔ handshake advertises matching points and intersects host capabilities (0.704125ms)
✔ protocol one is refused before register runs (0.165292ms)
✔ catalogue subscriptions wrap a service event added after initialization (0.520958ms)
ℹ tests 11
ℹ suites 0
ℹ pass 11
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 80.684459
```

</details>

<details>
<summary>discipline-record — exit 0</summary>

```text
$ scripts/check_discipline.sh
dependency direction ok
warn crates/bingo-plugin-rpc/examples/stub_plugin/main.rs: 709 non-test lines (>700)
warn crates/bingo-plugin-rpc/src/wire.rs: 860 non-test lines (>700)
warn crates/bingo-channel-feishu/src/feishu/tests.rs: 894 non-test lines (>700)
warn crates/bingo-surface-tui/src/ui.rs: 731 non-test lines (>700)
warn crates/bingo-surface-tui/src/transcript.rs: 949 non-test lines (>700)
warn crates/bingo-surface-tui/src/test_support.rs: 957 non-test lines (>700)
warn crates/bingo-surface-tui/src/run.rs: 982 non-test lines (>700)
warn crates/bingo-surface-tui/src/screens.rs: 1000 non-test lines (>700)
warn crates/bingo-surface-tui/src/input.rs: 898 non-test lines (>700)
warn crates/bingo-surface-tui/src/view.rs: 804 non-test lines (>700)
warn crates/bingo-surface-tui/src/form.rs: 846 non-test lines (>700)
warn crates/bingo-surface-tui/src/motion.rs: 941 non-test lines (>700)
warn crates/bingo-mcp/tests/server.rs: 771 non-test lines (>700)
warn crates/bingo-provider-fake/src/lib.rs: 703 non-test lines (>700)
warn crates/bingo-agents/src/tests.rs: 758 non-test lines (>700)
warn crates/bingo-channels/src/host/tests.rs: 947 non-test lines (>700)
warn crates/bingo-channels/src/deliver/tests.rs: 798 non-test lines (>700)
warn crates/bingo-channels/src/runner.rs: 707 non-test lines (>700)
warn crates/bingo-core/src/session.rs: 934 non-test lines (>700)
warn crates/bingo-core/src/host.rs: 995 non-test lines (>700)
warn crates/bingo-core/src/host/tests/resume.rs: 996 non-test lines (>700)
warn crates/bingo-core/src/host/tests.rs: 736 non-test lines (>700)
warn crates/bingo-core/src/turn.rs: 983 non-test lines (>700)
warn crates/bingo-core/src/test_support.rs: 740 non-test lines (>700)
warn crates/bingo-core/src/turn/tests.rs: 984 non-test lines (>700)
warn crates/bingo-core/src/session/tests.rs: 704 non-test lines (>700)
warn crates/bingo-surface-rpc/tests/wire/bounded.rs: 902 non-test lines (>700)
warn crates/bingo-surface-rpc/tests/wire/main.rs: 811 non-test lines (>700)
warn crates/bingo-surface-rpc/src/server.rs: 738 non-test lines (>700)
warn crates/bingo/tests/pty/main.rs: 965 non-test lines (>700)
warn crates/bingo/tests/cli/agents.rs: 762 non-test lines (>700)
warn crates/bingo/tests/cli/rooms.rs: 978 non-test lines (>700)
warn crates/bingo/tests/cli/gateway.rs: 744 non-test lines (>700)
warn crates/bingo/tests/cli/main.rs: 831 non-test lines (>700)
warn crates/bingo/tests/cli/acp.rs: 818 non-test lines (>700)
warn crates/bingo/tests/channels.rs: 742 non-test lines (>700)
warn crates/bingo/tests/rpc.rs: 991 non-test lines (>700)
warn crates/bingo/src/main.rs: 963 non-test lines (>700)
warn crates/bingo-provider-acp/src/session.rs: 755 non-test lines (>700)
kernel names no tool
shared service declarations ok
TUI names no plugin or tool
cohesion ok
warn crates/bingo-context/src/compact.rs:24 fn compact is 61 lines (>60)
warn crates/bingo-core/src/chain/mod.rs:165 fn invoke is 75 lines (>60)
warn crates/bingo-core/src/host/registry.rs:142 fn add is 76 lines (>60)
warn crates/bingo-core/src/session.rs:137 fn dispatch is 98 lines (>60)
warn crates/bingo-core/src/turn/stream.rs:25 fn stream is 76 lines (>60)
warn crates/bingo-core/src/turn.rs:164 fn run_turn is 63 lines (>60)
warn crates/bingo-sdk/src/testing/calls.rs:85 fn fmt is 109 lines (>60)
warn crates/bingo-surface-print/src/render.rs:161 fn text is 61 lines (>60)
warn crates/bingo-surface-tui/src/test_lanes.rs:227 fn task_payload is 74 lines (>60)
warn docs/plans/M136-the-event-catalogue.md: 11339 lines (>150)
warn docs/plans/M26-the-bridge-learns-context-and-compaction.md: 152 lines (>150)
warn docs/plans/M27-the-bridge-learns-providers.md: 201 lines (>150)
warn docs/plans/M28-wire-services.md: 225 lines (>150)
warn docs/plans/M29-hooks-cross-the-bridge.md: 230 lines (>150)
warn docs/plans/M30-the-allowance.md: 230 lines (>150)
warn docs/plans/M31-stored-children-in-sight.md: 192 lines (>150)
warn docs/plans/M32-the-picker-says-what-a-session-is.md: 203 lines (>150)
warn docs/plans/M34-the-surface-answers-the-person.md: 179 lines (>150)
warn docs/plans/M35-the-endpoint-says-what-it-serves.md: 185 lines (>150)
warn docs/plans/M36-the-roster-stands-up.md: 168 lines (>150)
warn docs/plans/M37-the-room-is-read.md: 171 lines (>150)
warn docs/plans/M38-an-agent-answers-as-a-model.md: 227 lines (>150)
warn docs/plans/M39-shared-tools-cross-as-mcp.md: 255 lines (>150)
warn docs/plans/M43-a-question-reaches-the-person.md: 169 lines (>150)
warn docs/plans/M45-a-picture-beside-the-words.md: 232 lines (>150)
warn docs/plans/M46-the-picture-is-seen.md: 280 lines (>150)
warn docs/plans/M47-the-picture-from-anywhere.md: 235 lines (>150)
warn docs/plans/M48-the-draft-shows-its-pictures.md: 325 lines (>150)
warn docs/plans/M49-the-picture-through-tmux.md: 288 lines (>150)
warn docs/plans/M50-the-model-knows-the-picture-is-seen.md: 230 lines (>150)
warn docs/plans/M51-the-picture-in-the-words.md: 315 lines (>150)
warn docs/plans/M52-the-beat-on-the-persons-side.md: 363 lines (>150)
warn docs/plans/M53-the-questions-asked-together.md: 300 lines (>150)
warn docs/plans/M54-the-page-that-answers-back.md: 219 lines (>150)
warn docs/plans/M55-the-list-that-is-searched.md: 250 lines (>150)
warn docs/plans/M56-the-picture-where-the-eye-is.md: 225 lines (>150)
warn docs/plans/M57-the-form-card-compared.md: 275 lines (>150)
warn docs/plans/M58-the-list-reads-the-line.md: 279 lines (>150)
warn docs/plans/M59-the-form-holds-still.md: 240 lines (>150)
warn docs/plans/M60-the-late-answer.md: 326 lines (>150)
warn docs/plans/M61-the-picture-that-was-already-here.md: 331 lines (>150)
warn docs/plans/M62-the-band-above-the-words.md: 211 lines (>150)
warn docs/plans/M63-the-binary-that-replaces-itself.md: 196 lines (>150)
warn docs/plans/M64-one-fact-one-file.md: 154 lines (>150)
warn docs/plans/M65-the-bang-line.md: 191 lines (>150)
warn docs/plans/M67-the-turn-that-can-be-undone.md: 164 lines (>150)
warn docs/plans/M68-the-line-that-waits.md: 174 lines (>150)
warn docs/plans/M69-the-opening-shot.md: 232 lines (>150)
warn docs/plans/M70-the-box-that-opens.md: 277 lines (>150)
warn docs/plans/M71-the-look-that-follows.md: 232 lines (>150)
warn docs/plans/M72-the-line-that-draws-itself.md: 288 lines (>150)
warn docs/plans/M73-the-ink-the-terminal-owns.md: 203 lines (>150)
warn docs/plans/M84-worked-for-recap-and-memory-rows.md: 191 lines (>150)
warn docs/plans/M85-mcp-servers-that-need-signing-in.md: 304 lines (>150)
warn docs/plans/M86-the-chat-carries-more-than-words.md: 158 lines (>150)
warn docs/plans/M91-the-context-an-agent-holds.md: 157 lines (>150)
warn docs/plans/M97-a-plugin-is-switched.md: 220 lines (>150)
warn docs/plans/M98-the-settings-file-keeps-its-comments.md: 199 lines (>150)
warn docs/plans/M99-the-agent-has-a-view-of-its-own.md: 158 lines (>150)
warn crates/bingo-provider-acp/tests/harness/prompt_host.rs:19: use bingo_sdk::testing::TestHost or explain the retained host
test host and wall-clock advisory checks complete
catalogue entry declarations ok
discipline ok
```

</details>
