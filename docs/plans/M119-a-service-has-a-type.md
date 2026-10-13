# M119 — A service has a type

## Goal

Implement ADR-0069 §1–4: typed service lookup and registration, shared team/MCP
contracts in `bingo-services`, optional manifest `uses`, and removal of the
unread `sdk` field. No kernel door or new external dependency.

## Bricks, in build order

1. **Contracts first.** SDK tests register a trait object under a typed key,
   reject a different type under the same name, and find a computed page key.
   A compile-fail doctest pins lookup's result type. See failing evidence below.
2. **Typed keys.** `ServiceKey<T: ?Sized>` has `const fn new`, `named(String)`
   and `name()`. `HostHandle::service`, `ToolContext::service` and
   `Contribution::service` take typed keys; the erased value holds `Arc<T>`.
   `Pages::key` returns `ServiceKey<Pages>`. Delete the string lookup;
   `service_any`, `service_wire` and `open_service` keep their wire contract.
3. **Registry declarations.** Refuse a contributed service absent from its
   owner's `provides`, naming plugin and key. Delete redundant owner-side
   provides assertions; retain their independent configuration/behavior checks.
4. **Shared contracts.** Add `bingo-services`, library tier, depending on
   `bingo-sdk`, `async-trait`, `serde`, `serde_json` (all already in the tree).
   Each module owns its key, trait, both adapters, golden wire fixture and
   `find(host)` (typed first, wire fallback):
   - `team::TeamFile::section(&Path, &str) -> Result<Value, TeamError>`.
   - `mcp::Servers::servers() -> Map<String, Value>`.
5. **Owners.** Agents' team-file service and MCP's live rows implement the
   shared traits and register both faces through the adapters.
6. **Consumers.** Rooms uses `team::find`, losing private key/method strings,
   its JSON round trip and `TeamError::Unnameable`. ACP uses `mcp::find`,
   losing `ROWS`/`SERVERS`. Test each with its real owner at the binary layer.
7. **Optional use.** `PluginManifest.uses: &'static [&'static str]` names raw
   service keys: rooms uses `team::KEY.name()`, ACP uses `mcp::KEY.name()`;
   others use `&[]`. Missing optional services leave `standing()` unchanged;
   `bingo plugins` names missing used services, with black-box coverage.
8. **Delete sdk.** Remove the unused field, all manifest initializers and its
   claim of boot-time validation. The re-verified count is 39 files.
9. **Discipline 4e.** Discover keys from `ServiceKey::new` in the shared
   modules; importing crates must declare them in provides/requires/uses.
   Seed an undeclared consumer, observe failure, restore and observe success.
10. **Records.** List the library in ARCHITECTURE.md; accept ADR-0069 §1–4
    and annotate amended ADRs. §§5–6 remain proposed for M126.

## Files

- SDK: `src/{service,host,tool,plugin,page,lib,testing}.rs`, `testing/tests.rs`.
- Core: `src/host/registry.rs`, `src/host/tests/services.rs`.
- New `crates/bingo-services/{Cargo.toml,src/{lib,team,mcp}.rs,fixtures/}`.
- Agents: `src/{lib,team,team/service,team/seat}.rs`; MCP: `src/{lib,rows}.rs`.
- Rooms: `src/{lib,team,tests,hook}.rs`; ACP: `src/{lib,crossing}.rs`.
- Those four crates' Cargo.toml; workspace Cargo.toml and Cargo.lock.
- Every PluginManifest initializer; every page-registering `src/guide.rs`;
  skills `src/{pages,tests}.rs`; mechanical page-key assertions in owner libs.
- Plugin-RPC: `src/service.rs` fixture; `tests/plugin/{service,doors,hooks}.rs`.
- Binary: `src/plugins.rs`, `tests/cli/{plugins,rooms,acp/bridge}.rs`.
- `scripts/{budget.toml,check_discipline.sh}`, ARCHITECTURE.md,
  `docs/adr/{0001,0031,0054,0057,0069}-*.md`.

## Exit criteria

- [x] `cargo test -p bingo-sdk service` passes typed-key contracts.
- [x] `rg 'service::<' crates --glob '!**/tests/**'` finds no string lookup.
- [x] `rg '"agents\.team"|"mcp\.servers"' crates` finds one spelling each,
  both in the shared modules; `rg 'sdk: "' crates` finds nothing.
- [x] Registry refuses undeclared services; missing `uses` leaves plugins up.
- [x] CLI black-box test names the missing service; real-owner tests pass.
- [x] Discipline rejects a seeded violation and passes the restored tree.
- [x] Budget (+1 member, 347 → 348), dependency audit and all local gates pass.
- [x] Windows checks pass for services, rooms, ACP and 16 other touched crates.
- [ ] Eleven C-dependent Windows builds are left to Windows CI under protocol §7;
  the exact crates and failing C dependencies are recorded below.

## Non-goals

RoomPost/collaboration (M122), typed surface options/settings (M126), external
manifest `uses` (M132), and changes to service wire answers.

## Risks and re-verification

- Re-verified against `7265cfec`, committed before code as `baf9385c`.
  M111's ServiceHost needs the same typed storage; rooms' scripted fixture
  needs its wire face. M130 does not change the 39-file manifest count.
- Protocol §6 permits mechanical fixtures outside the original Files: page
  assertions, SDK/skills shared-host fixtures, hook key names and re-exports.
  Real-owner tests live in the binary, avoiding plugin-to-plugin dependencies.
- The extra Arc hop is not per token. External owners use wire fallback,
  covered by shared golden tests plus the existing plugin-RPC process tests.
- The declaration check remains textual; unusual import forms can evade it.
- A non-Unicode fixture originally assumed the filesystem could create such
  a name. It now passes the native path to an existing ancestor's file.

## Verified (2026-10-13)

Contracts were committed before implementation; failing output:
```text
$ cargo test -p bingo-sdk service
error[E0433]: cannot find type `ServiceKey` in this scope
$ cargo test -p bingo-core an_undeclared_service_is_refused_with_its_plugin_and_key
an undeclared service must be refused; test result: FAILED. 0 passed; 1 failed
$ cargo test -p bingo-services
test result: FAILED. 0 passed; 4 failed (adapter scaffolds: not implemented)
$ cargo test -p bingo-core a_missing_used_service
error[E0560]: PluginManifest has no field named `uses`
```
Targeted green output: SDK service 6 passed; registry services 3 passed;
shared service contracts 4 passed; standing 1 passed; three new CLI tests
(missing use, real team owner, real MCP owner) each `1 passed; 0 failed`.
```text
$ cargo fmt --all -- --check
exit 0; no output
$ cargo check --workspace --all-targets --locked
Finished `dev` profile ... in 12.78s
$ cargo clippy --workspace --all-targets --locked -- -D warnings
Finished `dev` profile ... in 12.92s
$ cargo test --workspace --locked -- --skip a_token_may_be_dialled_again_once_its_stream_has_closed --skip a_proxy_with_a_token_this_run_never_minted_gets_nothing
PTY picture test: FAILED (a terminator); 313 passed before the stop
$ cargo test -p bingo --test pty --locked a_picture_an_answer_named_in_markdown_is_read_in_and_sent -- --exact --test-threads=1
1 passed; 0 failed
$ cargo test --workspace --exclude bingo --locked -- --skip a_token_may_be_dialled_again_once_its_stream_has_closed
4682 passed; 0 failed; 3 ignored (summed result lines, including doctests)
$ cargo test -p bingo --test rooms --test rpc --test thinking --test views --locked
22 passed; 0 failed (summed result lines)
$ cargo test -p bingo-provider-acp --test bridge --locked a_token_may_be_dialled_again_once_its_stream_has_closed -- --exact --test-threads=1
1 passed; 0 failed
$ cargo test -p bingo --test acp_bridge --locked a_proxy_with_a_token_this_run_never_minted_gets_nothing -- --exact --test-threads=1
1 passed; 0 failed
```
Total: 5020 passed, 3 existing ignored; PTY flake passed alone, unchanged.
The native-path fixture correction also passed alone before the final rerun.
```text
$ scripts/check_discipline.sh # seeded by removing rooms' uses, then restored
bingo-rooms: agents.team is used but absent from provides, requires and uses
restored: shared service declarations ok; discipline ok
$ scripts/budget.sh
dependencies: 348 (max 348); warm core: 1s; relink: 0 crates; budget ok
soft warning: target/debug 6 GB (soft max 5)
$ cargo deny check
advisories ok, bans ok, licenses ok, sources ok
$ cargo check -p <each touched crate> --all-targets --target x86_64-pc-windows-msvc --locked
19 PASS; 11 WINDOWS_C_HEADERS; no Rust compile errors; no toolchain installed
```
Windows CI deferrals: `bingo`, channels, mcp, plugin-rpc, provider-anthropic,
provider-openai, surface-print, surface-tui, tool-fs, tool-web: `aws-lc-sys`;
permissions: `tree-sitter` and `tree-sitter-bash` (all shorthand names use
`bingo-`). All failures name missing Windows SDK C headers, per protocol §7.
No TUI-byte change; workspace PTY coverage ran, with the one isolated rerun.
