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
- Plugin RPC: `intercept`, `wire`, `bridge`, `host`, `hook`, `service`, `schema`, `events_ts`, `lib`; stub plugin fixtures.
- Tests: SDK intercept fixtures; core chain/host/session fixtures and new catalogue tests; RPC hook/reload fixtures; `crates/bingo/tests/catalogue.rs`.
- Generated: `schema/{events,plugin,plugin-2}.json`, `sdk/typescript/src/{events,wire}.ts`.
- TypeScript: `on`, `on.test`, `plugin`, `rpc`, `rpc.test`, `index`; shared `test/cases.json`; `scripts/check_discipline.sh`; named ADR notes.

## Exit criteria

- [ ] SDK catalogue tests pass; deleting an events.json entry fails (shown once); final guard rerun pending at stop.
- [x] Composed binary catalogue passes; reserved `tool_use_id` fixture fails (shown once).
- [x] Rewrite descriptions/received text; skip pins/tighten answers once; compose completion text beneath the answer.
- [x] Observe start, attach, turn.start, measure, detach, end in actual lifecycle order.
- [ ] Real MCP wire face raises `mcp.servers`; duplicate ownership names both owners (MCP passed; SDK collision rerun pending).
- [x] Generated TypeScript and stray Entry drift fail once; shared matcher/SDK tests pass.
- [ ] Existing tests and all gates pass; MSVC checks cover touched crates, with step-7 C-header exceptions recorded.

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
