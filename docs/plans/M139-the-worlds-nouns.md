# M139 — The world's nouns

## Goal

ADR-0078's `$`, part II (mods-parity §3). A mod reaches the world outside the session: `fs`, `http`, `process`
and `env` through faces the runtime registers (ADR-0078 §1), and `mcp` and `agent` as noun events of the
plugins that own them. Each method is raised as `<noun>.<method>`. A call without the grant is refused in
words that say how to grant it, and a granted unsafe noun is put on the record. `http` and `process` run on
`bingo-http` (M127) and `bingo-process` (M128): no new crate.

## Bricks, in build order

1. Commit contracts and failing fixtures first. One table row and one noun event per world method:
   fs read/write/list/exists/stat/ancestors; http fetch; process run/spawn/read/write/kill; env get/set;
   mcp call/connect; agent spawn/list/register. Project unsafe calls require user-layer trust.
2. Pure `scope(path, Roots { project, data }, Access)` lexically normalizes then canonicalizes the longest
   existing ancestor with `dunce`. Refuse escapes from both roots, including symlinks; safe writes stay in
   `<data_dir>/plugins/<name>/data`. Granted writes may use the project. `cap` reports actual/limit bytes.
   Authorized 2026-10-14: budget 410 → 411 for zero-dependency `dunce`, Windows-safe without UNC prefixes.
   Table-test traversal, symlinks, drive/verbatim/UNC prefixes with matching platform gates.
3. Files use canonical scoped paths; UTF-8/base64 reads and writes cap at 4 MiB, lists are bounded,
   stat returns fixed metadata, ancestors walks session cwd to project root for one filename.
4. One `bingo_http::client(Bounds::CALL)` per runtime; HTTP(S) request URL/method/headers/body, response
   status/headers/body with explicit 4 MiB truncation notice. Dropping the hook cancels pending I/O.
5. `bingo_process::tree` executes argv directly, scoped cwd, 4 MiB per stream, run timeout within hook wall
   bound. Mint mod/session-owned handles with read/write/kill and JS wrappers. Reload/session end kills
   owned trees; generation cancellation closes descendant stdout. Foreign handles are refused.
6. Env get reads the host; set stores a per-mod child overlay and never mutates the host. Validate literal
   identifier syntax at runtime; M142 later rejects computed names statically.
7. Resolve face nouns through the existing registry; reject noun/key collisions. Shared admission reads
   per-method unsafe metadata plus host UNSAFE declarations; process plugins retain default tighten,
   user/explicit modules rewrite, project modules tighten until user-layer pluginTrust grants more.
8. MCP owns call/connect for configured servers; connect is unsafe. Agents owns spawn/list/register,
   definitions live for the host lifetime, spawn is unsafe, and model SpawnAgent crosses the same face.
9. Golden refusal names plugin/noun/method/reason and actual user settings path. Log every granted unsafe
   use at info and post one notice per plugin/noun/session. Bind caller identity in both transports.
10. Generate schema and TypeScript from the table. A fixture uses one method of every noun and produces
    the same result under QuickJS and Node through service/call.

## Files

- `scripts/budget.toml` (authorized 410 → 411 normal-dependency allowance).
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

- [x] `cargo test -p bingo-mods scope::`: `..`, a symlink out, and a write outside the data directory are each
  refused. A read inside either root passes. A 4 MiB + 1 read is refused with both sizes.
- [x] `fetch` against M127's local test peer answers. An over-cap body is cut and says so. `esc` aborts a
  pending fetch.
- [x] A spawned tree's stdout reaches EOF after a reload: M128's test, with no signal probe. Killing an id of
  another mod is refused.
- [x] `env.set` is visible to the mod's child process, and `std::env::var` in the host is unchanged.
- [x] `mcp.call` reaches bingo-mcp's test server. A link at `agent.spawn` sees the model's own `Agent` call.
- [ ] Refusal goldens: workspace stopped before plugin-host tests; one-notice integration passed.
- [x] `cargo test -p bingo --test plugin_kit`: the world fixture gives the same output in process and under
  node.
- [ ] All gates: fmt/check/clippy pass; workspace fails as below. Isolated ACP, final discipline/budget/deny
  and Windows checks remain pending because protocol step 6 stops the run.
- [ ] Final budget run at the authorized 411 remains pending; no new resolved crate.

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
- The initial budget stop is resolved by the user’s 2026-10-14 authorization: `dunce` moves from build-only
  to normal use. No ADR named `dunce`; ADR-0078’s grants and filesystem boundary remain unchanged.

- **Protocol stop (2026-10-14).** Workspace RPC deletion failed with `Directory not empty`; the isolated
  rerun failed identically. Step 6 stops after two attempts without progress. Dev's `878f3c10` fixes this
  actor-removal race; the user reserved it for merge reconciliation, so it was not applied here.
  Options: authorize bringing that fix into this branch, or resume gates after reconciliation.
  M139 implementation is committed; milestone verification is incomplete. No live/credential tests ran.

## Verified (2026-10-14)

```text
cargo test -p bingo-sdk noun_lookup_resolves_the_same_wire_and_face --locked
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 138 filtered out; finished in 0.02s
cargo test -p bingo-mods world_methods_have_one_declared_door --locked
assertion `left == right` failed: fs.read
cargo test -p bingo-sdk service::tests --locked
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 131 filtered out; finished in 0.00s
scripts/budget.sh
dependencies (unique, normal): 411 (max  410)
budget FAILED
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk -p bingo-plugin-rpc -p bingo-mods --lib --locked
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 130 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.33s
test result: ok. 139 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.43s
cargo test -p bingo-mods --test world --locked
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
cargo test -p bingo-mcp --locked --test server the_mcp_noun_connects_and_calls_the_local_server_through_invoke
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 19 filtered out; finished in 0.04s
cargo fmt --all -- --check
exit_code: 0
cargo check --workspace --all-targets --locked
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 22.78s
cargo clippy --workspace --all-targets --locked -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 25.65s
cargo test --workspace --locked -- --skip a_proxy_with_a_token_this_run_never_minted_gets_nothing --skip a_token_nobody_minted_never_gets_a_conversation --skip a_second_concurrent_stream_on_one_token_is_refused --skip a_token_may_be_dialled_again_once_its_stream_has_closed
test the_models_spawn_agent_call_crosses_the_agent_noun_chain ... ok
test world_nouns_give_the_same_answer_in_process_and_under_node ... ok
test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.72s
test delete_removes_the_session_from_disk_and_shutdown_exits_zero ... FAILED
test result: FAILED. 14 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.72s
cargo test -p bingo --test rpc --locked delete_removes_the_session_from_disk_and_shutdown_exits_zero -- --exact --test-threads=1
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 14 filtered out; finished in 0.37s
```
