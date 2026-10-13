# ADR-0068 — Every task has an owner

Status: §1–4 accepted (M116); §5 proposed (M117) · 2026-10-13 · Amends: ADR-0002 §2,
ADR-0008 §2, §7, ADR-0035 §6

## Context

AGENTS.md says one esc ends the turn and drops every call in flight. In
the tree that is true where someone remembered a `select!`, and only
there:

- the turn awaits the gate unraced — `hooks.gather()`, then per call
  `before_tool` hooks, `policy.decide` and `prompter.ask`
  (`bingo-core/src/turn.rs` `gate`, `bingo-core/src/gate.rs`) — and the
  compactor (`compactor.compact(cx, reason).await`, handed a child token
  it may ignore) and its end hooks;
- a command runs on a bare `tokio::spawn` (`session/commands.rs`
  `Commands::spawn`) with no token and no kept handle, may hold the
  queue (`holding`), and is on no tracker; an interrupt with no turn
  running answers `NOT_READY` (`session/inputs.rs`) while that command
  holds the queue forever;
- the actor itself awaits plugin work inline: `commands.find` (a
  contribution source may do I/O), `hooks.at(Submit)` and each
  `on_submit` hook, `redirect` into another session, and
  `store.append`, which is synchronous `std::fs` on the runtime thread
  (session and event hooks already run on the tracker);
- nothing bounds a call into another process: MCP `call_tool`,
  plugin-rpc `tool/call` and `command/run`, the ACP agent after
  `session/cancel`, two bare HTTP clients (the codex token source, MCP
  auth).

ADR-0008 §2 says "a command runs on its own task and reports back by
mail; the actor never awaits it" — half of what is needed.

## Decision

1. **A turn is a supervised future.** The turn task races the turn
   against its token. On cancel the turn has `INTERRUPT_GRACE` (2 s) to
   close itself as it does today; after that the future is dropped
   where it stands and the actor closes the turn: every item the
   transcript still shows open is completed as interrupted, then
   `TurnCompleted { Interrupted }`. The actor can do this because every
   item the turn started reached it by mail. Esc reaches every await by
   construction, including ones nobody raced.
2. **A command is owned like a turn.** Commands run on the actor's existing
   `TaskTracker`, with a child `CancellationToken` per run and an
   intent-to-token map. Each run races like the turn: token, then
   `INTERRUPT_GRACE`, then drop. Close cancels the root token and joins
   through `tracker.wait()` under `AFTER_TURN_DEADLINE`.
   Interrupt stops a running turn first; else the command holding the
   queue; else the input in flight at the gate's head. A stopped command
   or gate input has its intent rejected `INTERRUPTED` (the person
   stopped it; not a failure), and inputs queued behind it proceed in
   order. Only with none of these is the answer `NOT_READY`. One esc
   always stops what the person is waiting on.
   *(Revised 2026-10-13 at M116's start: TaskTracker, not JoinSet; the gate head is interruptible.)*
3. **The actor awaits no plugin.** Work the actor needs from a plugin
   (`commands.find` on a source, the submit hooks, a redirect) runs on
   an owned task and reports back by mail, as
   commands do; the actor holds the input it is waiting on, so order is
   kept. The one await left in the actor is the journal write.
4. **The journal is written off the runtime.** A store does its file
   I/O on the blocking pool (`spawn_blocking`), never on the thread that
   polls the actor; the actor awaits the append before it applies or
   broadcasts (ADR-0002 §2, which M108 enforces). A failed write is
   M108's storage stop. The JSONL store still reopens the file per frame
   (`journal.rs` says why).
5. **Deadlines belong to whoever does the I/O.** The kernel's deadline
   is the person's esc (§1–2). A plugin that talks to another process
   bounds every call with a named constant and a per-server or
   per-plugin setting: MCP `call_tool` 10 min; plugin-rpc `tool/call`
   and `command/run` 10 min; an ACP agent gets 10 s after
   `session/cancel` before its process tree is killed; every HTTP client
   has a connect and a total timeout. Expiry is a tool error, not a
   hang.
6. **Non-goal: a bounded mailbox.** The actor sends to itself (commands
   and turns mail it back); a bounded mailbox would deadlock it on
   itself. Backpressure stays ADR-0002 §6's.

## Consequences

- A hung hook, policy prompt, compactor or command no longer takes the
  session with it; the worst case after esc is the grace.
- Items a dropped turn left open read as interrupted in every fold,
  because the actor closed them through the one transcript (ADR-0064).
- A plugin hook can no longer stall the session's mailbox; a slow
  `on_submit` delays only the input it guards.
- Journal I/O leaves the runtime threads; a slow disk delays its own
  session, not every session on the runtime.
- `ToolTraits` documentation stops naming an `interrupt` property that
  does not exist (`bingo-sdk/src/tool.rs`).
- ADR-0008 §2 gains a dated note: the actor owns the command's task.

Refs: ADR-0002, ADR-0008, ADR-0032, ADR-0064; Plans: M108, M116, M117;
audit-2026-10 §3
