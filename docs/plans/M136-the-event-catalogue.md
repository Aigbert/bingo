# M136 — The event catalogue

## Goal

ADR-0078 §1. Every event a link can hook gets one name, `<noun>.<verb>`,
and one entry in `bingo_sdk::catalogue`, generated to `schema/events.json`
and `sdk/typescript/src/events.ts` in Claude Code's spelling. New chains
and observations join ADR-0074's seven at existing call sites, and a service
that declares its face becomes a noun whose calls are events. The runtime
is M137's. With no link installed, a person sees no change.

## Bricks, in build order

1. **Contract and failing tests** (`bingo-sdk`, `catalogue.rs`).
   `Entry { name, kind: Chain | Observation, input, result, matcher,
   pinned, acts }`: `input`/`result` are `fn() -> Schema` over the Rust
   type, never hand-written JSON; `matcher`, the fields the host
   pre-filters on before crossing; `pinned`, JSON pointers no link may
   change; `acts`, which of deny, rewrite and answer a link may do.
   `catalogue!` defines each `pub const` and `ALL` from one list; M131's
   `LinkPoint` becomes `&'static Entry`, its seven first. Raising takes
   `&'static Entry`, so an unnamed kernel event does not compile, and
   `check_discipline.sh` fails an `Entry {` literal outside
   `catalogue.rs`. A `BINGO_UPDATE_SCHEMA` test writes
   `schema/events.json`, as for `schema/rpc.json`; it fails first.
2. **Claude Code's spelling** (ADR-0079 §1). Each entry's types are named
   as Claude Code names them (`ToolCall`, `Turn`, …) and carry its field
   names through serde, so `e` reaches a mod unmapped. `tool.call`'s `e`
   is the tool's input flattened beside the pinned `tool` and
   `tool_use_id`, and its result carries `isError`. `turn/late.rs`
   `gather` drops, with one notice, a plugin tool whose input names
   either field. `crates/bingo/tests/catalogue.rs` composes the built-in
   plugins and fails if any of their tools' input schemas does.
3. **Pure bricks, table-tested.** `Pattern::parse` (a name, `ns.*`, `*`,
   `!p`) and `matches`; `prefilter(scalars, e)` over the `matcher` fields,
   never excluding what the full matcher admits; `pinned_changed(before,
   after, pinned)`; M131's `admits` read through `acts`. Patterns and the
   scalar matcher share M133's `on.ts` case table, moved to
   `sdk/typescript/test/cases.json`: host and runtimes, one table.
4. **The JSON chain.** `Interceptor::chain(entry, cx, Value,
   Next<Value>)` defaults to `next.run(input)`, `observe(entry, cx,
   &Value)` to nothing; `LinkMatcher` becomes `{ patterns, prefilter }`.
   M131's `chain::run` checks each link's act and pinned fields against
   the entry and skips an overstepping link with one notice (plugin,
   event, field). The seven typed methods' defaults serialize into
   `chain` (a wrong shape is a skipped link), so a runtime that
   implements only `chain` hooks every point.
5. **New kernel chains**, each a kernel link around an existing call site, one
   module each under `chain/`:
   - `tool.describe`: the round's specs after `gather`, matcher `name` (the existing ToolSpec field);
     a link rewrites or drops a spec, never adds one (`name` pinned).
   - `prompt.autocomplete`: `HostApi::catalog`'s answer (`host.rs:904`);
     matcher `kind`.
   - `session.send` at `HostApi::deliver` (`host.rs:792`), before the
     mailbox, matcher `to`; `session.receive` at the actor's `deliver`
     (`session/inputs.rs:169`), matcher `origin.surface`. One delivery is
     raised once out and once in.
   - `session.append`: the actor's `extend` (`session.rs:230`), matcher
     `plugin`, `kind`; rewrites only of the link's own plugin's rows.
   - `turn.complete`: at the turn's end (`session.rs:800`), on the turn's
     id, outcome, usage and answer text. Its result is text drawn under
     the answer: the kernel link answers none, `next` resolves to what
     the links beneath answered, and a non-empty answer is appended as an
     `Item::Notice` after `TurnCompleted`.
   - `command.run`: `session/commands.rs` `spawn`; matcher `name`.
     `command.describe`: the `Commands` catalog entries.
   - `config.set`: one `config::set` that model/thinking commands and the live
     plugin switch call, running the chain before anything is
     reconfigured or remembered; matcher `key`. `config.describe`: the
     `ConfigView` before `Event::ConfigChanged` is sent.
6. **Observations.** A pure `observed(&Event) -> Option<(&'static Entry,
   Value)>` reads `turn.start` (`TurnStarted`) and `session.measure`
   (`TurnUsage`'s context reading) off the stream; the payload is the
   frame's own fields. `session.start` and `.end` come from `on_session`
   (`session.rs:904`); `session.attach` and `.detach` from the client port's creation/drop,
   carrying its existing `ClientIdentity`. `HostApi::close` has no identity
   and does not detach; the port owns the actual attachment lifetime.
   Observers run after the fact under the session's owner (M116).
7. **Faces.** `WireService::face(&self) -> Option<Face>`, default `None`;
   `Face { noun, methods }`, each method an `Entry` built at registration
   with its input and result schema and whether it is unsafe (ADR-0078
   §5). The external `ServiceSpec` gains `noun`, each method's `result`
   and `unsafe`, additively. The registry refuses a method whose
   `<noun>.<method>` the catalogue or another face already names, naming
   both owners: a face may share a noun with the kernel, never a name.
   `HostApi::service_wire` returns the face wrapped, so a call raises
   `<noun>.<method>` with the owner's call as the kernel link; plugin-rpc's
   `service/call` already goes through it. The `bingo-services` adapters
   declare faces, so the first noun events
   (`team.section`, `mcp.servers`) are tested against real owners.
8. **The wire**, additions to protocol 2: `intercept/<event>` takes any
   chain entry; additive `subscriptions: LinkMatcher[]` and negotiated
   `intercept/*` cover dynamically registered faces; observations cross as `hook/observe { event, payload }`.
9. **TypeScript.** `bingo-schema-ts::render` writes `events.ts` from
   `schema/events.json`, drift-tested. It exports each event's types
   under Claude Code's names; `on.ts` types `on(name)` by it, and the
   `'bingo'` module re-exports them (M137).

## Files

- `crates/bingo-sdk/src/{catalogue.rs (new),intercept.rs,service.rs,lib.rs}`
- `crates/bingo-core/src/chain/{describe,autocomplete,deliver,append,complete,command,config,observe}.rs` (new), `chain/mod.rs`
- `crates/bingo-core/src/{host,session,plugins,config (new),commands/mod,turn/late,session/inputs,session/commands}.rs`
- `crates/bingo-services/src/{team,mcp}.rs` (M119 owns both wire adapters).
  Its existing workspace `schemars` dependency is needed to derive each
  adapter's face schemas from its Rust payloads; no new external crate.
- `crates/bingo-core/src/session/{gate,mailbox,catalogue (new)}.rs` keep
  interception on owned tasks; `host/interception.rs` keeps host.rs within its size gate;
  `commands/{model,think,plugins}.rs` put
  config interception before reconfiguration. Mechanical matcher fixtures
  in sdk/core/plugin-rpc tests and the stub plugin follow the new type.
- Dynamic face entries own declared schemas and names; static kernel entries
  retain Rust schema functions. The chain borrows entries without leaking them.
- `crates/bingo-plugin-rpc/src/{intercept,wire,bridge,host,service,schema,lib,events_ts (new)}.rs`
- `crates/bingo/tests/catalogue.rs` (new), `schema/events.json` (new), `schema/{plugin,plugin-2}.json`
- `sdk/typescript/src/{events.ts (generated),on.ts,on.test.ts,plugin.ts,rpc.ts,index.ts}`,
  `sdk/typescript/test/cases.json`, `scripts/check_discipline.sh`

## Exit criteria

- [ ] `cargo test -p bingo-sdk catalogue::` passes; deleting one entry
      from `schema/events.json` fails it (shown once, pasted).
- [ ] `cargo test -p bingo --test catalogue` passes; a fixture tool whose
      input has `tool_use_id` fails it (shown once).
- [ ] `cargo test -p bingo-core chain::`: a `rewrite` link changes
      `tool.describe`'s description and `session.receive`'s text; a
      pinned change, or a `tighten` answer at `command.run`, is skipped
      with one notice; a `turn.complete` link's text shows under the
      answer, its `next` resolving to the text of the link beneath.
- [ ] On M111's test host, an observer of a one-turn session sees, in
      order, `session.start`, `session.attach`, `turn.start`,
      `session.measure`, `session.detach`, `session.end`.
- [ ] A link sees `mcp.servers` when `mcp`'s face is called through
      `service_wire`; a second face naming it is refused, both named.
- [ ] `events_ts::` (plugin-rpc) fails on a hand edit of `events.ts`;
      `check_discipline.sh` on a stray `Entry {`. Each shown once.
- [ ] Existing hook, chain, intercept and plugin tests pass unedited;
      except the authorized stop-name rename; all gates; the MSVC `cargo check` for sdk, core and plugin-rpc.

## Non-goals

- The runtime and `plugin.register` (M137); the `$` methods' op events,
  which land as faces (M138, M139); `ui.*` (M140, M141); `engine.create`,
  `classic.*`, `telemetry.*`.
- `prompt.mention` and `prompt.attachment` (mods-parity §8): surfaces
  resolve `@`, and a hook sees the result in `prompt.submit`'s input.
- Noun events on a typed lookup (`service::<T>`): only the wire face
  raises, so an owner calls itself through `service_wire` (M139, agents).

## Risks

- Resolved by the user, 2026-10-13: protocol 2 has never shipped. Amend
  it in place: old `turn.complete` becomes `turn.stop` without changing
  its shape or behaviour; `turn.complete` takes the catalogue contract.
  Regenerate current schemas with `BINGO_UPDATE_SCHEMA=1`; amend only
  these entries and their referenced definitions in `plugin-2.json` in
  one commit. Keep the additive-only test and record ADR-0075's dated
  exception. A change after the first protocol-2 release needs protocol 3.
- Gather links once per round; chain config before writes; never measure
  `UsageCharged`; keep actor plugin work on M116-owned tasks.

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
