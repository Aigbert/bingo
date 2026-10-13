# M134 — An agent is not a model
## Goal
ADR-0076: an ACP agent becomes an `Agent`, not a `Provider`. Its own tool
calls are journaled as external tool calls every surface draws without a
re-parse; a permission it asks for runs the `tool.check` chain (links,
hooks, checkpoints, policy, person); the agent turn sends the newest input
and no history; `holds_context` is deleted. An old journal draws the same.
Audit §6.
## Bricks, in build order
1. **Contracts first:** sdk Agent/AgentTurn/AgentStream/AgentEvent/Judge and
   ExternalCall/CallKind; Contribution::Agent and a golden journal spelling.
   Red core, legacy-plugin and historical-store fixtures precede implementation.
2. **Judge:** fail-closed kind traits and ACP narrowest-option mapping;
   stand-in calls run tool.check, hooks, policy and person, raced with cancel.
3. **Agent turn:** one run, newest input and ToolSet::gather's tools; Said
   shares the accumulator; external Call/Update/Done journal ordinary ToolCall
   items. tool.call links have tighten rights; AfterTool may end the turn.
4. **Composition:** an id names Provider or Agent, never both; resolve and
   catalogue both. Agents compact their own context and refuse model side calls.
5. **ACP:** rename provider/question to agent/permission; preserve pictures,
   restore ladder, knobs, reported context/compactions and drop cancellation.
   Pass resolved tools to the unchanged bridge. Remove the private window cache.
6. **Capability removal:** remove holds_context and literals; generate both
   plugin schemas. Ignore legacy input; one named Info notice for legacy true.
7. **Records:** version-3 journal migration upgrades 979a6d27-format metadata
   to external ToolCall; invalidate version-2 checkpoints for replay.
8. **Surfaces/checkpoints:** remove ACP-only TUI parser/screens; render titles
   through the ordinary tool path; snapshot every reported write location.

## Files
- `crates/bingo-sdk/src/{agent.rs (new),plugin.rs,model.rs,lib.rs,store/checkpoint.rs}`
- `crates/bingo-core/src/{judge.rs (new),turn/agent.rs (new),turn/endpoint.rs (new),turn/host.rs (new),turn.rs,turn/stream.rs,session.rs,session/{complete,invoke}.rs,spec.rs,host.rs,commands/model.rs,models/{resolve,learned}.rs,test_support.rs}`
- `crates/bingo-store-jsonl/src/journal/{migrate.rs,versions.rs}` and
  journal.rs, fixtures and checkpoint tests (journal/checkpoint versions 2 → 3)
- `crates/bingo-provider-acp/src/{lib.rs,provider.rs→agent.rs,events.rs,events_tests.rs,question.rs→permission.rs,render.rs,fixtures.rs,guide.md,windows.rs (deleted),transcript.rs}`
- `crates/bingo-core/src/{accumulator.rs,chain/tool.rs,turn/config.rs,turn/late.rs,host/{registry,choose,catalog,refresh}.rs}`
- `crates/bingo-provider-acp/src/{inbox.rs,refusal.rs,session.rs}` and `tests/{pictures.rs,fake_agent.rs,deadlines.rs,harness/mod.rs}`
- `crates/bingo-checkpoints/src/hook.rs`
- `crates/bingo-surface-tui/src/{acp.rs (deleted),screens/acp.rs (deleted),transcript.rs,fold.rs,thoughts.rs,pager.rs,screens.rs,lib.rs,test_support.rs,test_support/external.rs (new)}`
- `crates/bingo-surface-print/src/render.rs` (an external call prints as a tool line)
- `holds_context` literals in `bingo-{context,experience,persona,plugin-rpc,rooms,skills,tasks}`; mechanical constructor fixtures per protocol step 6
- `crates/bingo-plugin-rpc/src/{wire.rs,schema.rs,bridge.rs,legacy.rs (new),lib.rs}, legacy fixtures` and generated `schema/plugin.json` and generator-produced `schema/plugin-2.json`
- `crates/bingo/tests/{acp_asked.rs,cli/acp.rs,cli/acp/{asking,bridge,life}.rs}`
- `docs/adr/{0009,0035,0039,0055}-*.md` (dated notes)
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
  Actual TUI files changed: the TUI paths listed in Files, plus the four
  deleted `screens__acp_*` snapshots. No layout code or M133-owned code.

## Decisions and verification status (2026-10-13)

The delegate chose no open retry for an agent (ADR-0009 §6, ADR-0076).
Mid-answer loss fails with a reason, retains speech/work, buries the child
with one `ACP_RESPAWN`, and serves the next prompt on a new child.
`cli/acp/life.rs` is authorized; its silent-death test is unchanged.
`TurnHost` moves verbatim to `turn/host.rs`, matching dev’s module boundary.

## Verified (2026-10-13, incomplete — protocol stop)

Sequential gates, pasted output:
```
cargo fmt --all -- --check
fmt exit=0
cargo check --workspace --all-targets --locked
Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.68s
cargo clippy --workspace --all-targets --locked -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.89s
cargo test --workspace --locked -- --skip <three bridge token tests>
test exit=0
cargo test -p bingo-provider-acp --test bridge token --locked -- --test-threads=1
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out
scripts/check_discipline.sh
FAIL crates/bingo-surface-tui/src/test_support.rs: 1029 non-test lines (>1000)
FAIL crates/bingo-core/src/turn.rs: 1005 non-test lines (>1000)
FAIL crates/bingo-sdk/src/event.rs: 1004 non-test lines (>1000)
discipline FAILED
```
The full suite includes every PTY test; none flaked. The three token tests
were explicitly excluded there and then run alone as above. Focused ACP CLI:
`40 passed; 0 failed`; permission interactions: `2 passed; 0 failed`.
**Decision (delegate, 2026-10-13):** leave sdk `event.rs` unchanged. Its
1004 lines are baseline, fixed on dev by `98d7355d` (interactions extracted).
It is the sole allowed discipline failure. Split M134's TurnHost contract
and external TUI fixture into their own modules; finish all remaining gates.
