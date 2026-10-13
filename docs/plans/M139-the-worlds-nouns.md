# M139 — The world's nouns

## Goal

ADR-0078's `$`, part II (mods-parity §3). A mod reaches the world outside the session: `fs`, `http`, `process`
and `env` through faces the runtime registers (ADR-0078 §1), and `mcp` and `agent` as noun events of the
plugins that own them. Each method is raised as `<noun>.<method>`. A call without the grant is refused in
words that say how to grant it, and a granted unsafe noun is put on the record. `http` and `process` run on
`bingo-http` (M127) and `bingo-process` (M128): no new crate.

## Bricks, in build order

1. **Contract and failing tests.** M138's table gains a row per method: `fs` read, write, list, exists, stat,
   ancestors; `http` fetch; `process` run, spawn, read, write, kill; `env` get, set; `mcp` call, connect;
   `agent` spawn, list, register. Failing tests use fixture mods under `tests/mods/world/`. A
   project-directory mod is refused at every unsafe method. With a `pluginTrust` grant, the same call answers.
   A link sees each name once.
2. **Scope, pure.** `scope(path, Roots { project, data }, Access) -> Result<PathBuf, Refusal>`:
   - It normalises lexically, then canonicalises the longest existing ancestor with `dunce`.
   - It refuses a `..` or a symlink that leaves both roots.
   - A read may land in either root. A write may land in the mod's data directory
     (`<data_dir>/plugins/<name>/data`). Any other write is `fs` beyond the safe set (ADR-0078 §5).
   - `cap(len, 4 MiB)` refuses with both sizes.
   - Table tests. The Windows cases (a drive prefix, `\\?\`, UNC) carry the same `cfg` gate as the unix ones.
3. **`fs`.** A face over `tokio::fs`, with each path through `scope`.
   - `read` returns UTF-8 text or base64.
   - `list` and `stat` return no more than they are asked for.
   - `ancestors(name)` lists the directories from `cwd` up to the project root that hold `name`.
4. **`http.fetch`.** One client per runtime, from `bingo_http::client(Bounds::CALL)`.
   - The request is `{ url, method, headers, body }`, http and https only.
   - The response is `{ status, headers, body }`. A body is cut at 4 MiB, and the cut is said in words.
   - The hook's `signal` aborts the request.
5. **`process`.** `bingo_process::tree` (M128) over an argv, with no shell unless the argv names one. `cwd`
   goes through `scope`. Each stream is capped at 4 MiB, and `run` takes a timeout bounded by the wall bound.
   - `spawn` returns a handle id minted for this mod, with `read`, `write` and `kill`.
   - The JS handle in Claude Code's shape is built over those three.
   - A mod reaches only its own trees. A tree dies with its handle, at a reload and at `session.end` (`Drop`,
     then `terminate(GRACE)`).
6. **`env`.** `get` reads the host's environment. `set` writes a per-mod overlay that only that mod's
   `process.*` sees. Rust 2024 makes `std::env::set_var` unsafe, and this workspace forbids it. Names are
   literals: M142's `validate` refuses a computed name, and the runtime refuses a name that is not a valid
   identifier.
7. **Nouns by name, and unsafe by declaration.**
   - M136's registry indexes each face under its noun as well as its key, so `$.<noun>` resolves through
     `service_wire(noun)`. The registry refuses a noun that equals another key.
   - `admits` reads M136's per-method `unsafe` declaration for faces, and `UNSAFE` for the host's nouns
     (ADR-0078 §5). `uses` is one list of nouns, so a grant names `mcp` or `agent` as it names `http`.
8. **`mcp` and `agent`.** These are the owning plugins' faces, not bingo-mods'.
   - bingo-mcp's face gains `call { server, tool, args }` and `connect { server }` over `Manager`, for
     configured servers only. `connect` starts a process, so it declares itself unsafe (ADR-0078 §5).
   - bingo-agents' face gains `spawn`, `list` and `register`. `register` adds a definition to the agents'
     library for the host's lifetime. `spawn` spends a model, so it declares itself unsafe.
   - The `Agent` tool calls its own `spawn` through `service_wire`, so `agent.spawn` is raised for the model's
     spawns too.
9. **Words and record.** A pure `refusal(plugin, noun, method, reason)` is golden-tested: `"<plugin> may not
   <noun>.<method>: <reason>. Grant it under pluginTrust in <user settings path>."`
   - The first granted use of each unsafe noun by a plugin in a session posts one notice line.
   - Every use is logged at `info`.
   - plugin-rpc's bridge calls the same `admits` before a process's `service/call` reaches an unsafe face,
     binding `plugin` as M138 does.
10. **Node parity.** `dollar.ts` reaches every row by `service/call`. A fixture calls one method of each noun,
   and gives the same output in process and under node.

## Files

- `crates/bingo-mods/src/dollar/` (world faces, scope, table, contracts),
  `src/{plugin.rs,manager.rs,manager/reload.rs}`, `tests/mods/world/*`, `tests/world.rs`,
  `tests/support/mod.rs`, `guide.md`, `Cargo.toml`.
- `crates/bingo-plugin-host/src/{grant.rs,ops.rs}` (shared admission and caller binding);
  `crates/bingo-plugin-rpc/src/service.rs`.
- `crates/bingo-sdk/src/service.rs` (M136's actual registry owner); `crates/bingo-core/src/host.rs` and
  service contract tests.
- `crates/bingo-mcp/src/{lib.rs,rows.rs,manager.rs}`, `tests/server.rs`; `crates/bingo-services/src/mcp.rs`
  (the existing shared face contract).
- `crates/bingo-agents/src/{lib.rs,spawn.rs,list.rs,library.rs,service.rs}` and their tests;
  `src/definition.rs` (serializable definition contract).
- `schema/*.json`, `sdk/typescript/src/{dollar.ts,dollar-types.ts,wire.ts}`,
  `crates/bingo/tests/plugin_kit.rs`, world parity fixtures.
- `Cargo.lock` for existing workspace dependency edges only. Reuse `bingo-http`, `bingo-process`, `dunce`,
  `reqwest` (HTTP client types) and `base64` (binary filesystem results); no new resolved crate.

## Exit criteria

- [ ] `cargo test -p bingo-mods scope::`: `..`, a symlink out, and a write outside the data directory are each
  refused. A read inside either root passes. A 4 MiB + 1 read is refused with both sizes.
- [ ] `fetch` against M127's local test peer answers. An over-cap body is cut and says so. `esc` aborts a
  pending fetch.
- [ ] A spawned tree's stdout reaches EOF after a reload: M128's test, with no signal probe. Killing an id of
  another mod is refused.
- [ ] `env.set` is visible to the mod's child process, and `std::env::var` in the host is unchanged.
- [ ] `mcp.call` reaches bingo-mcp's test server. A link at `agent.spawn` sees the model's own `Agent` call.
- [ ] Refusal goldens pass. One notice per (plugin, unsafe noun, session).
- [ ] `cargo test -p bingo --test plugin_kit`: the world fixture gives the same output in process and under
  node.
- [ ] All gates pass. `cargo check --target x86_64-pc-windows-msvc` checks every touched crate; missing
  third-party Windows C headers are left to Windows CI under protocol step 7.
- [ ] `scripts/budget.sh` shows no change in crates. **Blocked:** 411 normal dependencies; limit 410.

## Non-goals

npm, sockets, a PTY, streamed fetch, changing the host environment, unconfigured MCP servers, audio, telemetry.

## Risks

- **TOCTOU.** A symlink swapped between `scope` and the open defeats the check. Open the canonical path that
  `scope` returned. The residue is written in the guide.
- **Localhost.** A granted `http` reaches localhost. The grant is the person's own act, and the refusal words
  name the noun.
- **Windows paths.** `\\?\` against plain prefixes: canonicalise both sides with `dunce`, and test it in CI.
- Re-verified at `5581b978` on 2026-10-14: M108–M138 and M147–M150 are present. M127 exports
  `client(Bounds::CALL)`; M128 exports `tree` and `Tree::terminate`. M138 owns the table in `dollar/table.rs`,
  faces in `dollar/runtime.rs`, and caller identity in plugin-host `ops.rs`. M136's `Services` lives in sdk
  `service.rs`, not core `host.rs`. The MCP face lives in services `mcp.rs`; agents currently expose only the
  separate `team` face, so the agent noun needs its own owner face.
- Preserve process-plugin default `tighten`; only user/explicit modules default to `rewrite`. Project modules
  need user-layer grants for unsafe calls, including dynamically unsafe filesystem writes.
- M140 owns sdk view/render, core chain/render, TUI and surface-rpc. M139 does not edit those; generated files
  are reconciled at merge.
- **Stopped under protocol step 6 (2026-10-14).** The required dependency budget fails after
  adding the planned `dunce` runtime edge: 411 normal dependencies, limit 410. It was previously
  only an `aws-lc-sys` build dependency. The plan's no-change assumption was wrong; no new
  version was resolved. Raising `scripts/budget.toml` is outside Files and requires an ADR line.
  Options: authorize that file and document a 410 → 411 allowance for `dunce` (recommended),
  or revise brick 2 to use a separately reviewed canonicalization implementation without `dunce`.
  Scope and noun lookup are committed; world faces, integration, parity, schema regeneration and remaining gates
  are unfinished. Scope emits dead-code warnings until consumed. All exit criteria remain unticked.

## Verified (2026-10-14)

```text
cargo test -p bingo-sdk noun_lookup_resolves_the_same_wire_and_face --locked
test service::tests::noun_lookup_resolves_the_same_wire_and_face ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 138 filtered out; finished in 0.02s
cargo test -p bingo-mods world_methods_have_one_declared_door --locked
assertion `left == right` failed: fs.read
  left: 0
 right: 1
cargo test -p bingo-sdk service::tests --locked
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 131 filtered out; finished in 0.00s
cargo test -p bingo-mods scope::
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 14 filtered out; finished in 0.00s
scripts/budget.sh
dependencies (unique, normal): 411 (max  410)
FAIL: too many dependencies
warm cargo check -p bingo-core: 0s (max  20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
target/debug: 2 GB (soft max  5)
test binaries: 25
budget FAILED
```
