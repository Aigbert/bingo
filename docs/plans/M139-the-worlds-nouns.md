# M139 — The world's nouns

## Goal

ADR-0078's `$`, part II (mods-parity §3): fs/http/process/env runtime faces and owner-plugin mcp/agent
faces raise every method once. Scoped I/O, explicit grants, actionable refusals and unsafe-use records
share QuickJS and Node semantics. Reuse M127's HTTP client and M128's process trees; no new crate.

## Bricks, in build order

1. Contracts and failing fixtures first: fs read/write/list/exists/stat/ancestors; http fetch;
   process run/spawn/read/write/kill; env get/set; mcp call/connect; agent spawn/list/register.
2. Pure scope normalizes lexically, canonicalizes the longest existing ancestor, and refuses traversal
   or symlink escapes from project/data roots. Safe writes stay in `<data_dir>/plugins/<name>/data`;
   granted writes may use the project. Test drive/verbatim/UNC paths with platform gates.
   Authorized 2026-10-14: budget 410 → 411 for zero-dependency `dunce`, Windows-safe without UNC prefixes.
3. Files use scoped canonical paths, UTF-8/base64 and a 4 MiB cap reporting actual/limit sizes; bounded
   lists, fixed stat metadata, and ancestors from session cwd to project root for one filename.
4. One `bingo_http::client(Bounds::CALL)` per runtime; HTTP(S) request URL/method/headers/body,
   response status/headers/body with explicit 4 MiB truncation notice. Dropping the hook cancels I/O.
5. `bingo_process::tree` executes argv directly with scoped cwd, bounded streams and run timeout.
   Mint mod/session-owned handles and JS read/write/kill wrappers; reload/session end terminates trees.
6. Env get reads the host; set stores a per-mod child overlay. Validate identifiers without host mutation;
   M142 later rejects computed names statically.
7. Resolve nouns through the existing registry; reject noun/key collisions. Shared admission reads face
   unsafe metadata and host UNSAFE declarations. Process default stays tighten; user/explicit modules
   default rewrite; project modules stay tighten until user-layer pluginTrust grants more.
8. MCP owns configured-server call/connect; connect is unsafe. Agents owns spawn/list/register with
   host-lifetime definitions; unsafe spawn is also the model SpawnAgent tool's service-wire path.
9. Golden refusal names plugin/noun/method/reason/settings path. Log every unsafe use and post one notice
   per plugin/noun/session. Bind caller identity in both transports.
10. Generate schema and TypeScript from the table; one fixture exercises every noun in QuickJS and Node.

## Files

- `scripts/budget.toml`; `Cargo.lock` for existing dependency edges only.
- `crates/bingo-mods/src/dollar/`, `src/{plugin.rs,manager.rs,manager/reload.rs}`, `tests/mods/world/*`,
  `tests/{world.rs,support/mod.rs}`, `guide.md`, `Cargo.toml`.
- `crates/bingo-plugin-host/src/{grant.rs,ops.rs}`; `crates/bingo-plugin-rpc/src/service.rs`.
- `crates/bingo-sdk/src/service.rs`; `crates/bingo-core/src/host.rs` and service contract tests.
- `crates/bingo-mcp/src/{lib.rs,rows.rs,manager.rs}`, `tests/server.rs`; `crates/bingo-services/src/mcp.rs`.
- `crates/bingo-agents/src/{lib.rs,spawn.rs,list.rs,library.rs,service.rs,definition.rs}` and their tests.
- `schema/*.json`, `sdk/typescript/src/{dollar.ts,dollar-types.ts,wire.ts}`,
  `crates/bingo/tests/plugin_kit.rs`, world parity fixtures.
- Reuse bingo-http, bingo-process, dunce, reqwest and base64; no newly resolved crate.

## Exit criteria

- [x] Scope refuses traversal/symlink escapes and unsafe writes; root reads pass and over-cap reads report sizes.
- [x] Local fetch answers, reports truncation, and esc aborts a pending request.
- [x] Generation cancellation closes descendant stdout; reload removes handles and foreign ids are refused.
- [x] Env overlay reaches only the mod's children; the host environment is unchanged.
- [x] MCP reaches the local test server; a link sees the model's SpawnAgent call at agent.spawn.
- [x] Refusal goldens pass; one notice per plugin/unsafe noun/session.
- [x] The world fixture produces identical QuickJS/Node output with black-box NDJSON coverage.
- [x] Gates pass, with Windows third-party C-header deferrals under protocol step 7 (details below).
- [x] Budget passes at the authorized 411 normal dependencies; no new resolved crate.

## Non-goals

npm, sockets, PTYs, streamed fetch, host environment mutation, unconfigured MCP servers, audio (no player or audio stack), telemetry.

## Risks

- TOCTOU remains if a symlink changes after scope; open the canonical result and document the residue.
- Granted HTTP can reach localhost; refusal text names the required noun.
- Re-verified at `5581b978` (2026-10-14): M108–M138/M147–M150 landed; M127 client and M128 tree exist.
  M138 owns dollar table/runtime and caller ops; registry is SDK service.rs; MCP contract is in services;
  agents needed an owner face. Preserve ADR-0074 process tighten and ADR-0078 module grant defaults.
- M140 owns SDK view/render, core chain/render, TUI and surface-rpc; only generated files need reconciliation.
- The authorized budget increase moves dunce from build-only to normal; no ADR named dunce or changed decision.
- The deletion-race stop was resolved on 2026-10-14 by authorized merge `9fd172c3` of dev's `878f3c10`.
- Windows SDK/services/plugin-host/agents/core pass. Plugin-rpc/MCP await aws-lc-sys headers; mods/bin also
  await rquickjs-sys headers (`stdlib.h`/`windows.h`). No Rust error; Windows CI owns those builds, no toolchain installed.
- Budget has a 9 GB debug-directory soft warning. No live/credential tests ran; ACP token tests ran separately.

## Verified (2026-10-14)

```text
cargo test -p bingo-sdk noun_lookup_resolves_the_same_wire_and_face --locked
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 138 filtered out; finished in 0.02s
cargo test -p bingo-mods world_methods_have_one_declared_door --locked
assertion `left == right` failed: fs.read
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk -p bingo-plugin-rpc -p bingo-mods --lib --locked
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
test result: ok. 130 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.33s
test result: ok. 139 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.43s
cargo fmt --all -- --check
exit_code: 0
cargo check --workspace --all-targets --locked
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 22.78s
cargo clippy --workspace --all-targets --locked -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 25.65s
cargo test --workspace --locked -- --skip a_proxy_with_a_token_this_run_never_minted_gets_nothing --skip a_token_nobody_minted_never_gets_a_conversation --skip a_second_concurrent_stream_on_one_token_is_refused --skip a_token_may_be_dialled_again_once_its_stream_has_closed
exit_code: 0
test delete_removes_the_session_from_disk_and_shutdown_exits_zero ... ok
test the_models_spawn_agent_call_crosses_the_agent_noun_chain ... ok
test world_nouns_give_the_same_answer_in_process_and_under_node ... ok
test grant::world_tests::refusal_words_name_the_operation_and_actual_settings ... ok
test the_mcp_noun_connects_and_calls_the_local_server_through_invoke ... ok
test dollar::scope::tests::symlinks_cannot_leave_either_root ... ok
test dollar::scope::tests::roots_bound_reads_and_writes_including_new_paths ... ok
test dollar::process::tests::generation_drop_closes_grandchild_stdout_and_foreign_ids_are_refused ... ok
test dollar::http::tests::local_fetch_answers_and_reports_a_cut_body ... ok
test esc_drops_a_pending_http_request ... ok
test a_granted_noun_posts_one_notice_per_session ... ok
test environment_overlay_reaches_only_its_child ... ok
test filesystem_methods_are_bounded_and_support_binary_data ... ok
test spawned_process_streams_are_owned_and_reload_closes_the_tree ... ok
test grant::world_tests::unsafe_nouns_are_recorded_once_per_plugin_and_session ... ok
cargo test -p bingo --test acp_bridge --locked a_proxy_with_a_token_this_run_never_minted_gets_nothing -- --exact --test-threads=1
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.04s
cargo test -p bingo-provider-acp --test bridge --locked token -- --test-threads=1
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 0.01s
scripts/check_discipline.sh
discipline ok
scripts/budget.sh
dependencies (unique, normal): 411 (max  411)
warm cargo check -p bingo-core: 0s (max  20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
target/debug: 9 GB (soft max  5)
budget ok
cargo deny check
advisories ok, bans ok, licenses ok, sources ok
cargo check -p bingo-sdk --all-targets --target x86_64-pc-windows-msvc
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 5.40s
cargo check -p bingo-services --all-targets --target x86_64-pc-windows-msvc
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.34s
cargo check -p bingo-plugin-host --all-targets --target x86_64-pc-windows-msvc
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.72s
cargo check -p bingo-plugin-rpc --all-targets --target x86_64-pc-windows-msvc
error: failed to run custom build command for `aws-lc-sys v0.44.0`
CC_x86_64_pc_windows_msvc='clang --driver-mode=cl' cargo check -p bingo-mods --all-targets --target x86_64-pc-windows-msvc
error: failed to run custom build command for `rquickjs-sys v0.14.0`
error: failed to run custom build command for `aws-lc-sys v0.44.0`
cargo check -p bingo-mcp --all-targets --target x86_64-pc-windows-msvc
error: failed to run custom build command for `aws-lc-sys v0.44.0`
cargo check -p bingo-agents --all-targets --target x86_64-pc-windows-msvc
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.80s
cargo check -p bingo-core --all-targets --target x86_64-pc-windows-msvc
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.00s
CC_x86_64_pc_windows_msvc='clang --driver-mode=cl' cargo check -p bingo --all-targets --target x86_64-pc-windows-msvc
error: failed to run custom build command for `aws-lc-sys v0.44.0`
error: failed to run custom build command for `rquickjs-sys v0.14.0`
```
