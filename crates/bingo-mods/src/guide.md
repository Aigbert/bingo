# Mods

A mod's `plugin.json` names a JavaScript or TypeScript `module`, run in
sandboxed QuickJS. It can change dispatch, add tools or commands, and draw the
interface. A skill supplies instructions (`guide-skills`), a shell hook runs a
configured command (`guide-hooks`), and an MCP server supplies tools
(`guide-mcp`). An external-process plugin runs its own executable and speaks the
plugin protocol (last section). Keys, `onKey` and wrapping a built-in feature are on `guide-mods-keys`.

## Start here

1. Make the mod's directory with a `plugin.json` (next section), then lay the kit: `bingo plugins types <dir>`. It writes under `<dir>/.bingo/`:
   - `types/`: this build's `bingo`, `bingo/testing` and `bingo-tools` types; every event, `$` method and element carries its description as JSDoc.
   - `reference.md`: the same descriptions on one page. Each event's input, result, matcher and allowed acts; each `$` method's arguments; each element's props; each site's props.
   - `examples/<name>/`: four worked mods with their tests, each passing `bingo plugins test`.
2. Look a name up: `grep -n -A12 'tool.call' <dir>/.bingo/reference.md`, or `grep -rn 'export type ToolCall' <dir>/.bingo/types/`.
3. Read the closest example, copy its files (never its `.bingo/`) into your directory, rename it in `plugin.json` and `types/index.d.ts`, and change it.
4. Run `bingo plugins validate <dir>` and `bingo plugins test <dir>` until both pass.

| The person asks for | What it is | Start from |
|---|---|---|
| block a tool, a file or a command | a `tool.check` deny refuses before the person is asked; a `tool.call` `{deny: reason}` runs only once permission is given | `.bingo/examples/ts-status`, `tool-call` |
| change a tool's input before it runs | `tool.call` hook calling `next({...e, command})` | `.bingo/examples/tool-call` |
| a slash command | `$.command.register({name, description}, ($, args) => …)` in a `session.start` hook; `args` is the text after the name | `.bingo/examples/pane` |
| a panel, list or side view | `$.ui.open({id, title})` plus `ui.render` on `{component:'Pane', requestId:id}` | `.bingo/examples/pane` |
| a strip above the prompt, a button | `ui.render` on `AbovePrompt`; `onPress` writes state | `.bingo/examples/band` |
| change how a built-in row looks | `ui.render` on that site (`ToolUse`, …) wrapping `await next(e)` | `.bingo/examples/tool-call` |
| a status line entry or a counter | `$.ui.status(text, {key})`; `atom`/`update` or `$.store` | `.bingo/examples/ts-status` |
| something remembered across reloads | `atom`, `read`, `update` (session) or `$.store` (global) | `.bingo/examples/band` |
| a tool the model can call | `$.tool.register(spec, handler)` in a `session.start` hook | the `$.tool` example in `reference.md` |

## Files and loading

User mods live in `~/.bingo/plugins/<name>/` (bingo's config directory); repository mods in `<cwd>/.bingo/plugins/<name>/`, which need approval (last section).
`--plugin-dir <dir>` is repeatable. Approved project plugins override user namesakes; explicit directories win in argument order.
A minimal `plugin.json`, in a directory named `counter`:

```json
{"name":"counter","version":"1","module":"register.ts","uses":["state","ui"]}
```

`name` must match the directory; `version` is required. `module` is the entry
file relative to that directory, mutually exclusive with `entry`. `uses` names
the host nouns or service keys the mod needs. `config` describes settings under
`plugins.<name>`; those settings reach `register` as `options`.
Local imports must stay inside the mod root; `'bingo'` is supplied by the host.
TypeScript and TSX are compiled on load, without type checking. There is no Node, DOM or npm loader;
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
    const command = String(e.command);
    if (command.startsWith('rm -rf')) return {deny: 'counter: no rm -rf'}; // Refuse.
    if (command === 'echo mod') // Answer without calling the tool.
      return {parts: [{type: 'text', text: 'from the mod'}], isError: false};
    const result = await next({...e, command: command.trim()}); // Rewrite.
    await $.ui.status(result.isError ? 'Shell failed' : null); // Afterward.
    return result;
  });
};
```

Tool inputs are beside `tool` and `tool_use_id`, not nested under `input`.
An undefined return falls through, or keeps the result of an already-called `next`.
Pinned fields and trust still apply. `next.called`, `next.signal`, `next.origin` and `next.budget` describe the
call. `on(...).catch(($, e, next) => …)` can recover using `next.error`.

## Events

`reference.md` gives each event's input, result, matcher, pinned fields and allowed
acts. These are the built-in events; service faces and `$` operations also expose
`noun.method` events. Observation events do not consume a returned answer.

- UI: `ui.render` draws a component; `ui.press`, `ui.input`, `ui.select` dispatch its handlers.
- Prompt: `prompt.submit` admits input; `prompt.compose` builds model context; `prompt.autocomplete` changes completions.
- Tool: `tool.check` judges permission; `tool.call` runs it; `tool.describe` describes it.
- Model: `model.complete` intercepts a host-requested completion.
- Turn: `turn.start` observes the start; `turn.step` calls the turn's model; `turn.stop` decides whether to stop; `turn.complete` composes completion text.
- Session: `session.start`, `session.end`, `session.attach`, `session.detach`, `session.measure` observe lifetime, clients and usage; the context reading at any moment is `context` in `await $.session.usage()`.
- Session dispatch: `session.send`, `session.receive`, `session.append`, `session.compact` handle delivery, records and compaction.
- Command: `command.run` runs one; `command.describe` describes it.
- Config: `config.set` changes a setting; `config.describe` describes the settings.
- Plugin: `plugin.register` adjudicates a plugin's tier and requested nouns.

## The `$` nouns

Methods are asynchronous; `reference.md` lists each one's arguments with an example per noun.
A call needs its noun in `uses`. A service can add another noun.
`$.plugin` (this mod's `name`, `root`, `version`), `$.session`, `$.turn`, `$.prompt`,
`$.tool` (calls still pass the permission gate), `$.command`, `$.config`, `$.model`,
`$.state` (this mod's state in the session), `$.store` (its global state), `$.clock`
(callbacks replace JavaScript timers), `$.fs` (within granted roots), `$.http`, `$.env`,
`$.process`, `$.agent` and `$.mcp` (through their services), and `$.ui`.

## Drawing and state

Match `ui.render` to replace a site or wrap `await next(e)`, bingo's own drawing. The event carries `component`, `requestId`, `surface`, `props` and `viewport`;
`reference.md` lists every site with its props. `const {Box, Text, Button} = await $.ui.resolve(e)` gives the element kinds the asking surface draws;
a view is `{kind: Box, children: [{kind: Text, children: ['hi']}]}`. Text surfaces get plain-text folds. Handlers such as `onPress` may write state; render hooks may not.
`Permission` and `SignIn` are sealed: wrap `await next(e)` exactly once with unchanged props, or bingo draws its own with a notice naming the mod.

`await $.ui.open({id:'counter', title:'Counter'})` publishes a pulled pane; match `{component:'Pane', requestId:'counter'}` to draw it; `$.ui.close(id)` removes it.
`place` lists where it may sit, in order (`'above'`, the default, `'side'`, `'overlay'` or `{rect}`); `size` is `{rows, columns}`, each `{min, ideal, max}` in cells or `'40%'`;
`focus` and `closeOnEscape` control keys. An open from a hook waits as `1 pane waiting` until a person opens it; a command's open shows at once.
`$.ui.status('Ready', {key:'counter'})` places status; `null` removes it. `$.ui.toast('Finished')` shows a notification.

Import `atom`, `read`, `update`, optionally `derive`, from `'bingo'`: `const calls = atom('calls', 0)`, `await read($, calls)`, `await update($, calls, n => n + 1)`.
Reads subscribe renders to changes; writes invalidate them. State survives reload and restart. Declare its keys in `types/index.d.ts`, keyed by manifest name:

```ts
export interface PluginState { 'counter': { calls: number } }
```

A mod without state declares `{ 'name': {} }`; validate reports a missing contract otherwise.

## Types and checks

`bingo plugins types <dir>` also writes `.bingo/tsconfig.json` and, when absent, a root `tsconfig.json`, and prints each file it changed;
`plugins test` and `plugins validate` lay the kit first too. A run never writes into a plugin's directory. A service another plugin serves is
declared only when its process starts, so the types leave it out; call it through `$` untyped. Type-check with `tsc --noEmit -p <dir>/tsconfig.json`.

- `bingo plugins validate <dir>` statically checks the module and reports events,
  calls, keys and uses. Keep event names, `$` member accesses, state keys and env
  names literal. Findings include missing grants and state-contract problems.
- `bingo plugins test <dir>` runs `*.test.ts` and `*.test.tsx` (outside `.bingo/`) with an isolated real
  host and fake provider. Import `test`, `expect`, `mock`, `reload` from
  `'bingo/testing'`; this import is available only in the test runtime.

In `test('draws', async $ => { … })`, `mock.session({model:{responses:[{steps:[{toolCall:{name:'Bash',input:{command:'ls'}}}]},{steps:[{text:'Done.'}]}]}})`
scripts the model, one response per model call; `mock.session.submit('…')` runs a turn; `mock.session.snapshot()` returns the session, history included; toasts and status entries are under its `signals`,
so `expect(JSON.stringify(await mock.session.snapshot())).toContain('Hello')` checks one.
`await $.command.run({name:'counter', args:''})` runs a command as if typed. `await reload()` checks persistence. `await $.ui.mount('AbovePrompt', props)`
or `await $.ui.mount({pane:'counter'})` returns a mount with `drawn`, `tree`, `find`, `press`, `input`, `select`, `advance` and `resize`.
Tests keep ordinary permission checks and mod grants: pass `permissions:{allow:['Bash(echo:*)']}` to `mock.session` for calls that would ask.

## Trust and failures

`tighten` can observe and tighten decisions, with input rewriting at the
permission/submission sites; it cannot grant permission. `rewrite` also permits
rewriting and answering where the event contract allows, but cannot weaken a
permission decision. `full` can also allow at `tool.check`.
User-directory and `--plugin-dir` mods default to `rewrite` with their declared
`uses`. Project mods default to `tighten`, with unsafe nouns removed. External
processes default to `tighten`. `pluginTrust` grants are honoured only in the
user settings layer (`~/.bingo/settings.toml`): `[pluginTrust.counter]` with `tier = "rewrite"` and
`uses = ["state", "ui"]`. `process`, `http`, `env`, `model`, writes outside the mod's data directory and
unsafe methods (marked in `reference.md`) require that authority.

User-directory and `--plugin-dir` mods are watched and load while idle, at `turn.complete`, or before a mod's tool or command runs.
Project mods require `/plugins reload`. Each reload runs `register` and `session.start` again, keeping state, store and panes.
A broken mod reports `PLUGIN_UNREADABLE` without taking others down. A hook that throws, exceeds its budget, returns an invalid shape or
oversteps its tier is skipped with an `INTERCEPTOR_SKIPPED` notice naming the plugin, event and reason; the rest of the chain runs.
`--print` reports these on stderr. `$.ui.log` writes to bingo's log under `mod::<name>`.

## External-process plugins and approval

An external-process plugin uses `entry: {command, args, env}` instead of
`module`, runs separately and speaks protocol 2 over stdin/stdout; `schema/plugin.json`
in bingo's repository is that contract, and its `examples/plugins/wordcount` is a Python one.
Process plugins default to `tighten`; more authority comes from user-layer `pluginTrust`.

`/plugins approve <name>` approves a repository mod's current bytes and loads it now; CI uses `bingo plugins approve <name>` during setup.
A changed directory waits again, leaving the user namesake running. Approval lives only in user settings. For `/plugins enable|disable`, read `guide-mods-switches`.
