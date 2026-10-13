# M127 — One HTTP stack

## Goal

Implement ADR-0072 §2 and audit-2026-10 §5: one bounded HTTP stack,
provider-owned decoding/refusals, and API-key lookup in `bingo-auth-oauth`.

## Bricks, in build order

1. Ban direct/implicit reqwest clients in clippy; record the red run.
2. Add library-tier `bingo-http`: `Bounds`, fallible `client`, `HttpError`,
   `Decode`, and ADR-0056's 20 s connect / 300 s idle / 64 KiB frame.
   `STREAMED` has no total timeout; `CALL` preserves OAuth/Feishu's 10/30 s.
   Dependencies: sdk, reqwest, bytes, http-body, futures, tokio, thiserror,
   already in the tree. The new member raises the dependency budget by one.
3. Diff and move `metered`/`sse`, with tests; test before deleting copies.
4. Move `stream`, generic over `Decode`; pin chunking, silence, cancellation,
   transport failure and unframed overflow. Keep provider fixture tests.
5. Move `round_trip(builder, idle)` and the loopback transport harness once;
   preserve slow upload, deaf/mute peer, and Content-Length assertions.
   Provider refusals and the subscription refresh retry stay provider-owned.
6. Build provider clients in registration; pass them through constructors.
   Route OAuth, MCP HTTP, Feishu, pictures, updates and web tools through
   `client`; propagate construction errors, preserve existing call bounds.
7. Move `ApiKey`, `Places`, `Configured` and tests into OAuth; delete twins.
8. Record implementation in ADR-0072 and the dated ADR-0056 §5 note.

## Files

- `crates/bingo-http/**`, workspace manifest/lock, clippy and budget config.
- Provider manifests, `src/*.rs` and integration constructor fixtures.
- OAuth manifest, `src/{lib,key,http}.rs` and existing test-client call sites.
- MCP manifest, `src/{auth,dial,manager,lib}.rs`; constructor-only fixtures
  in `src/{command,rows,manager,auth}.rs`, `tests/{server,signing_in}.rs`.
- Feishu manifest and `src/feishu/**`; pictures manifest and `src/load.rs`;
  web-tool manifest and `src/*.rs`; bin manifest and `src/{mcp,update}.rs`.
- TUI manifest and `src/run/updates.rs`: mechanical client migration only.
- ADR-0056/0072 and this plan.

## Re-verification (2026-10-13, `706a9b2d`)

Committed before code in `0a135995`. M130 moved Feishu into its platform
crate. M126 has not moved provider construction. OAuth/Feishu connect in
10 s, not the original plan's 20 s; `CALL` preserves their existing bounds.
The missed TUI update client needs a mechanical migration. Keys already
parameterize the provider. Workspace membership uses `crates/*`.
MCP/Feishu constructors take built clients; their callers change mechanically.
The duplicate scan matches definitions (`const IDLE_TIMEOUT`), not references.
Metered production differs only in comments; SSE also differs in test strings.
Request user agents stay unchanged; reqwest's default redirect limit is ten.

## Exit criteria

- [x] Clippy passes with the ban; initial failing output is recorded.
- [x] Duplicate transport/key definitions are absent from providers.
- [x] HTTP/OAuth and both provider suites pass; fixture streams are unchanged.
- [x] Budget is exactly +1; discipline, deny and all local gates pass.
- [ ] MSVC check passes for every touched crate — blocked by C headers; Windows CI.

## Non-goals

No retry, provider-error merge, MCP idle bound, HTTP server, reqwest version,
feature or TLS-backend change. No M123/M132 work, live smoke, push, merge or PR.

## Risks

- Real loopback I/O keeps the existing transport harness's pacing and deadline
  lower-bound assertions; owned stream/OAuth timers use paused time. If paused
  time cannot drive an attempted socket migration, require M111's exception;
  if M111 grants none, stop and ask.
- A changed ADR Decision or an out-of-scope gate fix stops under protocol §6.
- Constructor fixture changes are mechanical under protocol §6's M130 note.
- Missing Windows SDK C headers are left to Windows CI under protocol §7;
  any Rust compilation failure blocks.

## Verified (2026-10-13)

Logs: ignored `target/m127/`. Contracts `255a2dc9`, shared transport `492770c4`,
migration `b107b781`, test-client/lint fixes `b17bb437`. Fixture payloads and
assertions are unchanged. No live smoke ran. No terminal bytes changed.

Initial evidence (before implementation):
```text
$ diff -u crates/bingo-provider-{anthropic,openai}/src/metered.rs
-//! The same module sits in `bingo-provider-openai`: a plugin may not import
+//! The same module sits in `bingo-provider-anthropic`: a plugin may not import
$ diff -u crates/bingo-provider-{anthropic,openai}/src/sse.rs
-            frames(&[b"event: ping\ndata: {}\n\n"]),
+            frames(&[b"event: response.created\ndata: {}\n\n"]),
$ cargo clippy --workspace --all-targets --locked -- -D warnings
error: use of a disallowed method `reqwest::Client::builder`
error: use of a disallowed method `reqwest::Client::new`
    = note: ADR-0072: build a client with bingo_http::client
$ cargo test -p bingo-http
error[E0432]: unresolved imports `bingo_http::Bounds`, `bingo_http::CONNECT_TIMEOUT`, `bingo_http::FRAME`, `bingo_http::IDLE_TIMEOUT`, `bingo_http::client`
```
The first lint run reached OAuth/pictures before blocking dependents. The final
scan leaves only the allowed library builder; `reqwest::get` is also banned.

```text
$ cargo fmt --all -- --check
(exit 0; no output)
$ cargo check --workspace --all-targets --locked
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.18s
$ cargo clippy --workspace --all-targets --locked -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 14.34s
$ cargo test -p bingo-http -p bingo-auth-oauth -p bingo-provider-anthropic -p bingo-provider-openai --locked
(exit 0; includes the four moved loopback transport tests)
$ cargo test --workspace --locked --no-fail-fast -- --skip a_token_may_be_dialled_again_once_its_stream_has_closed
(exit 0)
$ cargo test -p bingo-provider-acp --test bridge --locked a_token_may_be_dialled_again_once_its_stream_has_closed -- --exact --test-threads=1
(exit 0)
$ python3 target/m127/results.py
targeted: 304 passed; 0 failed; 0 ignored
test: 5125 passed; 0 failed; 4 ignored
test-acp-alone: 1 passed; 0 failed; 0 ignored
test-pty-alone: 1 passed; 0 failed; 0 ignored
$ rg -l 'fn find_block_end|struct Metered|const IDLE_TIMEOUT|struct ApiKey' crates/bingo-provider-*
(no matches; exit 1)
$ scripts/check_discipline.sh
test host and wall-clock advisory checks complete
discipline ok
$ scripts/budget.sh
dependencies (unique, normal): 353 (max  353)
warm cargo check -p bingo-core: 0s (max  20s)
relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
target/debug: 5 GB (soft max  5)
test binaries: 109
budget ok
$ cargo deny check
advisories ok, bans ok, licenses ok, sources ok
$ cargo check -p bingo-http -p bingo-auth-oauth -p bingo-provider-anthropic -p bingo-provider-openai -p bingo-mcp -p bingo-channel-feishu -p bingo-pictures -p bingo-tool-web -p bingo-surface-tui -p bingo --all-targets --locked --target x86_64-pc-windows-msvc
error: failed to run custom build command for `aws-lc-sys v0.44.0`
error: failed to run custom build command for `tree-sitter v0.26.13`
error: failed to run custom build command for `tree-sitter-bash v0.25.1`
```
Windows: missing `stdlib.h`, `windows.h`, `stdio.h`; no Rust compile error.
All ten selected crates resolve `aws-lc-sys` (individual `cargo tree -p <crate>
-i aws-lc-sys --target x86_64-pc-windows-msvc --locked --depth 0` prints
`aws-lc-sys v0.44.0`); the bin also resolves both tree-sitter C crates.
These cross-checks remain for Windows CI under protocol §7; no toolchain installed.
The initial workspace run's rewind PTY timeout passed alone with
`cargo test -p bingo --test pty --locked esc_twice_rewinds_the_turn_and_the_file_it_wrote -- --exact --test-threads=1`,
and passed again in the final complete run. No out-of-scope fixes were needed.
