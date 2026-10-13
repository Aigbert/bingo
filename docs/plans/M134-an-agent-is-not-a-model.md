# M134 — An agent is not a model

## Goal

ADR-0076: an ACP agent becomes an `Agent`, not a `Provider`. Its own tool
calls are journaled as external tool calls every surface draws without a
re-parse; a permission it asks for runs the `tool.gate` chain (links,
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
   runs the `tool.gate` chain M131 built; maps `Gated` to `Verdict`.
   Raced inside the turn's supervised future (ADR-0068): esc drops a
   waiting permission and the agent is told reject on the way out.
4. **The agent turn.** `bingo-core/src/turn/agent.rs`: one `run` per
   turn. `Said` goes through the same stream fold the model turn uses
   (`turn/stream.rs`), `Context` and `Compacted` as ADR-0055 §2–§4 read
   them today; `Call`/`Update` start and update an external `ToolCall`
   item; `Done` passes `tool.run` as an observer (ADR-0076 §4) and ends
   the item. `run_turn` picks the driver by what the resolved id names.
5. **Composition.** `Registrar::agent`; a provider id registered as both
   is a `KernelError` at build; model resolution and the catalogue list an
   agent's `models()` as they list a provider's. `ModelRequest.session`
   stays for a stateful `Provider` (ADR-0035 §3 named OpenAI's).
6. **The ACP crate becomes an agent.** `provider.rs` → `agent.rs`;
   `events.rs` emits `AgentEvent::Call/Update/Done` and drops `Call::
   metadata` and the reasoning path; `question.rs` → `permission.rs`
   over `Judge` and `narrowest`; pictures cross as ACP image blocks when
   the agent declared `promptCapabilities.image`, else a note says so
   (audit §6: today they are dropped). `session/cancel` from the
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
- the record upgrade module M113 built (re-verify its path)
- `crates/bingo-provider-acp/src/{lib.rs,provider.rs→agent.rs,events.rs,events_tests.rs,question.rs→permission.rs,render.rs,fixtures.rs,guide.md}`
- `crates/bingo-checkpoints/src/hook.rs`
- `crates/bingo-surface-tui/src/{acp.rs (deleted),screens/acp.rs (deleted),transcript.rs,fold.rs,thoughts.rs,pager.rs,screens.rs,lib.rs,test_support.rs}`
- `crates/bingo-surface-print/src/render.rs` (an external call prints as a tool line)
- `holds_context` literals in `bingo-{context,experience,persona,plugin-rpc,rooms,skills,tasks}`
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
- [ ] `rg holds_context crates` finds nothing; `rg '"acp"' crates/bingo-surface-tui` finds nothing.
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
- Re-verify at start: M131's `tool.gate`/`tool.run` chains and observer
  rights; ADR-0068/M116's supervised turn; M112's fold (where the stream
  fold now lives) and M113's record upgrade; M109/M115 for the picture
  representation (`input` parts); M111's test host; whether M108 already
  sends ACP pictures. Line references are from `979a6d27`.
