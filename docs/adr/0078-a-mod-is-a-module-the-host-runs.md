# ADR-0078 — A mod is a module the host runs

Status: accepted for §1 catalogue/noun faces and §5 face metadata · 2026-10-14 · M136; all other decisions remain proposed.
Plans: M137 (runtime), M138, M139 (`$`), M142 (kit).
Amends: ADR-0074 §1 §3 and Non-goals, ADR-0075 §2 and Non-goals, ADR-0069 §3, ADR-0031 §6

## Context

The person asked on 2026-10-13 that a bingo mod do what a Claude Code
mod does ([design/mods-parity-2026-10.md](../design/mods-parity-2026-10.md)):
one sandboxed module the engine runs against 141 events plus noun events,
a `$` whose every method is itself an event, budgets, fail-open, hot
reload and a test harness. Phase 5 gives bingo seven chain points,
protocol 2 and a node SDK; out of process alone, every render and `$`
call crosses a process. Figma runs plugins in QuickJS for this.

The audit refused an embedded engine because V8 breaks the budgets.
Measured on 2026-10-13 (release build, stripped, macOS arm64):

| Engine | Binary | Crates | Memory cap | Time interrupt |
|---|---|---|---|---|
| rquickjs 0.14 (QuickJS-NG) | +1.0 MB | +23 | yes | yes |
| boa 0.22 | +10 MB | +149 | no | no |
| deno_core / V8 | +44.6 MB | +195 | — | — |

**The ratchet.** No kernel door opens beyond ADR-0074's chains and
ADR-0079's render pair; the runtime is a plugin, `$` maps onto doors.

## Decision

1. **The event catalogue is the contract.** `bingo_sdk::catalogue` names
   every event `<noun>.<verb>` with its input, result, matcher fields,
   rewritable and pinned fields, and chain or observation. It generates
   `schema/events.json` and TypeScript; a test fails an event the kernel
   raises that it does not name. Names are one namespace, spelled as
   Claude Code spells them (ADR-0079 §1): `tool.call`'s `e` carries the
   tool's input at the top level beside `tool` and `tool_use_id`, its
   result `isError`; `turn.complete` is a chain. Three kinds:
   - **Kernel events**: tool, prompt, turn, session, command, config,
     ui, plugin. ADR-0074's seven points come first.
   - **Noun events**: a service declares its face (`WireService::face()`,
     each method's input and result schema) and becomes `$.<noun>` and
     `<noun>.<method>`. Agents, skills, MCP and rooms reach a mod this
     way; the kernel names none of them.
   - **Op events**: every `$` method is raised as `<noun>.<method>`
     before the host performs it, and may be rewritten, denied or
     answered. The runtime raises a `$` method that no existing door
     raises as a method of a face it registers, through the service-wire
     door; a test fails a method raised twice.
2. **A mod is a plugin whose manifest names a module.**
   `"module": "<path>.ts"` stands where a process plugin has `command`.
   Manifest, discovery (user, project, `--plugin-dir`) and the state
   store move into the library crate `bingo-plugin-host`, which both
   runtimes read. The module imports `'bingo'` and exports
   `register(on, options)`; `on(pattern, matcher?, hook)` takes a name,
   `ns.*`, `*` or `!pattern`, a matcher that is a partial of `e`, and a
   hook `($, e, next)`, `e` frozen and `next` run at most once (`called`,
   `signal`, `origin`, `budget`). **One module, two transports, the same
   results:** in process, `bingo-mods` gives each mod its own QuickJS-NG
   `Runtime` (`rquickjs`, pinned exactly) on its own thread, and `oxc`
   strips `.ts`/`.tsx` with JSX against `h`/`Fragment`, static imports
   limited to the mod's own files and `bingo`; out of process, protocol
   2 through the TypeScript SDK under node (M133).
3. **The sandbox.** No DOM, Node, `require`, `import()`, `eval`,
   `Function`, WebAssembly or timers. Globals: `h`, `Fragment`, `URL`,
   `TextEncoder`/`TextDecoder`, `AbortController`, `crypto.randomUUID`,
   `crypto.subtle.digest`, `performance.now`. 64 MiB per mod (a setting)
   and a stack cap. A hook gets 10 s of its own time, counted by the
   interrupt handler and paused during `next` and `$` calls; `.catch`
   1 s more; each `$` await a wall bound. One `esc` drops all (ADR-0068).
4. **Fail-open, written down.** A hook that throws, overruns, re-enters
   or answers the wrong shape is skipped: the links beneath run, or its
   last `next` result stands, and one notice names plugin, event and
   reason. `.catch(($, e, next))` gets `next.error { kind, message }`. A
   `$` call raises its op chain with only the calling hook left out.
5. **Grants.** ADR-0074's tiers apply to mods. `uses` is one list of
   nouns: ADR-0069 §3's service keys are nouns, and so are the host's.
   - A mod in the user directory or under `--plugin-dir` is `rewrite`,
     with the `uses` its manifest declares: putting it there is the
     person's own act. A project-directory mod is `tighten` with the
     safe nouns only, until the user layer's `pluginTrust` grants more.
     Only that grant gives `full`.
   - The unsafe nouns are `process`, `http`, `env`, `model`, and `fs`
     writes outside the mod's data directory. A face method may declare
     itself unsafe (`agent.spawn` spends a model, `mcp.connect` starts a
     process). A call without the grant is refused in words.
   - `plugin.register` is a chain that sees `tier` and `uses`.
6. **State is the host's.** `$.state` is `host/state` (ADR-0075 §2) at
   session scope, `$.store` at global scope; both versioned, both survive
   a reload. A read in a render subscribes it (ADR-0079 §5); a write in a
   render is refused; only the owner writes, others rewrite `state.set`;
   the mod's `types/index.d.ts` declares its keys.
7. **Reload at the turn's end** (ADR-0075 §5's poll), or before one of
   the mod's own tools or commands runs: a fresh `Runtime`, `register`
   and `session.start` again; state and open panes stay.
8. **The author kit.**
   - `bingo plugins validate <dir>` parses with oxc. It lists the hooked
     events, the `$` calls, the state keys and the `uses`, and refuses a
     non-literal `on`, `$.<noun>`, state key or env name.
   - `bingo plugins test <dir>` runs `*.test.ts` in the same runtime,
     against the real host with the fake provider. `bingo/testing`
     supplies `test`, `expect`, `mock.*` and `$.ui.mount`.
   - `'bingo'` exports `Register`, each event's types under Claude
     Code's names, and `atom`, `derive`, `read`, `update`, `memberOf`. On
     load, types are written into the mod's `.bingo/types/`.

## Consequences

- `rquickjs` is a C engine, and `deny.toml` bans `onig` for being one
  where a Rust engine does the job. No Rust engine caps memory and time,
  and the sandbox rests on both. `oxc` adds about 122 crates; its parser
  is also what `validate` needs. The person accepted both (2026-10-13);
  M137 raises `budget.toml` by the count, one line per crate family.
- QuickJS-NG builds with MSVC; the Windows CI job checks it (M110). This
  record withdraws ADR-0074's and ADR-0075's "no JS engine in process".

Refs: ADR-0031, ADR-0068, ADR-0069, ADR-0074, ADR-0075, ADR-0079.
Non-goals are in the design doc's §8: `Client` modules, `blit`, audio,
`classic.*`, `prompt.edit` on each keystroke, and telemetry. Also out:
npm imports, and threads of a mod's own.
