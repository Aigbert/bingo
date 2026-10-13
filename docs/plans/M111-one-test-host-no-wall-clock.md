# M111 — One test host, no wall clock

## Goal

Twenty-three files implement `HostApi` (`rg -l "impl HostApi for"
crates`); most are test fakes that each answer the same ten verbs their
own way, so a new verb or a changed default is a 20-file edit and the
fakes drift from the kernel's behaviour. `bingo_sdk::testing` already
has `NoHost` and `ServiceHost`. Grow it into one scriptable, recording
test host and move the plugin fakes onto it. Then remove the tests that
assert a machine's speed. This is the floor M119's typed services and
every later milestone's tests stand on.

## Bricks, in build order

1. **`bingo_sdk::testing::TestHost`** (behind the existing `testing`
   feature):
   - records every call (`calls() -> Vec<HostCall>`, an enum with one
     variant per verb and its arguments);
   - answers from scripted responses per verb (`on_open(|spec| ...)`,
     `on_deliver(...)`, …) and otherwise as `NoHost` does;
   - keeps an in-memory session table (`sessions`, `open`, `close`,
     `delete`) with `SessionSummary`s a test seeds, and a services map
     like `ServiceHost`;
   - `gateway_events()` returns a stream the test drives.
   Its own tests pin the defaults; the doc comment says which verbs are
   recorded and which have state.
2. **Migrate the plugin fakes,** one crate per commit, deleting the local
   impl: `bingo-agents`, `bingo-rooms`, `bingo-tasks`, `bingo-schedule`,
   `bingo-skills`, `bingo-checkpoints`, `bingo-permissions`,
   `bingo-tool-bash`, `bingo-context`, `bingo-demo-ui`, the channels'
   `host/tests/kernel.rs`, the TUI's `doubles.rs`, `bingo-surface-print`'s
   test host. A fake whose behaviour `TestHost` cannot express without a
   feature noun stays, with one line saying why.
3. **Leave the real ones.** `bingo-core/src/host.rs`, `host/unavailable.rs`,
   `session/commands.rs`, `bingo-surface-rpc/src/client.rs` are
   implementations, not fakes. `bingo-core/src/test_support*` and the
   RPC wire test host stay (they drive the real kernel); note in each
   why it is not `TestHost`.
4. **No test measures the machine.** Replace speed assertions with
   behaviour assertions:
   - `bingo-surface-tui/src/blocks.rs` (`warm < 200 ms`) and
     `highlight.rs` (`delta < 1 ms`): assert the cache was hit (a counter
     or `Arc::ptr_eq` on the cached value), not how long it took;
   - `view.rs`'s per-draw average, if it asserts a bound: same;
   - liveness checks of the form `started.elapsed() < N s, "it hung"`
     become `tokio::time::timeout(N, fut)` with the same message, so a
     slow machine fails only by hanging; list each in the commit body.
   - Under `tokio::time::pause()` where a test waits on a timer the code
     owns (chase, wakes, retry backoff), instead of real sleeps.
5. **The discipline check holds it.** `scripts/check_discipline.sh`
   warns on a new `impl HostApi for` outside the allowed files and on
   `elapsed() <` in a test assertion; warnings, not failures, so history
   is not rewritten (AGENTS.md, Records).

## Files

- `crates/bingo-sdk/src/testing.rs` (or `testing/` if it splits)
- the test modules named in brick 2 and their `Cargo.toml`
  dev-dependencies
- `crates/bingo-surface-tui/src/{blocks.rs,highlight.rs,view.rs}` tests
- tests with `elapsed() <` assertions across the workspace
- `scripts/check_discipline.sh`

## Exit criteria

- [ ] `rg -l "impl HostApi for" crates` lists only the real
      implementations, `bingo_sdk::testing`, and fakes each carrying a
      reason line (list pasted).
- [ ] `rg -n "elapsed\(\) ?<" crates --glob '**/test*'` and in
      `#[cfg(test)]` modules: none left that bounds speed (pasted).
- [ ] Test count does not drop: `cargo test --workspace` totals before
      and after pasted.
- [ ] All gates.

## Non-goals

- Typed services (M119); changing any `HostApi` verb or default.
- Rewriting the kernel's own test support or the PTY smoke.

## Risks

- A fake may encode behaviour a test relies on that `TestHost` answers
  differently; port the test's expectation, never loosen the assertion.
- Migrating 13 crates is wide; one commit per crate keeps each
  revertible. Run that crate's tests per commit, the full gates at the
  end.
- Re-verify at start: recount the impls and the wall-clock assertions.
