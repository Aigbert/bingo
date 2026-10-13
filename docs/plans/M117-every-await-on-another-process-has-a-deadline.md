# M117 — Every await on another process has a deadline

## Goal

ADR-0068 §5. After M116 the person's esc ends any turn within
`INTERRUPT_GRACE`. A session nobody is watching gets no esc: a
scheduled run, a channel bot, a subagent. For those, each plugin that
waits on another process or server bounds the wait itself, with a named
constant and, where a person may need longer, a setting. Expiry is a
tool error the model reads, never a hang.

## Bricks, in build order

1. **Failing tests first** (paused clock where the crate's dev tokio has
   `test-util`; add it to dev-dependencies where it does not; no new
   dependency):
   - MCP: a stdio fixture server (the crate's existing test server, or
     a `tool/call` that never answers) → `call` returns
     `ToolError::Failed` naming the server and the timeout after
     `TOOL_CALL` (10 min virtual); with `toolTimeout: 5` after 5 s.
   - plugin-rpc: the crate's test plugin (`testing.rs`) with a tool and
     a command that never answer → a tool error / a command error after
     `deadline::TOOL_CALL` / `deadline::COMMAND_RUN`; with the manifest's
     `callTimeout` after that. The tool's `abandons` guard still sends
     its abandon notice on expiry.
   - ACP: the fixture agent that ignores `session/cancel` → after
     `CANCEL_GRACE` (10 s) the adapter is dropped (process tree killed;
     assert via the fixture's exit) and the next prompt spawns a fresh
     child; an agent that answers the cancel within the grace keeps its
     child.
   - HTTP: a local `TcpListener` that accepts and never writes → the
     codex token refresh and the MCP auth discovery fail with a timeout
     error after the total bound, not hang.
2. **plugin-rpc.** `deadline.rs` gains `TOOL_CALL` and `COMMAND_RUN`
   (10 min each), each with the doc-comment reasoning the file already
   keeps for every constant, and the ordering test extended
   (`SERVICE < COMMAND_RUN`). `Manifest` gains an optional
   `callTimeout` (seconds, both verbs; schema and fixture updated;
   absent keeps the constant). `tool.rs` races the `select!` loop
   against the deadline; `command.rs` wraps `request` in
   `tokio::time::timeout`. The error says which plugin, which verb and
   how long.
3. **MCP.** `tool.rs` `call` adds a deadline arm to its `select!`:
   `TOOL_CALL = 10 min` beside `dial.rs`'s `CONNECT_TIMEOUT`, overridden
   by a per-server `toolTimeout` (seconds) on both `Server` transports.
   `config.rs` parses it with the same unknown-field strictness; the
   settings schema snapshot and the guide page (`guide.md`) say what it
   is. A `0` is refused at startup ("never" is not a setting).
4. **ACP.** `provider.rs`: after `session/cancel` is sent, the wait for
   the agent's answer gets `CANCEL_GRACE = 10 s`. Past it the link is
   marked dead and its `Adapter` dropped, so `child.rs`'s `Drop` kills
   the tree. The next prompt takes the existing fresh-child path. The
   wait runs on a task the link owns, not on the turn, because M116 may
   already have dropped the turn's future at 2 s. Dated note on
   ADR-0035 §6 ("the child is not killed" → "not killed while it
   answers the cancel within 10 s").
5. **HTTP clients.** `bingo-auth-oauth` (a library) gains
   `pub fn client() -> reqwest::Client` with `connect_timeout(10 s)` and
   `timeout(30 s)`. These are token and discovery requests, small by
   nature. The three production `reqwest::Client::new()` sites that
   reach it use that function: `bingo-provider-openai/src/lib.rs`
   (`subscription`'s `TokenSource`), `bingo-mcp/src/auth.rs` (`handles`)
   and `crates/bingo/src/mcp.rs`. Test-module constructions stay.
   Provider stream clients keep ADR-0056's connect and idle bounds; a
   total bound would cut a long answer.
6. **The kernel's side.** None. A deadline is a tool error through the
   existing `ToolError::Failed`, a command error through
   `KernelError`.
7. ADR-0068 §5 accepted, with a dated note that §1–4 landed in M116.
   Add ADR-0035 §6 to ADR-0068's Amends line.

## Files

- `crates/bingo-plugin-rpc/src/{deadline,manifest,tool,command,
  schema}.rs`, its fixtures and `tests/plugin/**`
- `crates/bingo-mcp/src/{tool,config,dial,auth}.rs`, `guide.md`, tests
- `crates/bingo-provider-acp/src/{provider,connection,child}.rs`,
  fixtures
- `crates/bingo-auth-oauth/src/lib.rs` (or a new `http.rs`),
  `crates/bingo-provider-openai/src/lib.rs`, `crates/bingo/src/mcp.rs`
- the settings schema snapshot; `docs/adr/{0068,0035}*.md`

## Exit criteria

- [ ] Each test in brick 1 fails before and passes after (pasted).
- [ ] `rg -n 'reqwest::Client::new\(\)' crates --glob '!**/tests/**'`
      shows only `#[cfg(test)]` modules (pasted with the line ranges).
- [ ] `rg -n '\.request\(' crates/bingo-plugin-rpc/src` — every hit is
      inside a `timeout` or a `select!` with a deadline arm (pasted,
      one line of reasoning per hit).
- [ ] A real MCP server configured with `toolTimeout: 1` and a tool
      that sleeps longer: the model sees the timeout error (notes
      pasted; skip with the reason if no such server is at hand).
- [ ] All gates, Windows target check included (the ACP kill path is
      `process-wrap`'s on both).

## Non-goals

- One HTTP stack for the workspace (M127); this only bounds the three
  unbounded clients.
- A deadline on provider streams beyond ADR-0056's idle bound.
- A kernel-wide tool timeout. The kernel cannot know how long a tool
  should take; the plugin that knows the other end does.

## Risks

- A 10 min default cuts a genuinely long MCP tool (a build, a crawl).
  The setting exists for that. The error text names the setting, so the
  person can find it from the transcript.
- Killing an ACP agent that was about to answer loses its session-side
  state. 10 s is five times `codex-acp`'s own stdin-close grace. The
  fresh child resumes with `session/load` where the agent supports it
  (existing path).
- Re-verify at start: M116 has landed; the `select!` in MCP `call`,
  `Abandons` in plugin-rpc `tool.rs`, the cancel arm in ACP
  `provider.rs`.
