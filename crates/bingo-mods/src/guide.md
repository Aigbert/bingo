# Mods

A mod's `plugin.json` names a JavaScript or TypeScript `module`, run in
sandboxed QuickJS. It can change dispatch, add tools or commands, and draw the
interface. A skill supplies instructions (`guide-skills`), a shell hook runs a
configured command (`guide-hooks`), and an MCP server supplies tools
(`guide-mcp`). An external-process plugin runs its own executable and speaks the
plugin protocol; it is the other code-authoring path, described below. Keys, `onKey` and wrapping a built-in feature are on `guide-mods-keys`.

## Files and loading

The user directory follows bingo's config directory: `~/.bingo/plugins/<name>/`; repository plugins live in `<cwd>/.bingo/plugins/<name>/`.
`--plugin-dir <dir>` is repeatable. Approved project plugins override user namesakes; explicit directories win in argument order.
A minimal `plugin.json`, in a directory named `counter`:

```json
{"name":"counter","version":"1","module":"register.ts","uses":["state","ui"]}
```

`name` must match the directory; `version` is required. `module` is the entry
file relative to that directory, mutually exclusive with `entry`. `uses` names
the host nouns or service keys the mod needs. `config` describes settings under
`plugins.<name>`; those settings reach `register` as `options`. A `types` field
is not read by this build: the validator looks at `types/index.d.ts` directly.
Local imports must stay inside the mod root; `'bingo'` is supplied by the host.
TypeScript and TSX are compiled on load. There is no Node, DOM or npm loader;
use `$` for host operations and clocks.

## Hooks

Export `register`, called once per runtime: `export const register = (on, options) => …`.
`on(event, matcher?, hook)` takes an exact event, `noun.*`, `*` or `!pattern`.
An object matcher matches a subset of the event; arrays match alternatives and
regular expressions match strings. A hook receives `($, e, next)`: `e` is frozen,
and `next` runs the remaining links, ending in bingo's own action, at most once.

```ts
import type { Register } from 'bingo';
export const register: Register = (on, options) => {
  on('tool.call', {tool: 'Bash'}, async ($, e, next) => {
    if (e.command === 'echo mod') // Answer without calling the tool.
      return {parts: [{type: 'text', text: 'from the mod'}], isError: false};
    const result = await next({...e, command: e.command.trim()}); // Rewrite.
    await $.ui.status(result.isError ? 'Shell failed' : null); // Afterward.
    return result;
  });
};
```

Tool inputs are beside `tool` and `tool_use_id`, not nested under `input`.
Return the event's result shape to answer without `next`; an undefined return
falls through or keeps the result of an already-called `next`. Rewriting uses a
copy such as `next({...e, command: 'echo changed'})`. Pinned fields and trust still
apply. `next.called`, `next.signal`, `next.origin` and `next.budget` describe the
call. `on(...).catch(($, e, next) => …)` can recover using `next.error`.

## Events

`schema/events.json` gives each input, result, matcher, pinned field and allowed
act. These are the built-in events; service faces and `$` operations also expose
`noun.method` events. Observation events do not consume a returned answer.

- UI: `ui.render` draws a component; `ui.press`, `ui.input`, `ui.select` dispatch its handlers.
- Prompt: `prompt.submit` admits input; `prompt.compose` builds model context; `prompt.autocomplete` changes completions.
- Tool: `tool.check` judges permission; `tool.call` runs it; `tool.describe` describes it.
- Model: `model.complete` intercepts a host-requested completion.
- Turn: `turn.start` observes the start; `turn.step` calls the turn's model; `turn.stop` decides whether to stop; `turn.complete` composes completion text.
- Session: `session.start`, `session.end`, `session.attach`, `session.detach`, `session.measure` are observations of lifetime, clients and usage.
- Session dispatch: `session.send`, `session.receive`, `session.append`, `session.compact` handle delivery, records and compaction.
- Command: `command.run` runs one; `command.describe` describes it.
- Config: `config.set` changes a setting; `config.describe` describes the settings.
- Plugin: `plugin.register` adjudicates a plugin's tier and requested nouns.

## The `$` nouns

Methods are asynchronous; `schema/dollar.json` and generated types specify their
arguments. Calls need their noun in the grant. A service can add another noun.

- `$.plugin` — read-only `name`, `root` and `version` data about this plugin.
- `$.session` — messages, cwd/root, model, turns, id, surfaces, usage/version, compaction, delivery, append and authorization.
- `$.turn` — abort the turn.
- `$.prompt` — read the prompt or submit input.
- `$.tool` — list, call or register tools; calls still pass the permission gate.
- `$.command` — list, run or register commands.
- `$.config` — list, read or set settings.
- `$.model` — complete, classify or fork a model request using the host.
- `$.state` — versioned state belonging to this plugin in the current session.
- `$.store` — versioned global plugin state, including keys and delete.
- `$.clock` — now, sleep, after and every; callbacks replace JavaScript timers.
- `$.fs` — read, write, list, exists, stat and ancestors within granted roots.
- `$.http` — fetch through the host.
- `$.env` — get or set the mod's environment overlay.
- `$.process` — run or spawn a process, then read, write or kill its handle.
- `$.agent` — list, register or spawn agents through the agent service.
- `$.mcp` — call tools or connect through the MCP service.
- `$.ui` — draw, open panes, place status, notify and handle interaction.

## Drawing and state

Match `ui.render` to replace a component or wrap `await next(e)`. The event carries `component`, `requestId`, `surface`, `props` and `viewport`. `await $.ui.resolve(e)` provides
supported element names (`Box`, `Text`, `Button`); text surfaces use folds. Handlers such as `onPress` may write state; render hooks may not.

Components: `Pane`, `AbovePrompt`, `Question`, `UserMessage`, `AssistantMessage`, `ToolUse`, `ToolResult`, `ToolGroup`, `ToolProgress`, `CommandOutput`, `Spinner`, `TurnDuration`,
`InfoNotice`, `SessionMode`, `PromptHint`. Fact-only sites: `Reasoning`, `Compaction`, `Rewind`, `Interruption`, `QuestionAnswer`, `PermissionReceipt`, `Asset`, `ShellRun`, pending
`Action`, `TurnProgress`, `SessionStatus`, `Toast`, `Welcome`. All list `terminal`, `desktop`, `text` in `Component::surfaces()`; validate warns about surfaces that never ask for a
site. `chat` also asks `AssistantMessage`, `CommandOutput`, `InfoNotice`, `ShellRun` with a zero-cell viewport, once prose finishes streaming.
Chat draws Box, Text, Code, Link, Image, Button, Select, Input and Engine; other elements fold. The `text` table remains folds only.

Only `Permission` and `SignIn` are sealed, also on all three surfaces. Wrap `await next(e)` exactly once with unchanged props. Missing, duplicated or rewritten engine nodes yield
the host drawing and a plugin-named notice. Permission props are `interactionId`, `tool`, full `input` and `answers`; sign-in props are `interactionId`, `provider` and `flow`
(browser URL, device URL/code, or paste). The terminal rejects hidden, zero-size or clipped cards with one local notice. Leave room for full input and choices. Wrapper buttons
handle clicks; answer keys remain the host's. Text clients keep their folds. `plugins validate` advises on sealed matchers; runtime checks decide. See the `sealed-frame` test
fixture.

`Spinner.word` is the animated verb; `message` overrides it or is `null`; `suffix` is `…`. This fixes older bingo builds' interrupt-hint/token misuse of those props. `TurnProgress`
carries `elapsedMs`, `tokensOut`, `interruptible`, `stopping`; surfaces format them. The terminal asks once per elapsed second or changed fact.

`await $.ui.open({id:'counter', title:'Counter'})` publishes a pulled pane. Match `{component:'Pane', requestId:'counter'}` to render it; `$.ui.close(id)` removes it. The pulling
client supplies `surface`. `place`, `size`, `focus`, `closeOnEscape` control placement and keys. Autonomous opens wait as `1 pane waiting` until a person opens them. Clicks and
commands remember open; Escape and close remember closed. Preferences persist by plugin and pane id across projects and starts; autonomous moves never change them, and missing or
corrupt memory means waiting. `$.ui.status('Ready', {key:'counter'})` places status; `null` removes it. `$.ui.toast('Finished')` shows a notification. `AbovePrompt` draws above the
prompt.

Import `atom`, `read`, `update`, optionally `derive`, from `'bingo'`: `const calls = atom('calls', 0)`, `await read($, calls)`, `await update($, calls, n => n + 1)`. Reads
subscribe renders to changes; writes invalidate them. State survives reload/restart. Declare literal keys in `types/index.d.ts`, keyed by manifest name:

```ts
export interface PluginState { 'counter': { calls: number } }
```

## Types and checks

Loading a personal mod writes host-derived `bingo`, `bingo/testing` and `bingo-tools` types under `.bingo/types/`, plus `.bingo/tsconfig.json` and an absent root `tsconfig.json`. Repository startup leaves approved bytes unchanged.
Run `tsc --noEmit -p <dir>/tsconfig.json` separately; the runtime strips types without checking them.

- `bingo plugins validate <dir>` statically checks the module and reports events,
  calls, keys and uses. Keep event names, `$` member accesses, state keys and env
  names literal. Findings include missing grants and state-contract problems.
- `bingo plugins test <dir>` runs `*.test.ts` and `*.test.tsx` with an isolated real
  host and fake provider. Import `test`, `expect`, `mock`, `reload` from
  `'bingo/testing'`; this import is available only in the test runtime.

In `test('draws', async $ => { … })`, use `mock.session({model:{responses:[…]}})`
to script model output and `mock.session.submit('…')` to run a turn.
`await reload()` checks persistence. `await $.ui.mount('AbovePrompt', props)`
or `await $.ui.mount({pane:'counter'})` returns a mount with `drawn`, `tree`,
`find`, `press`, `input`, `select`, `advance` and `resize` for assertions and input.
Tests keep ordinary permission checks and mod grants; declare needed permissions.

## Trust and failures

`tighten` can observe and tighten decisions, with input rewriting at the
permission/submission sites; it cannot grant permission. `rewrite` also permits
rewriting and answering where the event contract allows, but cannot weaken a
permission decision. `full` can also allow at `tool.check`.
User-directory and `--plugin-dir` mods default to `rewrite` with their declared
`uses`. Project mods default to `tighten`, with unsafe nouns removed. External
processes default to `tighten`. `pluginTrust` grants are honoured only in the
user settings layer (`~/.bingo/settings.toml`), never a project or flag layer.
For example, `[pluginTrust.counter]` with `tier = "rewrite"` and
`uses = ["state", "ui"]` grants that named plugin its tier and nouns.
`process`, `http`, `env`, `model`, writes outside the mod's data directory and
unsafe service methods require the corresponding authority.

User-directory and `--plugin-dir` mods are watched, including additions and removals.
Changes load while idle, at `turn.complete`, or before a mod's tool or command runs.
Project mods require `/plugins reload`, which rescans all three directories and
answers with counts and added names, or `nothing changed`. Broken mods report
errors without taking others down. Each replacement runs `register` and
`session.start` again, retaining state, store and panes under the same trust checks.
`PLUGIN_STALE`, `PLUGIN_RELOADED`, `PLUGIN_UNLOADED` and `PLUGIN_RELOAD` report changes.
Load failures appear as `PLUGIN_UNREADABLE` notices and a disabled plugin reason
in `/plugins`; malformed names have `PLUGIN_MISNAMED`. A hook that throws,
exceeds its budget, returns an invalid shape or oversteps its tier is skipped
with an `INTERCEPTOR_SKIPPED` notice naming the plugin, event and reason. The
remaining chain runs, or its previous `next` result stands. Notices reach the
session event stream and surfaces; `--print` reports diagnostics on stderr.
`$.ui.log` writes to the application tracing subscriber under `mod::<name>`.

## External-process plugins and worked examples

An external-process plugin uses `entry: {command, args, env}` instead of
`module`, runs separately and speaks protocol 2 over stdin/stdout. See
`schema/plugin.json` for the plugin wire contract and `schema/plugin-2.json`
for its baseline. `schema/rpc.json` describes the separate `bingo serve --stdio`
session-client API, not the plugin handshake. Process plugins default to
`tighten`; extra rewrite/full authority comes from user-layer `pluginTrust`.
Start from the repository's `examples/plugins/` and read the adjacent tests:
- `examples/plugins/tool-call` — deny edits, rewrite shell input, count calls and wrap `ToolUse`.
- `examples/plugins/band` — `AbovePrompt`, a button and state across reload.
- `examples/plugins/pane` — a tool ledger, commands, pane rendering, resize and Escape.
- `examples/plugins/ts-status` — persistent call count, status and a permission hook.
- `examples/plugins/wordcount` — the Python external-process example, with `plugin.json` and `main.py`.

## Repository approval
`/plugins approve <name>` approves the current directory bytes and loads modules now; process plugins start next time. CI uses `bingo plugins approve <name>` during setup.
For `/plugins enable|disable`, immediate switches and restart reasons, read `guide-mods-switches`.
A changed directory waits again. A waiting project plugin leaves the user namesake running. Approval names the canonical checkout path and lives only in user settings; `pluginTrust` separately limits authority.
Fingerprints cover paths and bytes, capped at 8 MiB or 2,000 entries. Escaping/looping symlinks and special files cannot be approved. Repository `pluginDirs` is ignored; personal settings and explicit `--plugin-dir` still work. Project shell hooks are outside this mechanism.
