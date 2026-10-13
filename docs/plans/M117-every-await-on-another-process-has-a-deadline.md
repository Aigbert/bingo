# M117 — Every await on another process has a deadline

## Goal

ADR-0068 §5. M116 makes esc reach every turn; unattended sessions get no
esc. Each plugin bounds its own external calls with named constants and,
where longer work is legitimate, a setting. Expiry returns an error rather
than hanging a scheduled run, channel bot or subagent.

## Bricks, in build order

1. **Failing tests first**, using paused Tokio time (`test-util` added to
   ACP and auth-oauth dev dependencies; no new dependency):
   - MCP's real stdio fixture never answers: `ToolError::Failed` names the
     server after 600 s by default and after `toolTimeout: 5`.
   - The plugin fixture's tool and command never answer: errors at 600 s
     or manifest `callTimeout`; tool expiry still sends the abandon notice.
   - ACP ignores cancel: after 10 s its process exits even after the turn
     drops its stream, and the next prompt spawns afresh. A responsive
     cancel keeps the child (the passing control).
   - A local TCP server accepts but never writes: token refresh and MCP
     discovery return timeout errors at the total bound.
2. **plugin-rpc.** `TOOL_CALL` and `COMMAND_RUN` are 10 min, documented
   beside the existing bounds; assert `SERVICE < COMMAND_RUN`.
   Optional manifest `callTimeout` is seconds for both verbs; absent uses
   their constants. Pin the schema and round-trip fixture before behavior.
   Race the tool loop against a deadline; wrap command `request` in timeout.
   Errors name plugin, verb, duration and setting; retain `Abandons`.
3. **MCP.** `TOOL_CALL = 10 min` beside `CONNECT_TIMEOUT`. Both server
   transports accept positive `toolTimeout` seconds; zero is refused.
   Preserve unknown-field strictness and round trips; pin the settings
   schema and document the setting in the guide. Add a deadline select arm.
4. **ACP.** `CANCEL_GRACE = 10 s` after sending `session/cancel`.
   The link owns the prompt task: M116 may drop the turn at 2 s.
   Expiry marks the link dead and drops its adapter; `process-wrap` kills
   its tree on Unix and Windows. The next prompt uses the fresh-child path.
   Record on ADR-0035 §6 that the child survives only while cancel is
   answered within the grace. Closing the link aborts its owned work.
5. **HTTP.** Export `bingo_auth_oauth::client() -> reqwest::Client` with
   10 s connect and 30 s total bounds for small token/discovery requests.
   Use it only at OpenAI subscription's `TokenSource`, MCP auth `handles`,
   and `bingo/src/mcp.rs`; test constructions remain. Preserve timeout
   errors through discovery. Provider streams retain ADR-0056's bounds.
6. **Kernel.** No changes: use existing `ToolError::Failed` and `KernelError`.
7. Accept ADR-0068 §5 and note that §1–4 landed in M116. Its Amends line
   already names ADR-0035 §6; add the dated note to that ADR.

## Files

- `bingo-plugin-rpc/src/{deadline,manifest,tool,command,schema,bridge,manager,connection}.rs`
  and its `examples/stub_plugin/**`, `tests/plugin/**`, `schema/plugin.json`
- `bingo-mcp/src/{tool,config,dial,auth,manager}.rs`, guide, snapshot, examples, tests
- `bingo-provider-acp/src/{provider,connection,child,session}.rs`, fake agent and tests
- `bingo-auth-oauth/src/{lib,http,discover,error}.rs`; ACP/auth dev `Cargo.toml`
- `bingo-provider-openai/src/lib.rs`, `bingo/src/mcp.rs`
- Mechanical `Server` constructor updates: MCP `src/command.rs`, tests, bin `src/mcp.rs`
- `docs/adr/{0068,0035}*.md`; crate paths above are under `crates/`.

## Exit criteria

- [x] Brick 1 regressions fail before and pass after; responsive control stays green.
- [x] `rg -n 'reqwest::Client::new\(\)' crates --glob '!**/tests/**'` shows only
  test modules plus exactly the three builder fallbacks annotated below as M127 brick 6.
- [x] `rg -n '\.request\(' crates/bingo-plugin-rpc/src`: every wait is bounded,
  including requests bounded by their caller; each hit is accounted for below.
- [x] Real stdio MCP fixture at `toolTimeout: 1`: the model receives the timeout.
- [x] Native gates pass; Windows ACP passes and five header-blocked checks go to CI (protocol §7).

## Non-goals

One HTTP stack (M127), total timeouts on provider streams, kernel tool timeouts.

## Risks

- Ten minutes can cut a legitimate build/crawl; the error names its override.
- ACP killing loses agent state; the existing fresh-child ladder uses load when supported.
- 2026-10-13 decision: preserve ADR-0056 stream bounds; only brick 5's three clients change; the three builder-failure fallbacks remain for M127 brick 6.
- Re-verified at `2e3d9065`: bridge/manager carry settings, `session::Link` owns
  ACP's adapter, discovery hid timeout errors, and ACP/auth lacked dev `test-util`.
  M113/M120 implementation files are untouched; no dependency or budget change.

## Verified (2026-10-13)

Before implementation: `cargo test -p bingo-plugin-rpc -p bingo-mcp -p bingo-provider-acp -p bingo-auth-oauth --locked deadline --no-fail-fast`:
```text
OAuth requests must have a total deadline: Elapsed(())                         # 2 tests
MCP calls must have a deadline: Elapsed(()); unknown field `toolTimeout`      # call/config
command/run must have a deadline: Elapsed(()); tool/call ... Elapsed(())      # 4 tests
manifest round trip: left: Null; right: Number(5)
the ignored cancel must kill the child: Elapsed(())
deadline_keeps_an_agent_that_answers_cancel ... ok
```
The example binaries were built first with `cargo build -p bingo-plugin-rpc -p bingo-mcp --examples --locked`.
After implementation: `cargo test -p bingo-plugin-rpc --locked deadline`: `7 passed` (lib), `5 passed` (integration).
`cargo test -p bingo-mcp --locked deadline`: `2 passed` (lib), `3 passed` (integration).
`cargo test -p bingo-provider-acp --test deadlines --locked`: `2 passed; 0 failed`.
`cargo test -p bingo-auth-oauth --locked deadline`: `2 passed; 0 failed`.
Schema generators: `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc --lib --locked schema::tests::the_committed_schema_is_this_document` and
`INSTA_UPDATE=always cargo test -p bingo-mcp --lib --locked deadline_settings_schema_is_pinned`: `1 passed` each; workspace run verifies them without updates.

`cargo fmt --all -- --check`: exit 0, no output (fixture indentation corrected).
`cargo check --workspace --all-targets --locked`: `Finished dev ... in 39.22s`.
`cargo clippy --workspace --all-targets --locked -- -D warnings`: `Finished dev ... in 17.19s` (new test's lint usage corrected).
`INSTA_UPDATE=no cargo test --workspace --locked -- --test-threads=1 --skip a_proxy_with_a_token_this_run_never_minted_gets_nothing --skip a_token_may_be_dialled_again_once_its_stream_has_closed`:
exit 0; summed `5044 passed; 0 failed; 4 ignored` (existing manual/stress tests); all 16 PTY tests passed first run.
`cargo test -p bingo --test acp_bridge --locked a_proxy_with_a_token_this_run_never_minted_gets_nothing -- --exact --test-threads=1` and
`cargo test -p bingo-provider-acp --test bridge --locked a_token_may_be_dialled_again_once_its_stream_has_closed -- --exact --test-threads=1`:
`1 passed; 0 failed` each. No silent skips or simultaneous gates; no TUI-byte changes requiring a separate smoke.
`scripts/check_discipline.sh`: `discipline ok` (advisory warnings).
`scripts/budget.sh`: `348 (max 348)`; core `1s`; isolation `0`; target `6 GB` (soft max 5); `budget ok`.
`cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.

Client search output (grouped hit lines; paths relative to `crates/`):
```text
bingo-auth-oauth/src/mcp.rs:542,890,975; source.rs:454                         # cfg(test) at 469,366
bingo-auth-oauth/src/challenge.rs:207,228,247,262; register.rs:134,158          # cfg(test) at 139,67
bingo-auth-oauth/src/discover.rs:435,483,503,539,554                           # cfg(test) at 285
bingo-mcp/src/auth.rs:146,160,177                                             # cfg(test) at 121
bingo-tool-web/src/page.rs:216                                               # cfg(test) at 195
bingo-channel-feishu/src/feishu/token.rs:133,156,171                          # cfg(test) at 105
bingo-provider-openai/src/lib.rs:494: .unwrap_or_else(|_| reqwest::Client::new()) # M127 brick 6
bingo-provider-anthropic/src/lib.rs:265: .unwrap_or_else(|_| reqwest::Client::new()) # M127 brick 6
bingo-channel-feishu/src/feishu/api.rs:96: .unwrap_or_else(|_| reqwest::Client::new()), # M127 brick 6
```
Request search output (`crates/bingo-plugin-rpc/src/`, each `.request(` hit):
```text
connection.rs:540 initialize          # test's HANDSHAKE timeout
contributor.rs:67 CONTEXT_CONTRIBUTE  # CONTRIBUTE timeout
service.rs:91 SERVICE_CALL           # SERVICE timeout
compactor.rs:56 COMPACTOR_COMPACT     # COMPACT timeout
hook.rs:89 HOOK_DECIDE               # HOOK timeout
tool.rs:86 TOOL_CALL                 # pinned deadline arm in exchange
command.rs:59 COMMAND_RUN            # configured timeout
bridge.rs:480 INITIALIZE             # handshake wraps initialize in HANDSHAKE timeout
provider.rs:141 PROVIDER_STREAM      # Body::wait races its handle against PROVIDER_IDLE; Drop aborts it
```
`python3 target/m117/smoke.py` (isolated home, real stdio fixture, print JSON, scripted model matching the timeout):
`exit=0; 25 valid NDJSON lines; toolTimeout=1; model received timeout`.
The verifier was corrected for command output and tagged frame shapes before this successful run.

`cargo check -p <crate> --all-targets --locked --target x86_64-pc-windows-msvc`, run separately for all six touched crates:
`bingo-provider-acp`: `Finished dev ... in 10.57s`.
`bingo-auth-oauth`, `bingo-plugin-rpc`, `bingo-mcp`, `bingo-provider-openai`, `bingo`: Windows CI owns completion.
Each failed only in `aws-lc-sys v0.44.0`: `fatal error: 'stdlib.h' file not found` / `'windows.h' file not found`.
No Rust diagnostic, no toolchain installed; protocol step 7 explicitly permits this deferral.
ADR-0068 §5 accepted, §1–4 credited to M116; ADR-0035 §6 dated. No exit criterion remains unticked.
