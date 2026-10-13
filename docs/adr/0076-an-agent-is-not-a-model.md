# ADR-0076 — An agent is not a model

Status: accepted · 2026-10-13 · Plan: M134 · Amends: ADR-0035 §1, §4, §5; ADR-0039 §3; ADR-0055 §1

## Context

ADR-0035 let an ACP agent join as a `Provider`, and every fact that does
not fit a model was bent to fit (audit §6). The agent runs its own tools,
so its calls are journaled as reasoning items with the call hidden in
`acp` metadata, and the TUI re-parses that metadata into tool rows
(`bingo-surface-tui/src/acp.rs`, 425 lines, plus its screens): two
representations of one tool call, joined by field names copied across a
crate boundary. The agent keeps its own context, so `holds_context` was
added to two capability types and the turn still assembles a request the
agent never reads. The agent asks permission, so `session/request_permission`
became a free-standing `Question` (ADR-0039 §3) that no `BeforeTool` hook,
no checkpoint and no permission rule sees: a person's `deny` rule for
`rm` does not reach an agent's `rm`.

**The ratchet.** This adds a trait the kernel calls (`Agent`), a
`Contribution` arm, and a turn driver in core. Refusing it keeps the call
twice (reasoning metadata and the TUI's re-parse), keeps a history built
to be thrown away, and keeps a permission decision outside the policy
that owns permission decisions. Yes: the door opens; `holds_context`
closes behind it.

## Decision

1. **An agent is its own seam.** `bingo-sdk` gains `Agent { id(),
   models(), run(AgentTurn) -> AgentStream }` and `Contribution::Agent`.
   `AgentTurn` is `{ session, cwd, input, model, reasoning, judge }`:
   the newest input only — text and pictures — the agent's own model and
   effort knobs (ADR-0037), and `judge: Arc<dyn Judge>`. A provider id
   names a `Provider` or an `Agent`, never both; registering both is a
   composition error. Dropping the stream is the cancel (ADR-0068): the
   ACP agent sends `session/cancel` from its drop.
2. **What an agent says.** `AgentEvent` is `Said(ModelEvent)` — text,
   reasoning, `Usage`, `Context`, `Compacted`, the finish; a `ToolCall`
   there fails the turn in words — `Call(ExternalCall)` when it starts
   one of its own, `Update(ExternalCall)` as it changes, and `Done
   { call, output }`. `ExternalCall` is `{ id, kind, title, locations,
   raw }` with `kind` a closed sdk enum (`Read`, `Edit`, `Delete`,
   `Move`, `Search`, `Execute`, `Think`, `Fetch`, `Other`).
3. **The kernel journals an agent's call as a tool call.** One
   `ItemBody::ToolCall` named `<agent id>:<kind>` with input `{ title,
   locations, raw }`, marked external (`Item::mark_external`, the mark a
   handed-in call already wears: the provider fold skips it). Its result
   is the reported output. Every surface draws it as the tool row it is;
   `bingo-surface-tui/src/acp.rs` goes. A journal written before this
   reads its old reasoning-with-`acp.external` items as these calls
   through the record's upgrade step (ADR-0065), so an old session draws
   the same.
4. **A call the agent ran passes `tool.run` as an observer.** Its kernel
   link returns the reported output; every link at it holds `tighten`'s
   rights only (the agent already acted on its result). `AfterTool`
   hooks see it; a `Block` ends the agent's turn.
5. **A permission the agent asks for is judged by `tool.gate`.** The
   `Judge` runs the `tool.gate` chain of ADR-0074 — links, hooks, policy,
   person — for a stand-in call whose traits come from a fail-closed
   table by `kind` (`Read`/`Search` read-only; `Edit`/`Delete`/`Move`
   edit, `Delete` destructive; everything untrusted; `Other` nothing).
   The verdict maps back to the agent's own options: allow → its
   narrowest allow option; allow for the session → also its narrowest
   allow (bingo's policy holds the session's scope); deny or no answer →
   its narrowest reject. ADR-0035 §5's "the row speaks first" stands: an
   agent told not to ask never reaches the judge.
6. **An agent holds its context by being one.** `holds_context` leaves
   `EndpointCapabilities` and `ModelCapabilities`; the agent turn sends
   no history and draws no line. ADR-0055 §2–§4 (a reading beats a bill,
   the agent's own cut is a row) move to the agent turn unchanged.

## Consequences

- The turn has two drivers in core: rounds for a model, one `run` for an
  agent. They share the stream fold for what is said and the journal.
- A checkpoint is taken before an agent's edit that asked: checkpoints
  read `locations` through `ExternalCall::from_input` when `kind` writes.
  An edit the agent makes without asking is outside `/rewind`; the guide
  says so.
- Permission rules name an agent's calls as `acp/claude:execute(…)`; a
  rule for `Bash` does not allow an agent's `Execute` (fail closed).
- `bingo-agents` children and room members driven by an ACP row change
  nothing: they open sessions on a provider id, and the id now names an
  agent. ADR-0036's bridge (bingo's tools served to the agent) is
  untouched; those calls are bingo's and run both chains.
- `HostApi::ask` stays for every other asker; only ADR-0039 §3 moves.

Refs: ADR-0035, ADR-0036, ADR-0037, ADR-0039, ADR-0055, ADR-0065,
ADR-0068, ADR-0074, audit-2026-10 §6.
Non-goals: ACP plans, modes and slash commands; `fs/*` and `terminal/*`
served to the agent; rewinding an agent's unasked edits; an agent
speaking anything but ACP (the trait allows it; nothing builds it).

*2026-10-13, §6 clarification by the person's delegate: protocol 2 has
not shipped; remove `holdsContext` from both sdk types and regenerate
both schemas before the freeze starts at its first release (ADR-0075).
Unknown legacy fields deserialize without effect. A legacy `true` emits
one Info notice at plugin load naming the plugin: an endpoint holding
its own conversation is an agent. Fixtures pin absent and legacy true.*

*2026-10-13, §1 clarification by the person's delegate: `AgentTurn` also
carries `tools: Vec<ToolSpec>`, the turn's already resolved offer. ADR-0036
§1 requires the unchanged bridge to offer exactly those tools; this adds
no history, host reader or second selection.*

*2026-10-13, §1 clarification by the person's delegate: ADR-0009 §6's open
retry is a model's, not an agent's: the agent may already have acted. A child
stopping mid-answer fails the turn without withdrawing speech or work and
without `TurnRetrying`. It is buried with one `ACP_RESPAWN`; the next prompt
uses a new child. Before anything was said, the plugin may still replace a
dead child and ask again inside the same run.*

*2026-10-13, at the merge: ADR-0010's check (`agent` is a plugin noun) now
exempts only this seam's modules and their tests. The agent turn opens the
bridged-call door around its stream, as a model's round trip does.*
