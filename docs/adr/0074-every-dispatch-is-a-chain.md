# ADR-0074 — Every dispatch is a chain

Status: accepted · 2026-10-13 · Plan: M131 · Amends: ADR-0032 §4

## Context

The kernel dispatches in a handful of places — a submitted input, a tool
call's gate and its run, the model request, the composed context, a
compaction, the decision to end a turn — and a plugin reaches each only
through a `Hook` with a fixed answer (`Continue`, `Deny`, `Ask`, `Block`,
`Redirect`). A plugin can refuse; it cannot wrap. To answer a tool call
for the engine, log a model request or rewrite a composed context, it
must shadow a tool by name or wrap a `Provider` — a second copy of the
dispatch, and of the fact of who answered. Each site also combines its
hooks its own way (`session/inputs.rs` submit, `gate.rs` `run_hooks`,
`turn.rs` `after_tool_hooks` and `stop_hooks`), unwritten, and the gate's
awaits are not raced, so `esc` does not reach a hung hook (audit §3).
`HostApi::invoke` gates a handed-in call but never runs `AfterTool`.

Claude Code's mods are one middleware chain per event with the engine's
behaviour at the bottom (audit, Mods gap 1). bingo wants the same shape
without loosening ADR-0032 §4 for a process nobody vetted.

**The ratchet.** This adds a trait the kernel calls (`Interceptor`) and a
`Contribution` arm. Refusing it leaves every wrapper to re-implement the
dispatch it wraps — a provider that routes, a tool that shadows — a second
representation of which tool or model answered, and the journal records
the wrapper as if it were the thing. Yes: the door opens.

## Decision

1. **Seven points, one chain each**, named as ADR-0078's event
   catalogue names them: `prompt.submit`, `tool.check` (the gate),
   `tool.call` (the run), `turn.step` (the model request),
   `prompt.compose`, `session.compact` and `turn.stop`. A link is
   `(cx, input, next) -> output`; `next` is consumed by value, so it is
   called at most once, and the kernel's own behaviour is the last link
   (the actor's submit, policy-then-person, the executor, the provider,
   the composer, the compactor, closing the turn). Links run in plugin
   order — the order two in-process hooks already run in (ADR-0032 §2).
   At `tool.check` the permission policy is a link at a fixed place:
   after every plugin link, before the person.
2. **The `Hook` trait stays, as an adapter.** A hook is a link that may
   observe and tighten. Its combination rules are today's, now written
   as the adapter's contract with one test each:
   - `Submit`: hooks in order, each sees the input as the last left it;
     `Ask` is `Continue`; the first `Deny`/`Block` (rejected as
     `PermissionDenied`) or `Redirect` (delivered elsewhere) ends the run.
   - `BeforeTool` (at `tool.check`): matched by exact name or `prefix*`;
     `Redirect` is `Continue`; the first `Deny`/`Block` ("Denied by hook
     `<id>`: …") or `Ask` ends the run, and an `Ask` still lets the policy
     deny before a person is asked.
   - `AfterTool` (after `next` at `tool.call`): every matched hook sees
     every result; any `Block` ends the turn after the round.
   - `Stop`: the first `Block` adds a user piece with origin
     `hook:<id>` and another round.
   `Turn`, `Compact`, `Session` and `Event` stay notifications: no chain,
   no answer read.
3. **Trust tiers decide what a link may do.**
   - `tighten` — an external plugin's default: exactly ADR-0032's rights
     (rewrite the input at `prompt.submit` and `tool.check`, deny, ask,
     block),
     never an allow; at the other five points it observes only.
   - `rewrite` — may also rewrite inputs and results anywhere and answer
     for the engine (not call `next`) at `tool.call`, `turn.step`,
     `prompt.compose` and `session.compact`; at `tool.check` its verdict
     may only
     be as strict as `next`'s or stricter (Deny > Ask > Allow).
   - `full` — may also allow at `tool.check`. Every in-process plugin is
     `full`: it is compiled into the binary the person runs.
   An external link is registered with the name of the process it speaks
   for; the kernel, not the plugin, reads that name's tier from
   `pluginTrust` (`{"<name>": {"tier": "rewrite" | "full", "uses":
   [...]}}`, beside
   `enabledPlugins`) **in the user layer only** — a project layer cannot
   grant a repository trust in itself, and an absent name is `tighten`.
   A link that oversteps its tier is skipped for that call with a notice.
4. **The chain runs inside the turn's supervised future** (ADR-0068): one
   `esc` drops a hung link with everything else in flight. An external
   link keeps the `HOOK` deadline per call; a link that errors or misses
   it is skipped with a notice (ADR-0032 §5), and the chain goes on.
5. **`HostApi::invoke` runs the same two chains**, `tool.check` then
   `tool.call`, so `AfterTool` observers see a handed-in call.

## Consequences

- One file per point in `bingo-core` owns its chain and its kernel link;
  the four hand-written combination loops go, and the rules survive as
  tests rather than as code that happens to agree.
- A test plugin can answer a tool call for the engine without shadowing
  its name; a checkpoint, a logger, a redactor become ordinary links.
- A `rewrite` link can change what the model reads and what the journal
  records. The tier is a person's grant, said in the words of
  `bingo plugins list`, and the receipt of a rewritten call names the link.
- ADR-0032 §4's "tighten-only is the type's own law" becomes "the
  default tier's law"; `HookOutcome` still has no `Allow`.
- Per-call cost: an empty chain is the kernel link and nothing else; a
  matcher still rules a link out before anything crosses a process.

Refs: ADR-0032, ADR-0057 §1, ADR-0068, audit-2026-10 §3 and Mods.
Non-goals: a chain at an observation point; links that reorder
themselves; policy over the wire below `full`.

Revised 2026-10-13 for ADR-0078: the points take the catalogue's names,
`pluginTrust` carries `uses` (ADR-0078 §5), and the in-process JS engine
this record refused is ADR-0078's.

Verified 2026-10-14 (M136): the catalogue owns event names, shapes, pins and
authority rules. The seven typed points bridge through JSON chains; the stop
hook is `turn.stop`, while `turn.complete` composes completion text.

Verified 2026-10-14 (M137, ADR-0078): module links own their active-time budget; process links keep their deadline and tighten default. Only user/explicit modules default to rewrite; user-layer pluginTrust can lift either runtime.

Verified 2026-10-14 (M138, ADR-0078 §4): completion and configuration cross their kernel chains once; reverse RPC and actor work carry the calling hook so only that hook is omitted.

Verified 2026-10-14 (M139, ADR-0078): world methods cross one noun chain; model SpawnAgent uses agent.spawn. Shared unsafe admission retains process tighten and user/explicit module rewrite defaults.
