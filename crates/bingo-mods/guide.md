# Module plugins

A plugin directory has one `plugin.json`. Use exactly one launch form:

```json
{"name":"status","version":"1","module":"hooks/register.ts","uses":["store","ui"]}
```

The directory name must match `name`. Export `register(on, options)` from the
module, and import SDK types from `bingo`. The same registration function can
run under Node through the TypeScript SDK with a separate `entry` manifest.
`examples/plugins/ts-status` and its plugin-kit tests exercise both transports.

The runtime reads user plugins, project `.bingo/plugins`, then explicit
`--plugin-dir` directories; approved project plugins override user namesakes. A repository plugin waits for `/plugins approve <name>` (or `bingo plugins approve <name>` in CI); changed bytes wait again, and a waiting plugin shadows nothing. Repository `pluginDirs` is ignored. Disabling
`bingo.mods` refuses module plugins. Listings name `native`, `process` or
`module`; the live plugin catalogue carries the granted tier, nouns and refusal.

Each module has its own QuickJS-NG runtime and thread. Static imports stay inside
its directory, including after resolving symlinks. TS and TSX are compiled with
oxc; JSX produces `h`/`Fragment` JSON nodes. There is no Node, DOM, WebAssembly,
`require`, dynamic import, `eval`, `Function` or timer API. UTF-8 encoders, URL,
AbortController, UUID, SHA-256 and performance.now are available.

Defaults are 64 MiB memory, 512 KiB stack and 10 seconds of active time per hook.
Configure `mods.memoryMiB` and `mods.hookMs`. Time awaiting `next` or `$` is paused;
each host operation has a five-second wall bound. A catch gets a fresh second.
A failure keeps the last continuation result or runs the remaining chain, with
one named notice. Re-entry omits only the calling hook. Interrupt drops the
chain and signals cancellation to the module thread.

User and explicit-directory modules receive rewrite authority and declared
`uses`; project modules receive tighten authority and safe nouns. Only user
`pluginTrust` can grant more. Process plugins retain their `tighten` default
for every directory; declared safe nouns need no tier lift. Registration can reduce authority or deny a
plugin with `{reason: "..."}`. Calls consult the granted catalogue; declaring
a noun does not bypass an unsafe method's authority check.

`$` exposes the host's `session`, `turn`, `prompt`, `tool`, `command`,
`config`, `model`, `state`, `store` and `clock` nouns. The generated
`sdk/typescript/src/dollar-types.ts` lists every method's input and result.
Each method raises its named chain once; other hooks can rewrite, deny or
answer it. `$.plugin` is the frozen `{name, root}` identity and raises no chain.
Calls default to the hook's session and turn where applicable. Reads fold the
same ordered frames as other clients, including configuration, usage, prompt
queue and attached surfaces. `session.messages` pages the persisted history.
`session.extend` and `session.deliver` remain compatibility aliases for
`session.append` and `session.send`.

`$.state.get(key)` / `set(key, value, ifVersion?)` use the current session;
`$.store` uses global scope and also has `keys()` and `delete(key)`. A read
returns `{version, value}` or null. CAS conflicts report the held version.
Only the owning plugin writes its shelf, with a 4 MiB total quota across scopes.
State survives reload; each hook records the keys and versions it reads.

Register a tool with `$.tool.register(spec, async ($, input) => output)`;
its name is `plugin__<name>__<tool>`. Undeclared tool traits fail closed.
`$.command.register(spec, async ($, args) => outcome)` registers an unprefixed
command. Callbacks receive a fresh dollar for their invocation. Before a
registered tool or command runs, a stale module reloads and repeats
`session.start`, so callbacks resolve in the new generation under both runtimes.
Explicit directories are polled once a second; normal hook reload waits for
`turn.complete`. Reload joins the old thread and adjudicates grants again.

`$.clock.now()` returns a timestamp; `sleep(ms)` waits, `after(ms, callback)`
runs once, and `every(ms, callback)` repeats with a floor set by `mods.everyMs`
(default 1000). Timers belong to their session and module generation and stop
at session end or reload. Timer callbacks have no active turn.

`$.model.complete(request)`, `classify({text, labels})` and `fork({prompt})`
require the model grant and use the session's configured model. They charge
the hook's turn, or session usage from a timer. Classification accepts an exact
label and refuses an unmatched answer in words. Fork builds context from the
persisted history (including compaction), without contributor pieces or tools;
it does not reproduce the parent turn's assembled prompt.

`plugins.<name>` supplies options (an empty object when absent). Manifest config
schemas validate types, object properties/required/additionalProperties, arrays,
choices, local references, composition and size/numeric bounds. Unsupported
validation keywords are refused with a named reason instead of being ignored.
Full author-kit analysis belongs to a later plan.

For a repeatable local memory measurement, run:

```sh
cargo test -p bingo-mods --test engine fifty_idle_modules_rss --locked -- --ignored --nocapture
```

The 2026-10-14 macOS run measured 3,232 KiB before and 29,792 KiB with 50 idle
modules. This is a measurement, not a CI threshold.

World nouns (`fs`, `http`, `process`, `env`) use the same service-call chain
under QuickJS and Node. Declare each noun in `uses`. Project modules need a
user-layer `pluginTrust` grant for unsafe operations; process plugins also
keep their default `tighten` tier until granted. Refusals name the settings
file. The first unsafe noun use per plugin and session posts a notice.

Filesystem reads stay inside the project and plugin data roots. Writes to
`<data_dir>/plugins/<name>/data` are safe; other writes inside the project
need the unsafe `fs` grant. The runtime opens the canonical scoped path.
A concurrent replacement of a directory with a symlink between validation
and open remains a TOCTOU limitation. Files and process streams have a 4 MiB
limit. HTTP responses stop at that limit and return `truncated` and `notice`.

`process.spawn` returns a handle with `id`, `read()`, `write(text | null)`
(null closes stdin), and `kill()`. Reads collect stdout and stderr through
EOF. Handles belong to the creating plugin and session; reload, session end
and shutdown terminate their trees. `process.run` is bounded by the host
operation deadline. Arguments are argv, with no implicit shell. `env.set`
changes only the caller's child-process overlay; `env.get` reads the host.

## Pane places

A pane opens from the person's command, key or press. Otherwise it waits in
status until clicked. `open` resolves `{}`; the next render's `viewport`
says which `place`, `columns` and content `rows` the pane received.

```ts
const size = {rows: {min: 8, ideal: 10, max: 20},
              columns: {min: 48, ideal: 64, max: '80%'}};
await $.ui.open({id: 'notes', title: 'Notes', place: ['above'], size});
await $.ui.open({id: 'notes', title: 'Notes', place: ['side', 'above'], size});
await $.ui.open({id: 'notes', title: 'Notes', place: ['overlay'], size});
await $.ui.open({id: 'notes', title: 'Notes',
  place: [{rect: {anchor: 'bottomRight', width: '40%', height: '30%'}}]});
```

Lengths are cells or percentages of the available transcript area. Rect
anchors are `topLeft` (default), `topRight`, `bottomLeft`, `bottomRight`,
and `center`; `x` and `y` default to zero and offset inward from a corner,
or right/down from the center. Out-of-bounds rects move in before shrinking.
One above and one side pane can dock while leaving the transcript at least
60 columns and 8 rows. Up to three floating panes can cover that area; the
newest draws last. The composer and status remain available.

The first fitting place wins. A pane that cannot fit becomes `<title> ▸`
in status; click it to try an overlay. Click the focused title's `place ▾`
or press `alt+p` to cycle fitting places. Moves last for this TUI process.
A collapsed pane receives no render request. Moving or resizing a seated
pane asks again when its place, content width or content height changes.
Old `rows` options still read as the ideal row count; new code uses `size`.
