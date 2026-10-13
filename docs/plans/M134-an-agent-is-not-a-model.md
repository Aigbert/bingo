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
   `ModelCapabilities` and `models/resolve.rs`, and every literal.
8. **Old journals.** The record's upgrade step (ADR-0065): a reasoning
   item whose metadata carries `acp.external` reads as the external
   `ToolCall` it held. A fixture journal written at `979a6d27` draws its
   calls as tool rows in the TUI and in `--print`.
9. **Surfaces.** Delete `bingo-surface-tui/src/acp.rs` and its screens;
   the transcript draws an external call's `title` as the row heading via
   `ExternalCall::from_input`. Checkpoints' `written` reads `locations`
   when `kind.writes()`.

## Files

- `crates/bingo-sdk/src/{agent.rs (new),plugin.rs,model.rs,lib.rs}`
- `crates/bingo-core/src/{judge.rs (new),turn/agent.rs (new),turn.rs,turn/stream.rs,session.rs,plugins.rs,models/resolve.rs,context.rs,test_support.rs}`
- `crates/bingo-store-jsonl/src/journal/{migrate.rs,versions.rs}` and
  journal/record/checkpoint compatibility tests (M115 records are version 2)
- `crates/bingo-provider-acp/src/{lib.rs,provider.rs→agent.rs,events.rs,events_tests.rs,question.rs→permission.rs,render.rs,fixtures.rs,guide.md}`
- `crates/bingo-core/src/{accumulator.rs,chain/tool.rs,turn/config.rs,turn/late.rs,host/{registry,choose,catalog,refresh}.rs}`
- `crates/bingo-provider-acp/src/{inbox.rs,refusal.rs,session.rs}` and `tests/{pictures.rs,fake_agent.rs,harness/mod.rs}`
- `crates/bingo-checkpoints/src/hook.rs`
- `crates/bingo-surface-tui/src/{acp.rs (deleted),screens/acp.rs (deleted),transcript.rs,fold.rs,thoughts.rs,pager.rs,screens.rs,lib.rs,test_support.rs}`
- `crates/bingo-surface-print/src/render.rs` (an external call prints as a tool line)
- `holds_context` literals in `bingo-{context,experience,persona,plugin-rpc,rooms,skills,tasks}`; mechanical constructor fixtures per protocol step 6
- `crates/bingo-plugin-rpc/src/{wire.rs,schema.rs}` and generated `schema/plugin.json` (blocked below; frozen `schema/plugin-2.json` stays unchanged)
- `crates/bingo/tests/{acp_asked.rs,cli/acp.rs}`
- `docs/adr/{0035,0039,0055}-*.md` (dated notes)

## Exit criteria

- [ ] `cargo test -p bingo-sdk agent::` (fixture round-trip) and
      `cargo test -p bingo-core judge::` (kind table, `narrowest`).
- [ ] `cargo test -p bingo-core agent_turn::`: an external call journaled
      as `ToolCall`, seen by an `AfterTool` hook; esc while the judge
      waits ends `Interrupted` within the M116 bound.
- [ ] `cargo test -p bingo-provider-acp`: a scripted adapter's
      `request_permission` is denied by a project rule and answered with
      `reject_once`; a picture reaches the adapter as an image block.
- [ ] A checkpoints test: an agent's asked-for `Edit` is restored by
      `/rewind`.
- [ ] `rg holds_context crates` finds nothing (blocked below); no ACP metadata parser remains in the TUI. The surface-name test may still name `"acp"`.
- [ ] The 979a6d27 fixture journal draws tool rows (TUI `TestBackend`, `--print`).
- [ ] `cargo test -p bingo --test acp_asked` passes; all gates; the ACP
      bridge token test run alone; `scripts/tui-smoke.sh`.

## Non-goals

- A non-ACP agent; ACP modes, plans, slash commands; `fs/*`, `terminal/*`.
- Checkpointing an agent's unasked edits; a new kernel word for them.
- Changing `HostApi::ask` or `InteractionKind::Question` for other askers.
- Splitting the ACP crate's 8.7k lines further than the rename and the
  two files above.

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
- **STOP (2026-10-13, protocol step 6):** ADR-0076 §6 removes
  `holds_context` from both sdk capability types, but ADR-0075 §1 freezes
  their `holdsContext` fields in protocol 2. `wire.rs` embeds these sdk
  types directly; `schema.rs::protocol_two_keeps_every_frozen_shape`
  rejects either removal. Regeneration cannot reconcile the Decisions.
  Option A (recommended): explicitly allow deprecated protocol-2 wire
  compatibility fields while removing the kernel capability; specify the
  treatment of legacy `true` declarations before implementation.
  Option B: defer deletion and retain the deprecated capability until a
  separately authorized protocol 3 migration. Both require clarification
  of ADR-0076 §6 and the deletion criterion; neither is applied here.

## Re-verified (2026-10-13, e9e45bb9)

- Clean branch `m134-an-agent-is-not-a-model`; M108–M123, M127–M132 landed.
  M131 uses `chain/tool.rs` (`tool.check`/`tool.call`), not the old names;
  the observer path still needs to restrict every link to tighten rights.
- M112 folds through `accumulator.rs` and sdk `Transcript`; M116 owns the
  supervised turn in `session/running.rs`. M111 supplies sdk test hosts.
  M115 hydrates persisted pictures in store `record.rs`; the agent input
  can keep live `ContentPart` images without another picture representation.
- Parallel M124: intended TUI changes only delete the ACP parser/screens
  and remove their references in the TUI Files above; no layout changes.
  Actual TUI files changed: none. M133-owned code is unchanged.

## Verified (2026-10-13 — stopped before implementation)

Read-only Python reproduction of `schema.rs::preserves`; field removals in memory only:

```text
Current schema preserves protocol 2: True
Remove EndpointCapabilities.holdsContext -> preserves: False
Remove ModelCapabilities.holdsContext -> preserves: False
```

No schema was written. Gates/tests were not run: stopped before brick 1.
All criteria remain unticked; ADR-0076 is proposed. No TUI/live-service/PTY run.
