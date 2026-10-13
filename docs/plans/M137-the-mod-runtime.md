# M137 — The mod runtime

## Goal

ADR-0078 §2–5 and §7. A plugin whose manifest names a `module` runs in
process: `bingo-mods` gives each mod a QuickJS-NG runtime, strips TS with
oxc, runs `register(on, options)`, and serves its hooks as links at M136's
entries. Sandbox, budgets, fail-open, grants and reload are each pinned by
a test; manifest, discovery and the state store move into
`bingo-plugin-host`. The acceptance: M133's `ts-status` runs in process
as it does under node.

## Bricks, in build order

1. **The shared library, moved before it grows.** Create
   `crates/bingo-plugin-host` (`tier = "library"`) and move into it
   plugin-rpc's `manifest.rs`, `discovery.rs` (with M132's `--plugin-dir`
   layer), `tool_name`, M132's `changed` over stamps and its state store
   (`swap`, the shelf layout). plugin-rpc's tests pass unedited, then its
   copies go. The manifest gains `module`, beside a `command` if it likes;
   a pure `runtime_of(manifest, mods_stand) -> Module | Process` decides,
   so one plugin name runs in exactly one runtime.
2. **Failing tests.** Fixture mods under `crates/bingo-mods/tests/mods/`:
   `hello` (its `tool.call` hook answers), `loop` (`while (true) {}`),
   `hog` (allocates past the cap), `thrower` (throws, with a `.catch`),
   `reentrant` (calls a `$` method whose event it hooks), `escaper`
   (`eval`, `new Function`, `import()`, `require`). No runtime yet.
3. **The engine, pinned, and its cost.** `rquickjs = "=0.14.0"` with
   `futures`, `loader`, `parallel` (no `bindgen`); the oxc crates at
   `=0.152.0`. A `deny.toml` comment beside the `onig` ban says why a C
   engine is admitted here (ADR-0078 Consequences). Paste
   `scripts/budget.sh` before and after; raise `max_dependencies` by the
   measured count, one comment line per family; `cargo deny check`.
   **Threading** (ADR-0078 §2): each mod's thread holds its `AsyncRuntime`
   on a current-thread tokio runtime, calls crossing by channel, so a JS
   loop never holds the worker the turn and its `esc` run on (ADR-0068).
4. **Strip and load, pure.** `strip(path, source) -> Result<String,
   StripError>`: oxc parser, semantic, transformer (TS stripped, JSX to
   `h`/`Fragment`), codegen. It refuses `import()`, `require`, and any
   specifier that is neither a relative path inside the mod's root nor
   `bingo`, naming `file:line`. The loader resolves only those two kinds.
5. **The sandbox (§3).** `Context::custom` without the `Eval` intrinsic,
   so `eval` and `Function` throw; QuickJS has no timers or WebAssembly
   to remove. Rust installs `h`/`Fragment` (JSON nodes, typed in M140),
   `URL` (`url`, in tree), UTF-8 `TextEncoder`/`TextDecoder`,
   `AbortController` (a short prelude), `crypto.randomUUID` (`uuid`),
   `crypto.subtle.digest('SHA-256')` (`bingo-digest`, M114) and
   `performance.now`. `set_memory_limit` (`mods.memoryMiB`, 64) and
   `set_max_stack_size`. Tests: every `escaper` attempt fails in words;
   `hog` is skipped as out of memory, the host lives on.
6. **One `'bingo'` for both runtimes.** M133's `on.ts` splits: `on.ts`
   keeps patterns, matcher and registration table (pure); `cross.ts`
   takes node's `intercept/`. `'bingo'` exports `Register` (the type of
   `register(on, options)`) and each event's types under Claude Code's
   names (`ToolCall`, `Turn`, …, from M136's `events.ts`); M141 adds the
   state helpers (`state.ts`), M142 their types and tests. In process it
   is `on.ts` (`include_str!`, stripped at start). `on` throws on a hook
   registered twice; one `.catch` each; `options` is the valid `config`.
7. **Hooks as links.** `Mods` implements `InterceptorSource`, one
   `Interceptor` per mod, matched by its hooks' patterns and prefilter
   (M136). `chain` runs them on the mod's thread with `e` deeply frozen;
   `next` runs the rest at most once (again throws), carrying `called`,
   `signal` (aborted on `esc` or drop), `origin {plugin, tier}`, `budget
   {ms, remainingMs}`. `observe` ignores what a hook returns.
8. **Budgets and fail-open (§3, §4).** A pure `Budget { resume(now),
   suspend(now), spent() }`, table-tested over fake instants, suspended
   around `next` and each `$` await. The interrupt handler stops a hook
   at 10 s of its own time (a setting; tests set 50 ms and assert the
   outcome, never a duration). `.catch` gets 1 s more, with `next.error
   { kind: throw | timeout | re-entry | shape, message }` and
   `next.called`. A skipped hook lets the links beneath run, or its last
   `next` result stands; one notice names plugin, event and reason. The
   runtime skips a hook already on its own stack (re-entry), so only the
   caller is left out. Each `$` await has a wall bound (tokio, paused in
   tests). A dropped chain sets the interrupt flag and aborts `signal`.
9. **Grants (§5).** Pure, in the library: `grant(origin, trust, uses) ->
   Grant { tier, uses }` (user dir or `--plugin-dir`: `rewrite` with the
   declared uses; project dir: `tighten`, safe nouns only; `pluginTrust`
   (M131) lifts either and alone gives `full`) and `admits(grant, noun,
   method)`, refusing in words `UNSAFE` (`process`, `http`, `env`,
   `model`, `fs` writes outside the data dir) and a face method declared
   unsafe (M136). `uses` is one list of nouns, service keys and the
   host's alike. `plugin.register`, a catalogue chain, is raised by core
   when a `PluginSource` first reports a plugin or reloads it, on `{ name,
   version, origin, runtime, tier, uses }`; a link lowers the tier, drops
   a use, or denies (switched off, with its reason). Both runtimes read
   the granted `uses` from `catalog(Plugins)`.
10. **Reload (§7).** A `--plugin-dir` mod is polled with `changed` on
    tokio time; a change marks it stale, and it reloads at the turn's end
    (the runtime's own link at `turn.complete`, which only calls `next`)
    or before one of its own tools or commands runs (M138 registers them;
    the check lands here). A reload drops the thread, runs `register` in
    a fresh runtime, raises `session.start` per open session; state stays.
11. **Parity, and the acceptance run.** Until M138, `$` is exactly what
    M133's `dollar.ts` offers, each a translation onto the same door:
    `$.ui.status`, `$.state`/`$.store` (the library's store),
    `$.model.complete`, `$.session.deliver` and `extend`, `$.<noun>.<method>`;
    plugin-rpc's `host/model/complete` calls the same `admits`. `ts-status`
    gains `"module": "hooks/register.ts"`; M133's scenario runs in process
    and again with `bingo-mods` switched off, so `command` runs it under node.

## Files

- `crates/bingo-plugin-host/src/{lib,manifest,discovery,tool_name,stamps,grant,state}.rs` (new)
- `crates/bingo-mods/src/{lib,strip,load,sandbox,globals,thread,link,budget,reload,dollar,guide}.rs`, `guide.md`, `tests/mods/*` (new)
- `crates/bingo-plugin-rpc/src/{manifest,discovery,tool,reload,state,host,lib}.rs` (onto the library)
- `crates/bingo-sdk/src/catalogue.rs`; `crates/bingo-core/src/{plugins.rs,host/registry.rs,chain/register.rs (new)}`
- `crates/bingo/src/{main.rs,plugins.rs}` (a runtime column), `Cargo.toml`, `Cargo.lock`
- `sdk/typescript/src/{on.ts,cross.ts (new),plugin.ts}`, `examples/plugins/ts-status/plugin.json`
- `deny.toml`, `scripts/budget.toml`

## Exit criteria

- [ ] `cargo test -p bingo-plugin-host`; plugin-rpc's tests unedited.
- [ ] `cargo test -p bingo-mods`: each `strip` refusal names `file:line`;
      `escaper`, `hog`, `loop`, `thrower`, `reentrant` each end as brick 2
      says, one notice each; `thrower`'s `.catch` sees `kind: "throw"`.
- [ ] While a `loop` hook holds `tool.check`, `esc` ends the turn
      `Interrupted` within M116's bound.
- [ ] A project-dir mod's `$.model.complete` is refused in words, and
      answers once `pluginTrust` grants `model`. A denying
      `plugin.register` link switches a mod off, its reason shown by
      `bingo plugins list`.
- [ ] A mod touched mid-turn reloads after `turn.complete` and keeps its
      `$.store` value; no sleep in the test.
- [ ] `cargo test -p bingo --test plugin_kit`: `ts-status` passes in
      process and under node (M133's skip rule).
- [ ] `budget.sh` before and after; `cargo deny check`; all gates; MSVC
      `cargo check` for the library and plugin-rpc; CI's `windows` job
      builds `bingo-mods` (log pasted), as in M109 and M110.

## Non-goals

- `$` beyond M133's set (M138, M139); elements (M140); mod `validate` and
  `test` (M142); npm imports; threads of a mod's own; timers; a runtime
  shared by two mods; `Client` modules; a file-watcher dependency;
  reloading mods that `--plugin-dir` does not name.

## Risks

- rquickjs with `parallel` and exotic classes (issue #764): natives are
  plain functions and objects. The interrupt fires only while JS runs, so
  no native blocks; each spawns onto the host and awaits under a bound.
- QuickJS-NG under MSVC is tested only in CI; a failure is fixed before
  landing, never `cfg`-gated away. oxc releases weekly: the pin is exact.
- A thread per mod parks when idle: paste 50 fixture mods' RSS, assert none.
- Re-verify at start: M132's `state.rs`, `reload.rs`, `intercept.rs`,
  `host/model/complete`, `--plugin-dir`; M133's `on.ts`, `dollar.ts`,
  `ts-status`, `plugin_kit`; M136's `chain`, `observe`, `catalogue!`;
  M131's `pluginTrust`, `Tier`; M116's owner; M114's `bingo-digest`.
