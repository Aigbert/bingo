# M127 — One HTTP stack

## Goal

ADR-0072 §2. The two providers' transport, about 1.4k lines kept twice,
becomes one crate, `bingo-http`. After this, every `reqwest::Client`
bingo builds has a connect bound, and none falls back to
`Client::new()`, which panics where the builder failed. The twin
`key.rs` moves to `bingo-auth-oauth`.

## Bricks, in build order

1. **The rule first.** Add `disallowed-methods` to `clippy.toml` for
   `reqwest::Client::new` and `reqwest::Client::builder`, with the
   reason `ADR-0072: build a client with bingo_http::client`. Paste the
   run: both `http()`, M117's `bingo_auth_oauth::client()` (which the
   codex token source, MCP's `auth.rs` and the bin's `mcp.rs` call),
   Feishu `Api::new`, pictures `load.rs`, the bin's `update.rs`,
   tool-web, and auth-oauth's test clients. This is the failing test; it stays red until brick 6.
2. **The crate and its contract.** Create `crates/bingo-http`,
   `tier = "library"`. It depends on `bingo-sdk`, `reqwest`, `bytes`,
   `http-body`, `futures`, `tokio` and `thiserror`, all already in tree.
   Add the member and a `budget.toml` comment in M67's style (+1). The
   contract:
   - `Bounds { connect, call: Option<Duration> }`, with `STREAMED`
     (20 s, none) and `CALL` (10 s, 30 s, Feishu/OAuth call bounds today);
   - `client(Bounds) -> Result<Client, HttpError>`, the one
     `#[allow(clippy::disallowed_methods)]`;
   - `trait Decode { fn decode(&mut self, event, data) ->
     Result<Vec<ModelEvent>, ProviderError> }`;
   - `CONNECT_TIMEOUT`, `IDLE_TIMEOUT` and `FRAME`, with a test that
     pins ADR-0056's numbers.
3. **`metered` and `sse` move.** `diff` the two copies and paste the
   result: production differs only in comments; SSE test payloads differ. Move anthropic's copy with
   its tests and run `cargo test -p bingo-http`. Point both providers at
   the crate, then delete both copies.
4. **`stream` moves, generic over `D: Decode`.** The library tests the
   five behaviours the providers test today with a trivial line decoder:
   chunked equals whole, quiet times out, cancel stops, a transport
   failure ends, a body that never frames is an error. Each
   `events::Decoder` implements `Decode`. Fixture tests stay in each
   provider and call `bingo_http::stream::model_stream`.
5. **`round_trip` moves, and the transport harness lands once.**
   `round_trip(builder, idle) -> Result<(Response, Clock),
   ProviderError>` is the shared half of `send`. Each provider's
   `refusal` keeps classifying a non-success status, because their
   `error.rs` files differ. Move the harness and its tests into
   `crates/bingo-http/tests/transport.rs` and delete both inline copies:
   - helpers: `peer`, `head`, `declared_length`, `a_trickle`,
     `a_deaf_peer`, `a_mute_peer`;
   - tests: `a_slow_upload_is_not_a_silence`,
     `a_peer_that_stops_reading_ends_the_request`,
     `a_server_that_accepts_and_says_nothing_times_out`.

   Preserve the loopback socket harness; use paused time where I/O permits.
6. **Every client goes through `client`.**
   - The provider constructors (`keyed`, `subscription`) take a client
     built in `register`, where a failed build can refuse the provider
     with the builder's reason. `http()` goes.
   - M117's `bingo_auth_oauth::client()` is deleted. Its three callers
     (the codex token source, MCP's auth handles, the bin's MCP login)
     take `bingo_http::client(Bounds::CALL)`; M117's numbers are
     `CALL`'s.
   - MCP's streamable-HTTP client (`dial.rs`) takes `Bounds::STREAMED`.
     A silent server is caught by M117's tool-call deadline.
   - Feishu, pictures, update and tool-web keep their numbers and lose
     `unwrap_or_else(|_| Client::new())`.
   - Tests in auth-oauth take a dev-dependency on `bingo-http`.

   Brick 1's clippy run is now clean.
7. **`key.rs` moves to `bingo_auth_oauth::key`.** `ApiKey`, `Places` and
   `Configured` move with their tests. The provider name is already a
   parameter; preserve it and delete both copies.
8. **Records.** Add a dated note to ADR-0056 §5 pointing at ADR-0072.

## Files

- `crates/bingo-http/**` (new), `Cargo.toml`, `scripts/budget.toml`,
  `clippy.toml`
- `crates/bingo-provider-{anthropic,openai}/{Cargo.toml,src/*.rs}`
- `crates/bingo-auth-oauth/src/{lib,key}.rs` and its test clients (`src/http.rs` keeps deadline tests)
- `crates/bingo-mcp/src/{auth,dial,manager,lib}.rs`, `crates/bingo/src/{mcp,update,main}.rs`
- `crates/bingo-channel-feishu/src/feishu/**`,
  `crates/bingo-pictures/src/{load,lib}.rs`, `crates/bingo-tool-web/src/*.rs`
- TUI `src/run/updates.rs` and its manifest: mechanical bounded-client migration
- Consumer manifests, provider integration tests and MCP constructor-only fixtures
  (`src/{command,rows,manager,auth}.rs`, `tests/{server,signing_in}.rs`);
  `docs/adr/{0056,0072}-*.md`

## Re-verification (2026-10-13, `706a9b2d`)

Audit §5 and ADR-0072 §2 still hold. M126 has not moved construction:
`register` calls `instances::providers`; keyed/subscription callers are in
those modules and provider tests. M130 moved Feishu into its platform crate.
M117 OAuth and Feishu connect in 10 s, not the plan's stale 20 s; `CALL`
preserves 10/30 s, while streamed providers preserve ADR-0056's 20/300 s.
The missed TUI update client needs only a mechanical edit, coordinated by
scope with M123. `metered` differs only in comments; SSE additionally uses
different test strings. Key production is identical except comments and
already parameterizes the provider. Workspace membership uses `crates/*`.
Keep user agents on requests and reqwest's existing ten-redirect default.
No provider fixture payload or assertion changes; no live-network tests run.
The duplicate-definition scan matches `const IDLE_TIMEOUT`; references to
the shared constant are expected. MCP manager/Feishu adapter constructors
take the built client, with mechanical caller updates listed above.

## Exit criteria

- [ ] `cargo clippy --workspace --all-targets --locked -- -D warnings`
      is clean with `disallowed-methods` in place (paste brick 1's run
      beside it).
- [ ] `rg -l 'fn find_block_end|struct Metered|const IDLE_TIMEOUT|struct ApiKey'
      crates/bingo-provider-*` finds nothing.
- [ ] `cargo test -p bingo-http -p bingo-auth-oauth` passes, including
      the moved transport tests.
- [ ] `cargo test -p bingo-provider-anthropic -p bingo-provider-openai`
      passes, with unchanged fixture streams.
- [ ] `scripts/budget.sh` passes, with the count raised by exactly one.
      `cargo deny check` and `scripts/check_discipline.sh` pass.
- [ ] `cargo check -p bingo-http -p bingo-provider-anthropic
      -p bingo-provider-openai --all-targets --target x86_64-pc-windows-msvc`
      passes. All gates pass.

## Non-goals

- Changing ADR-0056 §1–4's numbers; adding a retry.
- Merging the providers' `error.rs` or `refusal`.
- An MCP HTTP idle bound. M117's tool-call deadline covers a silent
  server; other await deadlines are M117's too.
- An HTTP server. `bingo-loopback` is that.
- Changing the reqwest version, its features or its TLS backend.

## Risks

- The transport tests use real sockets and wall-clock gaps (20 ms
  gaps, a 500 ms idle). If M111's paused clock cannot drive real I/O,
  they need M111's named exception. If M111 grants none, stop and ask.
- A client built in `register` changes public constructors. Re-verify
  every caller of `keyed` and `subscription` first.
- Re-verify at start: that the copies are still identical; every symbol
  named here; M117's `bingo_auth_oauth::client()`; whether M126 moved
  provider construction.
