# M134 — An agent is not a model
## Goal
ADR-0076: an ACP agent becomes an `Agent`, not a `Provider`. Its own tool
calls are journaled as external tool calls every surface draws without a
re-parse; a permission it asks for runs the `tool.check` chain (links,
hooks, checkpoints, policy, person); the agent turn sends the newest input
and no history; `holds_context` is deleted. An old journal draws the same.
Audit §6.
## Bricks, in build order
1. **Contract and failing tests.** `bingo-sdk/src/agent.rs`: `Agent`,
   `AgentTurn`, `AgentStream`, `AgentEvent`, `ExternalCall`, `CallKind`,
   `Judge`, `Verdict`; `Contribution::Agent`. `ExternalCall::{to_input,
   from_input}` round-trips with a golden fixture (the journal's spelling
   is the contract, M113). Failing tests in core on a scripted agent: a
   `Call`/`Done` pair journals one external `ToolCall` named
   `fake:edit`; a `BeforeTool` hook sees a judged `Execute`; a deny rule
   for `fake:execute` answers the agent's reject option.
2. **Pure bricks.** `traits_of(CallKind) -> ToolTraits`, the fail-closed
   table of ADR-0076 §5 (`trusted` false everywhere, `Other` all false).
   `narrowest(options, Verdict) -> Option<OptionId>` over ACP's
   `allow_once`/`allow_always`/`reject_once`/`reject_always` (no
   allow option → reject; no reject option → the turn fails in words).
   `CallKind::writes()` for `Edit`/`Delete`/`Move`. Table tests.
3. **The judge.** `bingo-core/src/judge.rs`: builds the stand-in call
   (`<agent>:<kind>`, input from `to_input`, traits from the table) and
   runs the `tool.check` chain M131 built; maps `Gated` to `Verdict`.
   Raced inside the turn's supervised future (ADR-0068): esc drops a
   waiting permission and the agent is told reject on the way out.
4. **The agent turn.** `bingo-core/src/turn/agent.rs`: one `run` per
   turn. `Said` goes through the same stream fold the model turn uses
   (`turn/stream.rs`), `Context` and `Compacted` as ADR-0055 §2–§4 read
   them today; `Call`/`Update` start and update an external `ToolCall`
   item; `Done` passes `tool.call` as an observer (ADR-0076 §4) and ends
   the item. `run_turn` picks the driver by what the resolved id names.
5. **Composition.** `Registrar::agent`; a provider id registered as both
   is a `KernelError` at build; model resolution and the catalogue list an
   agent's `models()` as they list a provider's. `ModelRequest.session`
   stays for a stateful `Provider` (ADR-0035 §3 named OpenAI's).
6. **The ACP crate becomes an agent.** `provider.rs` → `agent.rs`;
   `events.rs` emits `AgentEvent::Call/Update/Done` and drops `Call::
   metadata` and the reasoning path; `question.rs` → `permission.rs`
   over `Judge` and `narrowest`; preserve M108's image blocks when
   the agent declared `promptCapabilities.image`, and its fallback note
   otherwise (`tests/pictures.rs` already covers both). `session/cancel` from the
   stream's drop. Fixtures re-recorded; the bridge (ADR-0036) untouched.
7. **Delete `holds_context`** from `EndpointCapabilities`,
   `ModelCapabilities` and `models/resolve.rs`, and every literal. Agent readings use the existing
   learned-window store; the ACP-only window cache is removed.
8. **Old journals.** The record's upgrade step (ADR-0065): a reasoning
   item whose metadata carries `acp.external` reads as the external
   `ToolCall` it held. A fixture journal in `979a6d27`'s recorded format draws its
   calls as tool rows in the TUI and in `--print`.
9. **Surfaces.** Delete `bingo-surface-tui/src/acp.rs` and its screens;
   the transcript draws an external call's `title` as the row heading via
   `ExternalCall::from_input`. Checkpoints' `written` reads `locations`
   when `kind.writes()`.
## Files
- `crates/bingo-sdk/src/{agent.rs (new),plugin.rs,model.rs,lib.rs,store/checkpoint.rs}`
- `crates/bingo-core/src/{judge.rs (new),turn/agent.rs (new),turn/endpoint.rs (new),turn.rs,turn/stream.rs,session.rs,session/{complete,invoke}.rs,spec.rs,host.rs,commands/model.rs,models/{resolve,learned}.rs,test_support.rs}`
- `crates/bingo-store-jsonl/src/journal/{migrate.rs,versions.rs}` and
  journal.rs, fixtures and checkpoint tests (journal/checkpoint versions 2 → 3)
- `crates/bingo-provider-acp/src/{lib.rs,provider.rs→agent.rs,events.rs,events_tests.rs,question.rs→permission.rs,render.rs,fixtures.rs,guide.md,windows.rs (deleted),transcript.rs}`
- `crates/bingo-core/src/{accumulator.rs,chain/tool.rs,turn/config.rs,turn/late.rs,host/{registry,choose,catalog,refresh}.rs}`
- `crates/bingo-provider-acp/src/{inbox.rs,refusal.rs,session.rs}` and `tests/{pictures.rs,fake_agent.rs,deadlines.rs,harness/mod.rs}`
- `crates/bingo-checkpoints/src/hook.rs`
- `crates/bingo-surface-tui/src/{acp.rs (deleted),screens/acp.rs (deleted),transcript.rs,fold.rs,thoughts.rs,pager.rs,screens.rs,lib.rs,test_support.rs}`
- `crates/bingo-surface-print/src/render.rs` (an external call prints as a tool line)
- `holds_context` literals in `bingo-{context,experience,persona,plugin-rpc,rooms,skills,tasks}`; mechanical constructor fixtures per protocol step 6
- `crates/bingo-plugin-rpc/src/{wire.rs,schema.rs,bridge.rs,legacy.rs (new),lib.rs}, legacy fixtures` and generated `schema/plugin.json` and generator-produced `schema/plugin-2.json`
- `crates/bingo/tests/{acp_asked.rs,cli/acp.rs,cli/acp/{asking,bridge}.rs}`
- `docs/adr/{0035,0039,0055}-*.md` (dated notes)
## Exit criteria
- [ ] `cargo test -p bingo-sdk agent::` (fixture round-trip) and
      `cargo test -p bingo-core judge::` (kind table); ACP tests pin `narrowest`.
- [ ] `cargo test -p bingo-core agent_turn::`: an external call journaled
      as `ToolCall`, seen by an `AfterTool` hook; esc while the judge
      waits ends `Interrupted` within the M116 bound.
- [ ] `cargo test -p bingo-provider-acp`: a scripted adapter's
      `request_permission` is denied by a project rule and answered with
      `reject_once`; a picture reaches the adapter as an image block.
- [ ] A checkpoints test: an agent's asked-for `Edit` is restored by
      `/rewind`.
- [ ] `rg holds_context crates` finds nothing; no ACP metadata parser remains in the TUI. The surface-name test may still name `"acp"`.
- [ ] The 979a6d27 fixture journal draws tool rows (TUI `TestBackend`, `--print`).
- [ ] `cargo test -p bingo --test acp_asked` passes; all gates; the ACP
      bridge token test run alone; `scripts/tui-smoke.sh`.
## Non-goals
- A non-ACP agent; ACP modes, plans, slash commands; `fs/*`, `terminal/*`.
- Checkpointing an agent's unasked edits; a new kernel word for them.
- Changing `HostApi::ask` or `InteractionKind::Question` for other askers.
- Splitting the ACP crate beyond the planned rename and files.

## Risks
- Rule grammar: whether `acp/claude:execute(…)` parses as a tool name in
  `bingo-permissions` (a `/` and a `:` inside the name). If not, the
  stand-in name changes in the plan, not ADR-0076 §5's mapping.
- `allow_always` never answers, even for "allow for the session": the
  agent will ask again next time, and the person's session-scoped answer
  answers it through the policy. A test pins that the person is not asked
  twice.
- The record upgrade needs the item's whole call in its metadata; items
  from before `Call::metadata` carried the id are left as thoughts.
- **Decision (person's delegate, 2026-10-13):** protocol 2 is unreleased
  (v0.6.6 carries protocol 1); its freeze starts at its first release.
  Remove the capability from both sdk types and regenerate both plugin
  schemas through the generator. This single pre-release removal is
  authorized by ADR-0076 §6; no compatibility field remains.
- Legacy unknown `holdsContext` input is ignored. A legacy `true` produces
  one Info notice at load naming the plugin and explaining that an endpoint
  holding its own conversation is an agent. Fixture tests pin absent/true.
- **Decision (person's delegate, 2026-10-13):** `AgentTurn` also carries
  `tools: Vec<ToolSpec>`, from `ToolSet::gather`'s resolved offer. ACP passes
  it to the unchanged bridge, with no host reader or second selection.
  A test pins that the bridge offers exactly that resolved set.

## Re-verified (2026-10-13, e9e45bb9)
- Clean branch `m134-an-agent-is-not-a-model`; M108–M123, M127–M132 landed.
  M131 uses `chain/tool.rs` (`tool.check`/`tool.call`), not the old names;
  the observer path still needs to restrict every link to tighten rights.
- M112: `accumulator.rs`/sdk `Transcript`; M116: `session/running.rs`.
  M111 supplies sdk test hosts; M115 hydrates live pictures in store `record.rs`.
- Parallel M124: intended TUI changes only delete the ACP parser/screens
  and remove their references in the TUI Files above; no layout changes.
  Actual TUI files changed: the nine TUI paths listed in Files, plus the four
  deleted `screens__acp_*` snapshots. No layout code or M133-owned code.

## STOP — mid-stream retry (2026-10-13)

Both prior Decisions were resolved. A third validation conflict remains:
`cargo test -p bingo --test acp_asked --test cli acp --locked` reports
`38 passed; 2 failed`. The metadata assertion in planned `cli/acp.rs` is
an expected M134 update. The other failure is outside Files:
`cli/acp/life.rs:152`, `a_child_that_dies_mid_stream_fails_the_turn_it_was_answering`:
```
left: Some(1)
right: Some(0)
stderr: [error] code=PROVIDER_UNAVAILABLE msg=transport: the connection closed
```
That test requires one visible kernel retry and success after partial speech.
M134's one `Agent::run` per turn no longer enters the provider retry loop.
Protocol step 6 requires a Decision before changing this behavioral assertion.
**Recommend A:** keep one run; fail and retain journaled work after a mid-stream
loss, authorize the `life.rs` assertion change and clarify ADR-0076 §1.
**B:** preserve automatic retries; decide their owner and revise the one-run
contract, including what happens when external calls have already acted.
Implementation is uncommitted; required gates are not yet run. Focused SDK/core,
ACP, migration, checkpoints and surface tests have passed during development;
the newest scope/context tests still need verification. No push, merge or PR.

Dev moved `TurnHost` to `turn/host.rs` after this branch; M134 leaves the trait unchanged.
