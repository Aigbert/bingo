# M134 — An agent is not a model
## Goal
ADR-0076, audit §6: ACP implements Agent, journals external ToolCall items, judges permissions
through tool.check, sends newest input without history, and removes holds_context. Historical journals still draw tool rows.
## Bricks, in build order
1. Contracts first: Agent/AgentTurn/AgentStream/AgentEvent/Judge, ExternalCall/CallKind, Contribution::Agent; red golden, core, legacy-plugin and historical-store fixtures.
2. Judge: fail-closed traits; narrowest ACP option; tool.check links/hooks/policy/person, raced with cancel.
3. Agent driver: one run; newest input and resolved tools; shared Said accumulator;
   external Call/Update/Done become ToolCall; tool.call links tighten, AfterTool can end the turn.
4. Composition: Provider or Agent per id; resolve/catalogue both; agents refuse model side calls.
5. ACP: agent/permission replace provider/question; retain pictures, restore ladder, knobs,
   readings/compactions and cancellation; unchanged bridge; remove private window cache.
6. Remove capability/literals; generate both plugin schemas; ignore legacy input with one named Info for true.
7. Version-3 migration upgrades 979a6d27-format metadata; invalidate version-2 checkpoints.
8. Delete ACP-only TUI parser/screens; ordinary tool titles; checkpoint every reported write location.
## Files
- `crates/bingo-sdk/src/{agent.rs (new),plugin.rs,model.rs,lib.rs,store/checkpoint.rs}`
- `crates/bingo-core/src/{judge.rs (new),turn/agent.rs (new),turn/endpoint.rs (new),turn/host.rs (new),turn.rs,turn/stream.rs,session.rs,session/{complete,invoke}.rs,spec.rs,host.rs,commands/model.rs,models/{resolve,learned}.rs,test_support.rs}`
- `crates/bingo-store-jsonl/src/journal/{migrate.rs,versions.rs}` and
  journal.rs, fixtures and checkpoint tests (journal/checkpoint versions 2 → 3)
- `crates/bingo-provider-acp/src/{lib.rs,provider.rs→agent.rs,events.rs,events_tests.rs,question.rs→permission.rs,render.rs,fixtures.rs,guide.md,windows.rs (deleted),transcript.rs}`
- `crates/bingo-core/src/{accumulator.rs,chain/{tool,compact}.rs,turn/config.rs,turn/late.rs,host/{registry,choose,catalog,refresh}.rs}`
- `crates/bingo-provider-acp/src/{inbox.rs,refusal.rs,session.rs}` and `tests/{pictures.rs,fake_agent.rs,deadlines.rs,harness/mod.rs}`
- `crates/bingo-checkpoints/src/hook.rs`
- `crates/bingo-surface-tui/src/{acp.rs (deleted),screens/acp.rs (deleted),transcript.rs,fold.rs,thoughts.rs,pager.rs,screens.rs,lib.rs,test_support.rs,test_support/external.rs (new)}`
- `crates/bingo-surface-print/src/render.rs` (an external call prints as a tool line)
- `holds_context` literals in `bingo-{context,experience,persona,plugin-rpc,rooms,skills,tasks}`; mechanical constructor fixtures per protocol step 6
- `crates/bingo-plugin-rpc/src/{wire.rs,schema.rs,bridge.rs,legacy.rs (new),lib.rs}, legacy fixtures` and generated `schema/plugin.json` and generator-produced `schema/plugin-2.json`
- `crates/bingo/tests/{acp_asked.rs,cli/acp.rs,cli/acp/{asking,bridge,life}.rs}`
- Companion contract/fixture tests; `docs/adr/{0009,0035,0039,0055,0075,0076}-*.md` (dated notes)
## Exit criteria
- [x] SDK golden round-trip, Judge kind table and ACP narrowest-option tests.
- [x] Agent external ToolCall, AfterTool observer, interruption while Judge waits (M116).
- [x] Project deny returns reject_once; images reach the adapter; bridge offers exactly resolved tools.
- [x] Asked agent Edit is restored by the rewind plan.
- [x] `rg holds_context crates` has no matches; no ACP metadata parser remains in the TUI.
- [x] 979a6d27-format fixture upgrades and renders via TUI TestBackend and print.
- [x] ACP permission interactions, mid-answer failure/new prompt, isolated token tests and tmux smoke.
- [x] Sequential local gates, with the delegate's sole baseline discipline exception below.
- [ ] Four MSVC checks deferred to Windows CI solely for missing C headers (protocol step 7).
## Non-goals
Non-ACP agents; ACP modes/plans/slash commands; fs/* or terminal/*; unasked edit rewind;
changes to other HostApi::ask clients; ACP crate splitting beyond planned files; TUI layout.
## Risks and decisions (delegate, 2026-10-13)
- Permission grammar accepts `acp/claude:execute(...)`. Session approval still selects allow_once;
  bingo retains scope. Historical metadata lacking an id remains a thought.
- Protocol 2 is unreleased (v0.6.6 ships 1): freeze starts at first release carrying 2.
  Remove holds_context from both sdk types and generator-produced plugin schemas (ADR-0075/0076).
  Unknown legacy fields are ignored; legacy true emits one plugin-naming Info; absent/true fixtures pin both.
- AgentTurn.tools is Vec<ToolSpec>, exactly ToolSet::gather's offer, passed to the unchanged bridge;
  no host reader or second selection (ADR-0036 §1, ADR-0076 §1).
- Open retry belongs to models. Mid-answer agent death fails with a reason, retains speech/work,
  emits no TurnRetrying, buries with one ACP_RESPAWN, and the next prompt gets a new child.
  Silent death may still retry inside the plugin; that existing test is unchanged (ADR-0009/0076).
- Leave sdk event.rs unchanged: baseline 1004 lines, fixed on dev by 98d7355d.
  Split M134's host contract and external fixture by responsibility; that baseline is the only allowed failure.
## Re-verified (2026-10-13, e9e45bb9)
Clean branch m134-an-agent-is-not-a-model; M108–M123, M127–M132 landed.
M131 uses chain/tool.rs tool.check/tool.call; M112 supplies Accumulator/Transcript;
M116 session/running.rs; M111 sdk test hosts; M115 store record picture hydration.
Re-verification committed before code (563e9526). Mechanical constructor fixtures follow protocol step 6.
Actual TUI changes: every TUI path in Files, plus four deleted screens__acp_* snapshots.
No layout or M133-owned code changes. TurnHost moves verbatim to turn/host.rs to ease the dev merge;
no rebase/merge and no streaming/Door changes.
## Verified (2026-10-13)
Contracts committed red before implementation; trimmed output from their commands:
```text
cargo test -p bingo-sdk agent::
error[E0599]: no associated function or constant named `from_input` found for struct `agent::ExternalCall` in the current scope
cargo test -p bingo-core agent_turn::
error[E0433]: cannot find `Endpoint` in `super`
cargo test -p bingo-plugin-rpc legacy::
error[E0425]: cannot find function `notice` in this scope
cargo test -p bingo-store-jsonl historical_agent_calls_upgrade_to_external_tool_items
assertion failed: item.external()
cargo test -p bingo --test cli a_child_that_dies_mid_stream_fails_the_turn_it_was_answering
assertion failed: matches!(first.event, Event::TurnCompleted
    { status: bingo_sdk::TurnStatus::Failed { error }, .. } if
    error.message.contains("agent stopped mid-answer"))
```
Both schemas were generated, never hand-edited:
```text
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc the_committed_schema_is_this_document
test schema::tests::the_committed_schema_is_this_document ... ok
```
Sequential gates after the final module splits (trimmed pasted output):
```text
cargo fmt --all -- --check
fmt exit=0
cargo check --workspace --all-targets --locked
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 2.95s
cargo clippy --workspace --all-targets --locked -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.75s
cargo test --workspace --locked -- --skip a_token_nobody_minted_never_gets_a_conversation --skip a_second_concurrent_stream_on_one_token_is_refused --skip a_token_may_be_dialled_again_once_its_stream_has_closed
test exit=0
test agent::tests::external_call_round_trips_the_journal_fixture ... ok
test judge::tests::kind_traits_fail_closed ... ok
test turn::tests::agent_turn::external_calls_reach_before_and_after_hooks ... ok
test turn::tests::agent_turn::interrupt_drops_a_waiting_judge ... ok
test acp::asking::a_project_deny_rule_rejects_the_agents_execute ... ok
test the_bridge_offers_exactly_the_turns_resolved_tools ... ok
test hook::tests::an_asked_agent_edit_is_restored_by_the_rewind_plan ... ok
test render::tests::an_upgraded_historical_agent_call_prints_as_a_tool ... ok
test transcript::tests::an_upgraded_historical_agent_call_draws_a_tool_row ... ok
cargo test -p bingo-provider-acp --test bridge token --locked -- --test-threads=1
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.02s
scripts/check_discipline.sh
FAIL crates/bingo-sdk/src/event.rs: 1004 non-test lines (>1000)
discipline FAILED
scripts/budget.sh
dependencies (unique, normal): 353 (max  353)
warm cargo check -p bingo-core: 0s (max  20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
target/debug: 7 GB (soft max  5)
warn: target/debug exceeds the soft limit
test binaries: 122
budget ok
cargo deny check
advisories ok, bans ok, licenses ok, sources ok
```
Workspace total: 5,120 passed, four pre-existing ignored stress/measurement/storyboards;
all 16 PTY tests passed, no flake. acp_asked: two passed. Token tests ran alone, not silently skipped.
MSVC command for each touched crate: `cargo check -p <crate> --all-targets --locked --target x86_64-pc-windows-msvc`.
```text
bingo: exit=101
bingo-checkpoints: exit=0
bingo-context: exit=0
bingo-core: exit=0
bingo-experience: exit=0
bingo-persona: exit=0
bingo-plugin-rpc: exit=101
bingo-provider-acp: exit=0
bingo-rooms: exit=0
bingo-sdk: exit=0
bingo-skills: exit=0
bingo-store-jsonl: exit=0
bingo-surface-print: exit=101
bingo-surface-tui: exit=101
bingo-tasks: exit=0
```
The four exit=101 checks reached only missing Windows C headers; defer to Windows CI per step 7.
`bingo`: tree-sitter, tree-sitter-bash, aws-lc-sys; plugin-rpc/print/tui: aws-lc-sys.
Pasted causes: `fatal error: 'stdio.h' file not found`, `fatal error: 'stdlib.h' file not found`,
`fatal error: 'windows.h' file not found`. No Rust compile error. No toolchain installed.
`cargo build -p bingo --locked` passed; `scripts/tui-smoke.sh` used a PATH wrapper mapping
its socket to owned `m134-6f0b88939b74449dbb7abe57f00f8752`, then cleaned only that server:
```text
tui-smoke ok
```
ADR-0076 accepted. No push, merge or PR. Only the documented MSVC criteria remain unticked.
