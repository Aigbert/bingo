# Plugin switches

`/plugins` (alias `/modules`) lists each plugin's version, state, reason and
source. `/plugins enable <name>` and `/plugins disable <name>` save
`enabledPlugins` in user settings and say when the change takes effect.

A plugin contributing only commands, catalogue entries or non-request-shaping
interceptors switches now. Its drawing falls back to the surface's drawing;
its commands and their default key bindings disappear. Enable restores them.
A running turn keeps the chain it started with, including its completion hooks.
A command already dispatched finishes with the command it resolved.

A plugin contributing tools, providers, stores, surfaces, services, context
contributors or a `prompt.compose` interceptor waits for the next start.
The response names the first reason, for example `off at next start (tools)`.
Legacy hooks, permission policies, compactors, native dependency requirements
and unknown late contributions also require a restart. Process plugins always
require a restart. Stores and surfaces needed by this build refuse disabling.

The listing derives `pending` from the saved wish and what is running.
Its reason says `off at next start` or `on at next start`; reversing a pending
switch clears the difference. `registering` means admission is still running.
Neither state grants repository approval or overrides a failed admission.

JavaScript modules switch through their reload lifecycle. Disabling removes
future dispatch and retires the old generation after its in-flight work ends.
Enabling loads a new generation with admission and current repository approval.
`$.state` and `$.store` survive; session-start hooks run again. A module declaring
`tool` authority is conservatively held until next start because it can register
tools after loading. An unloaded module's `prompt.compose` hooks are checked
before it can join the running chain.

`/plugins enable plugins:<name>` can save the wish for a waiting repository mod,
but it still waits for `/plugins approve <name>`. Edited bytes require a fresh
yes. A personal namesake cannot supply that yes.

The headless `bingo plugins enable|disable <name>` command only writes the wish
for the next start. `/mcp enable|disable` retains its own server lifecycle.
