# bingo TypeScript author SDK

Private, dependency-free source for protocol 2. `src/wire.ts` is generated
from the Rust plugin schema; regenerate with
`BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc wire_ts::`.
The renderer is the general `bingo-schema-ts` library, also intended for the
M136 event catalogue. No enums, namespaces, or parameter properties are emitted.

Export `register(on, options)` from an author module. A process entry imports
`plugin` from `src/index.ts` and calls `await plugin(new URL('./register.ts',
import.meta.url))`. Registration must finish before hooks run. `options` is the
manifest's resolved plugin configuration. Import author types with `import type`.
The example uses relative type imports, erased at runtime, so its register module
contains no Node API and can also be loaded by M137.

`on('tool.call', {tool: ['Read', 'Bash']}, async ($, e, next) => …)` matches
partial nested fields; scalar, RegExp and alternative-array matchers work.
Patterns are exact, `ns.*`, `*`, or negated with `!`. Each hook gets frozen
input and a single-use `next(e)`. Its promise resumes on the host's `/after`
reply; `next.signal` is aborted and the promise rejects on cancellation.
Returning without `next` answers directly. Returning nothing after `next`
preserves the downstream result. The host enforces trust tiers and deadlines.

`$.ui.status(text, {key, tone})` places a status slot (`null` removes it).
`$.state.get/set` use session scope; `$.store.get/set` use global scope.
An absent key reads as `null`; present values are `{value, version}`.
`set(key, value, ifVersion?)` performs the protocol's optimistic version check.
`$.model.complete(request)`, `$.session.deliver(input, wake?)`, and
`$.session.extend(key, value)` use the current hook's site. Service faces can
extend `WithServices<Faces>`: `$.<noun>.<method>(params)` routes `service/call`.
Only negotiated capabilities are requested. `runtime(peer, register)` exposes
`toolsChanged(specs)` for authors who provide `tool/call` through the peer.

Run `node --test sdk/typescript/src/*.test.ts` from the repository root.
Node 22.18+ and 23.6+ enabled type stripping by default; CI pins Node 24.17.0.
See [Node's TypeScript support](https://nodejs.org/api/typescript.html).
Deno runs `.ts` directly and provides `node:readline` and `node:process`
([compatibility](https://docs.deno.com/runtime/fundamentals/node/)); use
`deno run main.ts` and grant only permissions your module needs.
Bun also runs TypeScript directly ([documentation](https://bun.sh/docs/typescript));
use `bun main.ts`. Deno/Bun support is based on their documented APIs;
the acceptance fixture is exercised on Node.

`bingo plugins validate <dir>` parses the manifest, checks the structure of its
config schema, spawns the entry, handshakes, lists contributions, and stops it.
`bingo plugins test <dir>` runs sorted `scenarios/*.json` with a fresh temporary
host home per scenario and the real kernel, default permission policy, file and
shell tools, fake provider, and plugin host. This is an author tool that executes
local plugin code. It is not a sandbox. It reads no user settings or credentials.
Scenarios use the generated `schema/plugin-scenario.json`.
`denied` checks error output containing “denied” (the stream has no separate
permission verdict); tool expectations inspect completed outputs, slot expectations inspect placement keys, and notice
expectations inspect what the session received. Each failure names its step.
