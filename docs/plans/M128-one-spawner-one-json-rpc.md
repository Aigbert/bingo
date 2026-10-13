# M128 — One process spawner, one JSON-RPC stack

## Goal

ADR-0072 §3–4. Every child bingo starts becomes a `bingo_process::Tree`: a process group on unix, a job
object on Windows, killed on drop. That includes MCP stdio servers and RPC plugins, which have no tree
today. The line codec and the pending map, copied today in surface-rpc, plugin-rpc and ACP, become one
crate, `bingo-jsonrpc`, and every reader is bounded. `bingo-process` already exists with `shell` only
(M110, ADR-0063); this milestone adds the spawner. The two halves are independent, so stop between them
if the run is long.

## Bricks, in build order

1. **The tree contract, and a failing test.** `bingo_process::tree(tokio::process::Command) ->
   Result<Tree, SpawnError>` wraps `ProcessGroup::leader` (unix) or `JobObject` (Windows), both gated in
   this change, plus `KillOnDrop`. `Tree` has `take_stdin/stdout/stderr`, `id`, `wait`, `start_kill`, and
   `terminate(grace)` (TERM then kill on unix; kill on Windows). `Drop` kills unless the child is reaped
   (tool-bash's `Group` rule). The test: a shell from `shell::resolve()` runs `sleep 60 & wait`, and the
   grandchild inherits stdout. Dropping the tree must bring stdout to EOF, which happens only once every
   writer is dead. It needs no signal and no pid probe, and it is the same on every platform. Run it
   first against a plain `kill_on_drop` command and paste the hang, bounded by a generous deadline
   (M111).
2. **tool-bash moves onto it.** `run.rs` `spawn` and `Group` go, and `supervise.rs`'s TERM-then-GRACE
   becomes `terminate(GRACE)`. DRAIN, SCAN, PACE and CAP stay; they are the tool's.
3. **hooks-shell moves onto it.** `run.rs` `spawn` goes; DRAIN and `read_capped` stay.
4. **The ACP child moves onto it.** `child.rs` `wrapped` goes; stderr is still discarded.
5. **MCP stdio servers become trees.** `dial.rs` `spawn_child` spawns a `Tree` with stderr to
   `stderr_sink`, and hands rmcp `(stdout, stdin)` through `transport-async-rw`. The service holds the
   tree. Drop `transport-child-process` from `bingo-mcp`; the bin's `acp_bridge` test may keep it as a
   dev feature. Add brick 1's test through `spawn_child`.
6. **RPC plugins become trees.** `connection.rs` `spawn` builds a `Tree` with `current_dir(root)` and the
   stderr log, and `kill_on_drop(true)` goes. Add brick 1's test through `Connection::spawn`.
7. **The codec contract.** Create `crates/bingo-jsonrpc`, `tier = "library"`. Its dependencies (`serde`,
   `serde_json`, `tokio`, `tokio-util` codec, `futures`, `thiserror`) are already in tree, so
   `budget.toml` grows by one. Lift `bingo-surface-rpc/src/codec.rs` with its tests: `Version`, `Id`,
   `Request`, `Notification`, `Response`, `Outcome`, `Message`, the four standard codes, and `lines(max)`
   with `MAX_LINE` (16 MiB). Write the golden lines first. An over-long line is
   `CodecError::LineTooLong`, never a hang. Codes in the `-32000` range stay with their protocols.
8. **The peer contract.** Lift `Peer` from the pending map in surface-rpc's `client.rs`. It owns ids,
   `request`, `notify`, and a handler for inbound requests and notifications. Tests come first: responses
   out of order; a dropped request removes its entry; a closed peer fails each pending request with
   `Closed`; an over-long line ends the peer.
9. **surface-rpc moves onto both.** `server.rs` maps `LineTooLong` where it maps `MaxLineLengthExceeded`
   today, with the same words, and `RemoteKernel` runs on `Peer`. M107's budgets stay. The black-box rpc
   tests pass unchanged.
10. **plugin-rpc moves onto both.** Its `codec.rs` copy goes, along with the "deliberate" comment, and
    the reader becomes bounded. `tool/cancel` and `provider/cancel` stay the explicit notifications they
    are today.
11. **ACP moves onto `Peer`.** `connection.rs` loses its reader and writer tasks, `Pending`, its
    `AtomicI64` ids and both `.lines()` calls. The schema crate stays the vocabulary, with `RequestId`
    converted at the edge. The fake-agent suites pass unchanged, and a 17 MiB line from the agent ends
    the session with the named error.
12. **Records.** ADR-0015 gets a dated note: its codec copy ended (ADR-0072).

## Files

- `crates/bingo-process/**`, `crates/bingo-jsonrpc/**` (new), `Cargo.toml`, `scripts/budget.toml`
- `crates/bingo-tool-bash/src/{run,supervise}.rs`, `crates/bingo-hooks-shell/src/run.rs`
- `crates/bingo-provider-acp/src/{child,connection,wire}.rs`, `crates/bingo-mcp/{Cargo.toml,src/dial.rs}`
- `crates/bingo-plugin-rpc/src/{codec,connection}.rs`,
  `crates/bingo-surface-rpc/src/{codec,server,client}.rs`
- `docs/adr/{0015,0072}-*.md`, `Cargo.lock`, and each touched crate's `Cargo.toml`
- Consumer module declarations and protocol error conversions; new tests beside each changed module;
  `surface-rpc/src/server/discovery.rs` error conversions.
- Mechanical fixture update: `surface-rpc/tests/wire/main.rs` uses shared `Lines` and its explicit bound;
  plugin `examples/stub_plugin/{main,hooks}.rs` uses the shared `Id` type. Assertions and wire values are
  unchanged.
- Mechanical Tree fixtures: `bingo/tests/{support/{mod,process},rpc,channel_claim,
  channels,cli/prefix}.rs`, `provider-acp/src/bin/fake_agent/mcp.rs`, and `bingo/Cargo.toml` (dev-only
  library edge).

## Exit criteria

- [x] `rg -n 'ProcessGroup|JobObject|kill_on_drop' crates --type rust` matches only
      `crates/bingo-process`.
- [x] The grandchild test passes for tool-bash, MCP and plugin-rpc on macOS, and CI's Windows job runs it
      as a blocking test.
- [x] `cargo tree --workspace -e normal | rg 'process-wrap v9'` finds nothing.
- [x] `rg -n '\.lines\(\)'` finds no wire reader in plugin-rpc or ACP, and `rg -n 'oneshot::Sender'` no
      pending map in the three crates.
- [x] `cargo test -p bingo-jsonrpc -p bingo-process -p bingo-surface-rpc -p bingo-plugin-rpc -p
      bingo-provider-acp -p bingo-mcp` passes.
- [x] `cargo test -p bingo --test rpc --test plugin_rpc --test acp_asked` passes, and `acp_bridge` passes
      when run alone.
- [x] `scripts/budget.sh` passes (+1, `bingo-jsonrpc`), `cargo deny check` and all host gates pass.
- [ ] Windows checks for every touched crate: six pass; three await Windows CI because
      `aws-lc-sys` cannot find Windows SDK C headers on this Mac (protocol step 7).

## Non-goals

- The gateway's detached start (`start.rs`, `ProcessSession`). MCP on `bingo-jsonrpc` (rmcp stays). Call
  deadlines (M117). Plugin protocol 2 (M132). Logging ACP's stderr.

## Risks

- rmcp 3.1's `transport-async-rw` must accept the `(AsyncRead, AsyncWrite)` pair. If it does not, stop
  and ask, and do not keep process-wrap 9.
- A bounded reader may refuse a line that works today. Measure the largest plugin-rpc and ACP fixture
  before settling on 16 MiB.
- On Windows, `npx.cmd` shims under a job object are a risk. Command resolution does not change, only the
  wrap.
- Re-verified at `ff21d03d`: M110 exports `shell::resolve`; rmcp 3.1 implements `IntoTransport` for
  `(AsyncRead, AsyncWrite)`.
- Preserve M117's `Link::expire/stop`, adapter/prompt ownership and `Connection::mark_dead`; no
  `session.rs` edit is needed. Preserve every deadline test and plugin `callTimeout` / MCP `toolTimeout`
  behavior.
- Surface codec conversions of `KernelError` must become protocol-local functions when shared types make
  their old `From` impls orphan impls.
- Harness/fake-agent constructors also move mechanically to Tree to keep the original repository-wide
  scan. M129 already accepted ADR-0072; retain its history and add M128's implementation note.

## Verified (2026-10-13)

All commands ran in this worktree, one gate at a time. M117 deadline tests and `Link` ownership are unchanged.
- Red: `cargo test -p bingo-process dropping_a_tree_closes_a_grandchilds_stdout -- --nocapture`: `the grandchild must close stdout when its tree drops: Elapsed(())`; `0 passed; 1 failed; finished in 3.03s` against plain child kill-on-drop.
- Red: `cargo test -p bingo-plugin-rpc --lib dropping_a_connection_closes_a_grandchilds_stdout`: `0 passed; 1 failed; finished in 3.03s` before Tree migration.
- Contracts committed first: `cargo test -p bingo-jsonrpc` failed with `E0432: could not find codec`; the next peer-contract run failed on absent `Handler, Peer, PeerError, Sender`. Implemented: `10 passed; 0 failed`.
- Temporary measurement examples (`cargo run -p <crate> --example m128_measure --locked`, removed afterward): `ACP largest fixture: 727 bytes (request_permission_codex)`; `Plugin picture fixture envelope: 5594496 bytes`. Both fit 16 MiB.
- `cargo test -p bingo-jsonrpc -p bingo-process -p bingo-surface-rpc -p bingo-plugin-rpc -p bingo-provider-acp -p bingo-mcp`: summed `589 passed; 0 failed; 1 ignored` (existing 100 MiB stress test). Includes grandchild EOF and named 17 MiB ACP rejection tests.
- `INSTA_UPDATE=no cargo test --workspace --locked -- --test-threads=1 --skip a_proxy_with_a_token_this_run_never_minted_gets_nothing --skip a_token_may_be_dialled_again_once_its_stream_has_closed`: summed `5085 passed; 0 failed; 4 ignored` (existing manual/stress tests); all `16` PTY tests passed first run.
- `cargo test -p bingo --test acp_bridge --locked a_proxy_with_a_token_this_run_never_minted_gets_nothing -- --exact --test-threads=1`: `1 passed; 0 failed`.
- `cargo test -p bingo-provider-acp --test bridge --locked a_token_may_be_dialled_again_once_its_stream_has_closed -- --exact --test-threads=1`: `1 passed; 0 failed`. Both isolated tests excluded above were run, not silently skipped.
- `cargo test -p bingo --test rpc --test plugin_rpc --test acp_asked --test channel_claim --test channels --locked`: summed `37 passed; 0 failed`. Fixture assertions are unchanged.
- `cargo fmt --all -- --check`: exit 0, no output.
- `cargo check --workspace --all-targets --locked`: `Finished dev ... in 1m 12s`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: `Finished dev ... in 57.18s`. Fixture module duplication/import warnings were corrected before this final pass.
- `scripts/check_discipline.sh`: `discipline ok` (advisory warnings).
- `scripts/budget.sh`: `dependencies (unique, normal): 349 (max 349)`; warm core `0s`; relink isolation `0`; `budget ok`. `target/debug: 9 GB` exceeds only the 5 GB soft limit.
- `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.
- `rg -n 'ProcessGroup|JobObject|kill_on_drop' crates --type rust`: only `bingo-process/src/tree.rs:8,10,23,25`.
- `cargo tree --workspace -e normal | rg 'process-wrap v9'`: no matches. `rg -n 'oneshot::Sender'` in the three clients: no matches; remaining ACP `.lines()` calls are scripted-agent/test readers or text rendering, not the production wire.
- Each of the nine touched crates ran `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc --locked`, sequentially:
  `bingo-process`, `bingo-jsonrpc`, `bingo-tool-bash`, `bingo-hooks-shell`, `bingo-provider-acp`, `bingo-surface-rpc`: `Finished dev` (exit 0).
  `bingo-plugin-rpc`, `bingo-mcp`, `bingo`: `failed to run custom build command for aws-lc-sys v0.44.0`; `fatal error: 'stdlib.h' file not found` / `'windows.h' file not found`. No Rust errors; left to Windows CI under protocol step 7, with no toolchain installed.
- `.github/workflows/ci.yml:61–72` retains blocking Windows workspace tests; the new grandchild tests have no Windows ignore. No TUI-byte changes required a separate smoke.
- The cache wrapper delayed exit after the isolated ACP test, then exited successfully before an attempted stop; no process was signalled. Final Cargo gates used the installed toolchain directly.
- ADR-0072 records M128's §3–4 implementation alongside M129's acceptance; ADR-0015 records the ended codec copy. No stop condition, push, merge or PR. Only the three Windows C-header checks remain unticked.
