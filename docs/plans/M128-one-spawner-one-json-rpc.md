# M128 — One process spawner, one JSON-RPC stack

## Goal

ADR-0072 §3–4. Every child bingo starts becomes a
`bingo_process::Tree`: a process group on unix, a job object on
Windows, killed on drop. That includes MCP stdio servers and RPC
plugins, which have no tree today. The line codec and the pending map,
copied today in surface-rpc, plugin-rpc and ACP, become one crate,
`bingo-jsonrpc`, and every reader is bounded. `bingo-process` already
exists with `shell` only (M110, ADR-0063); this milestone adds the
spawner. The two halves are independent, so stop between them if the
run is long.

## Bricks, in build order

1. **The tree contract, and a failing test.**
   `bingo_process::tree(tokio::process::Command) -> Result<Tree,
   SpawnError>` wraps `ProcessGroup::leader` (unix) or `JobObject`
   (Windows), both gated in this change, plus `KillOnDrop`. `Tree` has
   `take_stdin/stdout/stderr`, `id`, `wait`, `start_kill`, and
   `terminate(grace)` (TERM then kill on unix; kill on Windows). `Drop`
   kills unless the child is reaped (tool-bash's `Group` rule). The test: a shell from `shell::resolve()` runs `sleep 60 & wait`,
   and the grandchild inherits stdout. Dropping the tree must bring
   stdout to EOF, which happens only once every writer is dead. It
   needs no signal and no pid probe, and it is the same on every
   platform. Run it first against a plain `kill_on_drop` command and
   paste the hang, bounded by a generous deadline (M111).
2. **tool-bash moves onto it.** `run.rs` `spawn` and `Group` go, and
   `supervise.rs`'s TERM-then-GRACE becomes `terminate(GRACE)`. DRAIN,
   SCAN, PACE and CAP stay; they are the tool's.
3. **hooks-shell moves onto it.** `run.rs` `spawn` goes; DRAIN and
   `read_capped` stay.
4. **The ACP child moves onto it.** `child.rs` `wrapped` goes; stderr is
   still discarded.
5. **MCP stdio servers become trees.** `dial.rs` `spawn_child` spawns a
   `Tree` with stderr to `stderr_sink`, and hands rmcp `(stdout, stdin)`
   through `transport-async-rw`. The service holds the tree. Drop
   `transport-child-process` from `bingo-mcp`; the bin's `acp_bridge`
   test may keep it as a dev feature. Add brick 1's test through
   `spawn_child`.
6. **RPC plugins become trees.** `connection.rs` `spawn` builds a `Tree`
   with `current_dir(root)` and the stderr log, and
   `kill_on_drop(true)` goes. Add brick 1's test through
   `Connection::spawn`.
7. **The codec contract.** Create `crates/bingo-jsonrpc`,
   `tier = "library"`. Its dependencies (`serde`, `serde_json`, `tokio`,
   `tokio-util` codec, `futures`, `thiserror`) are already in tree, so
   `budget.toml` grows by one. Lift `bingo-surface-rpc/src/codec.rs`
   with its tests: `Version`, `Id`, `Request`, `Notification`,
   `Response`, `Outcome`, `Message`, the four standard codes, and
   `lines(max)` with `MAX_LINE` (16 MiB). Write the golden lines first.
   An over-long line is `CodecError::LineTooLong`, never a hang. Codes
   in the `-32000` range stay with their protocols.
8. **The peer contract.** Lift `Peer` from the pending map in
   surface-rpc's `client.rs`. It owns ids, `request`, `notify`, and a
   handler for inbound requests and notifications. Tests come first:
   responses out of order; a dropped request removes its entry; a
   closed peer fails each pending request with `Closed`; an over-long
   line ends the peer.
9. **surface-rpc moves onto both.** `server.rs` maps `LineTooLong`
   where it maps `MaxLineLengthExceeded` today, with the same words, and
   `RemoteKernel` runs on `Peer`. M107's budgets stay. The black-box
   rpc tests pass unchanged.
10. **plugin-rpc moves onto both.** Its `codec.rs` copy goes, along with
    the "deliberate" comment, and the reader becomes bounded.
    `tool/cancel` and `provider/cancel` stay the explicit notifications
    they are today.
11. **ACP moves onto `Peer`.** `connection.rs` loses its reader and
    writer tasks, `Pending`, its `AtomicI64` ids and both `.lines()`
    calls. The schema crate stays the vocabulary, with `RequestId`
    converted at the edge. The fake-agent suites pass unchanged, and a
    17 MiB line from the agent ends the session with the named error.
12. **Records.** ADR-0015 gets a dated note: its codec copy ended (ADR-0072).

## Files

- `crates/bingo-process/**`, `crates/bingo-jsonrpc/**` (new), `Cargo.toml`,
  `scripts/budget.toml`
- `crates/bingo-tool-bash/src/{run,supervise}.rs`, `crates/bingo-hooks-shell/src/run.rs`
- `crates/bingo-provider-acp/src/{child,connection}.rs`,
  `crates/bingo-mcp/{Cargo.toml,src/dial.rs}`
- `crates/bingo-plugin-rpc/src/{codec,connection}.rs`,
  `crates/bingo-surface-rpc/src/{codec,server,client}.rs`
- `docs/adr/0015-*.md`, and each touched crate's `Cargo.toml`

## Exit criteria

- [ ] `rg -n 'ProcessGroup|JobObject|kill_on_drop' crates --type rust`
      matches only `crates/bingo-process`.
- [ ] The grandchild test passes for tool-bash, MCP and plugin-rpc on
      macOS, and CI's Windows job runs it as a blocking test.
- [ ] `cargo tree --workspace -e normal | rg 'process-wrap v9'` finds
      nothing.
- [ ] `rg -n '\.lines\(\)'` finds no wire reader in plugin-rpc or ACP,
      and `rg -n 'oneshot::Sender'` no pending map in the three crates.
- [ ] `cargo test -p bingo-jsonrpc -p bingo-process -p bingo-surface-rpc
      -p bingo-plugin-rpc -p bingo-provider-acp -p bingo-mcp` passes.
- [ ] `cargo test -p bingo --test rpc --test plugin_rpc --test acp_asked`
      passes, and `acp_bridge` passes when run alone.
- [ ] `scripts/budget.sh` passes (+1, `bingo-jsonrpc`), and so does
      `cargo deny check`. The Windows `cargo check` passes for every
      touched crate, and all gates pass.

## Non-goals

- The gateway's detached start (`start.rs`, `ProcessSession`). MCP on
  `bingo-jsonrpc` (rmcp stays). Call deadlines (M117). Plugin protocol 2
  (M132). Logging ACP's stderr.

## Risks

- rmcp 3.1's `transport-async-rw` must accept the `(AsyncRead,
  AsyncWrite)` pair. If it does not, stop and ask, and do not keep
  process-wrap 9.
- A bounded reader may refuse a line that works today. Measure the
  largest plugin-rpc and ACP fixture before settling on 16 MiB.
- On Windows, `npx.cmd` shims under a job object are a risk. Command
  resolution does not change, only the wrap.
- Re-verify at start: M110's API, M117's deadlines here, every line named.
