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
`--plugin-dir` directories; the last directory for a name wins. Disabling
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

`$` currently offers state/store get and CAS set, UI status, model completion,
session extend/deliver, and registered service faces. State belongs to the host
and survives reload. Explicit directories are polled once a second. An edit is
marked stale until `turn.complete`; reload joins the old thread, registers a
fresh runtime, adjudicates its grants and repeats `session.start` for open
sessions. The scenario harness can request a reload explicitly.

`plugins.<name>` supplies options (an empty object when absent). Manifest config
schemas validate types, object properties/required/additionalProperties, arrays,
choices, local references, composition and size/numeric bounds. Unsupported
validation keywords are refused with a named reason instead of being ignored.
Full author-kit analysis and module-owned tools/commands belong to later plans.

For a repeatable local memory measurement, run:

```sh
cargo test -p bingo-mods --test engine fifty_idle_modules_rss --locked -- --ignored --nocapture
```

The 2026-10-14 macOS run measured 3,232 KiB before and 29,792 KiB with 50 idle
modules. This is a measurement, not a CI threshold.
