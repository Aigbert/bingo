# Mods parity — 2026-10

> What a Claude Code mod can do, what bingo has, and which milestone
> closes each gap. The source is the mods API that Claude Code build
> 2.1.296 ships with its authoring skill: `types/claude-code.d.ts`,
> 21 461 lines, plus `reference.md` and the examples. Line numbers below
> are that file's. Written 2026-10-13 for ADR-0078 (the runtime),
> ADR-0079 (drawing) and ADR-0080 (the desktop app, rejected the same
> day: the GUI stays in its own repository). Phase 7 of
> [upgrade-2026-10.md](upgrade-2026-10.md) implements it.

## 1. The contract every event shares

- **The module.** A mod is one ES module. It exports
  `register(on, options)`, whose return value is ignored; a returned
  promise is awaited (9309).
- **`on(pattern, matcher?, hook)`** (6779).
  - A pattern is an event name, `*`, `ns.*` or `!pattern`.
  - Registering the same hook twice throws.
  - The returned registration takes exactly one `.catch`.
- **A hook is `($, e, next)`.**
  - `e` is frozen to every depth.
  - `next({ ...e, x })` rewrites what the rest of the chain sees.
  - Returning without calling `next` answers for everything beneath.
- **A matcher is a partial of `e`** (5808).
  - A scalar matches with `===`, a RegExp tests the string, an array
    matches any of its elements, and an object matches as a nested
    partial.
- **`next` carries** (6543) `to(e, tier)`, `signal`, `is(pattern, e)`,
  `event`, `origin {plugin, tier}`, `trace[]`, and
  `budget {ms, remainingMs}`.
- **Order** (12618). The tiers are `prepend`, `user`, `append`,
  `builtin` and `core`, outermost first. Within a tier, modules run in
  list order, managed plugins first. The order between two `user`
  plugins is not specified.
- **Failure is fail-open.**
  - A hook that throws, overruns or answers the wrong shape is skipped.
  - `.catch` gets `next.error {kind: throw|timeout|re-entry}` and
    `next.called`.
  - A deny after `next` has run undoes nothing.
- **Budgets** (5107).
  - A hook gets 10 s of its own time. The clock stops during `next` and
    `$` calls.
  - `catch` gets a further 1 s.
  - A hook may linger 5 s after its signal aborts.
- **Re-entry.** A hook never runs under its own frame. A `$` call raises
  the chain again, skipping only the calling hook.

bingo today: seven planned chain points (ADR-0074) with
`(cx, input, next)` links and trust tiers. Phase 7 adds the event
catalogue, the module, matchers over `e`, `.catch`, budgets on a
hook's own time, and re-entry.

## 2. Events

Claude Code has 141 fixed events: 46 engine call sites, 62 op events
(one per `$` method, every one of them rewritable, deniable or
answerable) and 33 `classic.*` settings-hook events. Each plugin adds
noun events on top.

| Claude Code | bingo before Phase 7 | Phase 7 | M |
|---|---|---|---|
| `tool.call` | tool.run point | `tool.call` | 131 (renamed) |
| `tool.check` | tool.gate point; policy then person | `tool.check`; the policy link stays fixed | 131 |
| `tool.describe` | — | chain over each round's tool specs | 136 |
| `prompt.submit` | submit point | `prompt.submit` | 131 |
| `prompt.compose`, `.section`, `.context` | context.compose point; contributors | `prompt.compose`; one contributor piece is a section | 131, 136 |
| `prompt.attachment`, `.mention` | surfaces resolve `@` | **gap**: a hook sees mentions in `prompt.submit`'s input | — |
| `prompt.autocomplete` | argument completion (M100) | chain over completion | 136 |
| `prompt.fill`, `.suggest` | — | `Slot::Prompt { fill, suggest }`, pushed | 140 |
| `prompt.edit` (each keystroke) | — | **gap**: the composer belongs to the surface | — |
| `turn.start` | `Turn` notification | observation | 136 |
| `turn.step` | model.request point | `turn.step` | 131 |
| `turn.complete` | `Stop` hook | `turn.stop` (may ask for another round) plus `turn.complete` (text drawn under the answer) | 131, 136 |
| `session.start`, `.end`, `.attach`, `.detach` | `Session` notification | observations | 136 |
| `session.compact` | compact point | `session.compact` | 131 |
| `session.receive`, `.send` | deliver | chains on deliver, in and out | 136 |
| `session.append` | extend | chain; only a plugin's own rows are rewritable | 136 |
| `session.measure` | the ruler | observation | 136 |
| `command.run`, `.describe` | commands | chains | 136 |
| `config.set`, `.describe` | settings writes | chains | 136 |
| `plugin.register` | `standing()` | chain judging tier and `uses` | 137 |
| `engine.create` | — | no equivalent: a noun comes from a declared service | 136 |
| `agent.spawn`, `.offer`, `skill.prompt` | the agents and skills plugins | noun events from those plugins' services | 136 |
| `ui.*` (9 events) | — | ADR-0079 | 140, 141 |
| op events (62) | — | one chain per `$` method, raised by the runtime | 138, 139 |
| noun events | services (ADR-0069) | `<noun>.<method>` over a service's wire face | 136 |
| `classic.*` (33) | hooks-shell, run through the `Hook` adapter | **gap**: shell hooks are not mod events | — |
| `telemetry.*`, `attribution.text` | — | not applicable: bingo has neither | — |

The kernel owns no feature nouns. `agent`, `skill`, `mcp` and `rooms`
reach a mod as noun events from the plugin that owns them, never as
kernel events.

## 3. `$`

| Noun | Claude Code methods | bingo source | M |
|---|---|---|---|
| `session` | messages, cwd, root, model, turns, id, repo, surface(s), usage, version, compact, send, append, authorize | `HostApi` sessions, history, deliver, extend | 138 |
| `turn` | abort | interrupt | 138 |
| `prompt` | submit, read, fill, suggest, compose | submit, `Slot::Prompt`, compose | 138 |
| `tool` | list, call, check, register | `HostApi::invoke`, a mod's `ToolSource` | 138 |
| `command` | list, run, register | the actor's commands, a mod's command source | 138 |
| `config`, `settings` | list, set, read | the settings view and layers | 138 |
| `model` | complete, classify, fork | `HostApi::complete` (ADR-0075) | 138 |
| `state` | get, set (versioned) | `host/state` (ADR-0075 §2), session scope | 138 |
| `store` | get, set, delete, keys | `host/state`, global scope | 138 |
| `clock` | now, sleep, after, every | tokio time, paused in tests (M111) | 138 |
| `plugin` | name, root | the mod's manifest | 138 |
| `ui` | status, toast, open, close, panes, invalidate, resolve, notice, log, ask, notify | ADR-0079; ask is `ToolHost::ask` | 140, 141 |
| `ui` | scroll, focus, copy, selection | **gap**: these are the surface's own | — |
| `fs` | read, write, list, exists, stat, ancestors (4 MiB) | scoped to the project and the mod's data | 139 |
| `http` | fetch | `bingo-http` (M127) | 139 |
| `process` | run, spawn | `bingo-process` (M128), behind a grant | 139 |
| `env` | get, set | behind a grant; names are literals | 139 |
| `mcp` | call, connect | the MCP plugin's noun | 139 |
| `agent` | spawn, list, register | the agents plugin's noun | 139 |
| `audio` | play, speak | **gap** | — |
| `telemetry` | log, mark | not applicable | — |

## 4. Drawing

- **Sites** (9368): AskUserQuestion, UserMessage, AssistantMessage,
  ToolUse, ToolResult, ToolGroup, ToolProgress, CommandOutput, Spinner,
  TurnDuration, InfoNotice, SessionMode, PromptHint, AbovePrompt and
  Pane. The permission dialog is the engine's alone.
- **Element tables** (3809).
  - terminal: Box, Text, Button, Input, Select, Link, Code, Markdown,
    Client, Raster, Image.
  - desktop: the same, with Svg in place of Raster and Image.
  - vscode: desktop's, without Client.
  - mobile: a subset.
- **`next(e)`** resolves to an opaque engine node. A hook may wrap it in
  a Box, never in a Text.
- **When a drawing is asked again.** On a change of props or width, or
  on `invalidate`, which is limited to 10 a second (30 on the terminal).
  A change of height alone redraws nothing.
- **Panes** (2506).
  - `open` resolves to `{isPlaced}`.
  - A pane opened unasked needs 144 columns, or 110 for an id the person
    has opened before.
  - A pane docks in fullscreen and otherwise sits inline.
- **Toasts** last 4 s. The stack holds 50, and toasts wait behind a
  `holdToasts` pane (bingo: `Slot::Pane.hold_toasts`).

In bingo, ADR-0079 maps all of this, under Claude Code's names, except
`Client` modules, `blit` and `open`'s `isPlaced` (§8).

## 5. State

- **`$.state`** holds versioned values per session. They survive a hot
  reload.
  - A `get` made while drawing subscribes that drawing.
  - A `set` made while drawing is refused.
  - Only the owning plugin writes a value; another plugin changes it by
    rewriting `state.set`.
- **`$.store`** holds JSON per plugin across sessions, 4 MiB in all.
- **Helpers** (14705): `atom`, `derive`, `read`, `update` with an
  `ifVersion` retry, and `memberOf`. They are pure TypeScript over
  `$.state`.
- **The contract.** A mod's `types/index.d.ts` declares
  `interface PluginState { '<plugin>': {...} }`, and `validate` holds
  every literal key the module uses to it.

In bingo, `host/state` with versions arrives in M132. The read
tracking, the refusal while drawing and the helpers arrive in M138 and
M142.

## 6. Lifecycle and isolation

- **Hot reload.**
  - Saving a file reloads the module in a fresh environment, when the
    turn ends.
  - `register`, `engine.create` and `session.start` run again.
  - `$.state`, `$.store` and open panes stay.
  - Module variables, timers and `derive` caches go.
- **The sandbox.**
  - There is no DOM, no Node and no `require`.
  - `import()` stops the module from loading.
  - `eval` and `new Function` throw.
  - There is no WebAssembly and there are no timers (`$.clock` replaces
    them).
  - The globals are `h`, `Fragment`, `URL`, `TextEncoder`/`TextDecoder`,
    `AbortController`, `crypto.subtle`/`randomUUID` and
    `performance.now`.
- **`userConfig`.** Options are typed and validated before load. A
  secret goes to secure storage. Every non-secret field is a `/config`
  row, and changing one reloads the module.
- **Validate** reads the source as the engine will. It lists what the
  module hooks and calls, and refuses a non-literal `on`, `$`, `$.env`
  or `$.state` name.
- **Test** runs `*.test.ts` against the engine's own `$`. It provides
  `mock.clock`, `mock.store`, `mock.env` and `mock.session`, and
  `$.ui.mount` for driving a drawing.

## 7. Where bingo differs on purpose

- **One runtime, in the host.** Mods run where the session runs. A
  surface draws trees and fires handlers; it never runs mod code
  (ADR-0079).
- **Trust is a grant, not a tier of order.** ADR-0074's `tighten`,
  `rewrite` and `full` stay, alongside the order tiers.
  - A project-layer mod is `tighten` until the user layer grants it
    more.
  - `uses` beyond the safe set (process, http, fs writes, env, model)
    needs the same grant (ADR-0078 §5).
- **Two transports, one module.** The same `register` runs in process
  (QuickJS) or out of process over protocol 2 under node, through the
  TypeScript SDK (M133).
- **No feature nouns in the kernel.** Agents, skills, MCP and rooms are
  nouns their plugins declare.

## 8. Gaps left after Phase 7

- `Client` modules and `blit`.
- `prompt.edit` on each keystroke.
- `audio`.
- `classic.*`.
- The vscode and mobile surfaces.
- `$.ui.scroll`, `focus`, `selection`, `copy`, and `open`'s `isPlaced`:
  a surface's scroll, focus, selection, clipboard and placement are its
  own (ADR-0013 §4), and the host has no verb toward a surface.
- `prompt.attachment` and `prompt.mention`: surfaces resolve `@`; a hook
  sees the result in `prompt.submit`'s input.
- `$.tool.check` without a run, `$.prompt.compose` as a read, and
  `$.session.repo`: each would need a kernel door of its own (M138).

Each is a non-goal written into the milestone that would have owned it,
with the reason.
