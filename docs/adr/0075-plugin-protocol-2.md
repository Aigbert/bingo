# ADR-0075 — Plugin protocol 2

Status: accepted · 2026-10-13 · Plan: M132 (wire); M133 (author kit) · Amends: ADR-0015 §3 and Consequences, ADR-0030, ADR-0031 Consequences

Implementation note (2026-10-13): M132 is locally verified; Windows runtime
cleanup and C-header-blocked builds remain for Windows CI (protocol step 7).
M133 author-kit verification (2026-10-13) is recorded in its plan, including
authorized baseline exceptions and pending Linux/Windows CI evidence.

Pre-release amendment (2026-10-13, M136): protocol 2 has not shipped
(v0.6.6 ships protocol 1). Its old `turn.complete` becomes `turn.stop`
with unchanged shape and behaviour; `turn.complete` takes the catalogue's
metadata-to-completion-text meaning. Amend the frozen baseline for these
entries only. Any protocol-2 change after its first release that breaks
its frozen contract still needs protocol 3. M134's `holds_context` note
is on its parallel branch; both pre-release amendments apply.

## Context

ADR-0015 kept `PROTOCOL` at 1 "until a plugin outside this repository
speaks it"; the wire has changed in place six times since. Phase 5 exists
so that one can: the acceptance test is a TypeScript plugin, written
outside the workspace, that places a status entry, intercepts a tool call
and keeps its state across a hot reload. Today a process can answer the
host but ask it for one thing only — `service/call` (ADR-0031 said
anything more "is a future ADR's door"; this is it). Any other request
gets no reply at all (`connection.rs` `served()`), so a plugin that asks
waits forever. A plugin process is spawned with `kill_on_drop` and no
process group, so an `npx` entry leaves grandchildren (audit §5), and
there is no way to see a change to a plugin without restarting bingo.

**The ratchet.** Four of the new verbs land on doors that exist
(`signal`, `extend`, `deliver`; state is the plugin host's own, on the
file shelf of ADR-0072). One is new: `HostApi::complete(session, turn,
request)`, defaulted to refuse. Refusing it forces a plugin process to
carry its own credentials and its own model choice — a second copy of the
person's model settings and sign-in — and to spend outside the turn's
usage, which the kernel owns. Yes: that door opens, and no other.

## Decision

1. **`PROTOCOL` is 2, and 2 is frozen.** The committed
   `schema/plugin.json` at the landing commit is copied to
   `schema/plugin-2.json`; a test fails any later schema that removes or
   retypes a method, field or capability of it. Additions are negotiated:
   `initialize` carries the host's `capabilities` and the plugin's answer
   carries its own, and a verb is served only when both name it. A break
   is protocol 3, and from then the host speaks N and N−1. A plugin that
   says 1 is refused with a notice naming both numbers.
2. **What a plugin may ask the host**, each behind its capability:
   - `host/place {session?, slot, view}` — a view for a slot (ADR-0070),
     carried as a `signal` under the plugin's own key; `null` removes it.
   - `host/extend {session, key, value}` — a durable plugin record.
   - `host/deliver {session, input, wake}` — input to a session, with
     origin `plugin:<name>`; the gate judges what it leads to as ever.
   - `host/state/get|set {scope, key, value?, ifVersion?}` — a value the
     host holds for this plugin, `scope` a session id or `global`, with a
     version that `set` bumps and `ifVersion` compares. Held on the file
     shelf under the plugin's data directory: it survives a reload and a
     restart; a session's values go when the session is deleted.
   - `host/model/complete {session, turn?, request}` — one model call on
     the session's resolved model, billed by an additive `UsageCharged`
     event to the running turn, or to the session when `turn` is absent
     (a clock or a press; revised 2026-10-13 for ADR-0078, whose `model`
     grant it needs).
   - `tools/changed {tools}` — a notification replacing the plugin's
     whole tool list (replace-whole); commands likewise.
   - `service/call` stays. **Any request the host does not serve is
     answered `METHOD_NOT_FOUND`**, never with silence.
3. **Links cross as `intercept/<point>`** (ADR-0074), declared at the
   handshake like hooks with an `after` flag per point. The request
   carries `{chain, tier, site, input}`; the answer is `{next: input}` or
   `{answer: output}`. A link that declared `after` and said `next` gets
   `intercept/<point>/after {chain, result}` and answers `{result}`;
   `intercept/cancel {chain}` tells it the chain was dropped. The tier is
   the host's word (ADR-0074 §3), sent so a plugin can tell an ignored
   rewrite from a lost one. `hook/decide` is withdrawn: a decision point
   crosses only as a link; `hook/observe` stays. The plugin host offers
   its links as a source the kernel reads per round, each named by its
   process, as it offers tools — so a reload changes them in place.
4. **A plugin process is spawned by `bingo-process`** — its own process
   group on unix, a job object on Windows (ADR-0072) — and framed by
   `bingo-jsonrpc`. Stopping it stops everything it started.
5. **Hot reload.** `bingo --plugin-dir <dir>` (repeatable) names one
   plugin's own directory and says the same thing as the `pluginDirs`
   setting, as `--channels` does `channels`. It is the last layer of
   discovery, so its name wins, and only it is watched — by polling its
   files' modification times once a second (no new dependency). On a
   change the host lets in-flight calls fail in
   words, stops the process, spawns and handshakes it again, re-reads
   its tools, commands, contributors and links, keeps every value of §2's
   state, and posts a notice. A changed provider or service declaration
   needs a restart, and the notice says so.

## Consequences

- An author can write against a number that holds: every outside plugin
  speaks 2 and keeps working while the host grows.
- The in-repo examples (`wordcount`, `stub_plugin`) move to 2 in the same
  change; nothing else in the world speaks 1.
- The plugin host grows a reverse-request table: one handler per verb,
  each a translation onto an existing door or the shelf.
- State a plugin keeps is not rewound by `/rewind` and not in the
  journal; a plugin that wants a fact in the transcript uses `extend`.
- `host/model/complete` spends the person's money on a plugin's say-so;
  it is its own capability so `bingo plugins list` can show who asks.

Refs: ADR-0015, ADR-0030, ADR-0031, ADR-0032, ADR-0070, ADR-0072,
ADR-0074, audit-2026-10 Mods.
Non-goals: a marketplace or publishing; process to process pipes; a
plugin spawning agents (it delivers to a session, whose model may).

Revised 2026-10-13 for ADR-0078: the embedded JS engine this record
refused is ADR-0078's; the TypeScript SDK speaks ADR-0078 §2's
`register(on)` so one module runs in either transport.
