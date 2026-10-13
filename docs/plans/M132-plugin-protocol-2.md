# M132 — Plugin protocol 2

## Goal

ADR-0075: the wire becomes protocol 2 and is frozen by a schema test; a
plugin process may ask the host to place a view, extend a record, deliver
input, keep state and call the session's model; links of ADR-0074 cross as
`intercept/<point>`; an unserved request is answered, not ignored; the
process is spawned as a tree that dies with it; and a `--plugin-dir`
plugin reloads when its files change. The acceptance test of Phase 5 is
M133's; this milestone makes every verb it needs answerable.

## Bricks, in build order

1. **Contract and failing tests.** `PROTOCOL = 2`; `schema/plugin.json`
   regenerated and copied to `schema/plugin-2.json`; a test that walks
   both and fails when a method, a field or a capability of 2 is removed
   or retyped (additions pass). Failing tests on the tree: a request the
   host does not serve gets `METHOD_NOT_FOUND` (today: no reply); a plugin
   that says `protocol: 1` is refused with both numbers in the notice.
2. **Capabilities.** `initialize` carries the host's `capabilities` and
   `plugin_name` beside `plugin_root` (for `$.plugin`, ADR-0078), and
   the answer the plugin's; `negotiated(host, plugin) -> BTreeSet<Cap>` is
   a pure brick with table tests. A verb neither side named is
   `METHOD_NOT_FOUND`; one only the host named is refused in words.
3. **The reverse table.** `served()` becomes `host.rs`: one handler per
   verb, keyed by method, each a translation and nothing more. `service/
   call` moves into it unchanged (ADR-0031 §4's tests pass unedited).
4. **`host/place`, `host/extend`, `host/deliver`** onto `HostApi::signal`
   (the slot placement of ADR-0070, keyed by the plugin's name), `extend`
   and `deliver` with origin `plugin:<name>`. One test each through the
   stub plugin.
5. **`host/state/*`.** Pure `swap(held: Option<Version>, if_version:
   Option<Version>) -> Result<Version, Conflict>` first, with tests. Then
   the shelf of M129 under `<data_dir>/plugins/<name>/state/{global,
   <session>}`, written once per `set`; values of a session go on
   `GatewayEvent::SessionRemoved`. A conflict answers the held version.
6. **`HostApi::complete`.** In `bingo-sdk`, defaulted to
   `Unsupported`; in `bingo-core`, one request on the session's resolved
   model with its provider, its `Usage` added to the running turn's bill
   (`TurnCompleted.usage` sums it), or with no turn running charged to the
   session by a `UsageCharged` without a turn (ADR-0075 §2, revised). No
   context reading changes. `host/model/complete` calls it. A test: a
   stub plugin's completion shows in the turn's usage.
7. **`tools/changed`.** Replaces the plugin's held tool list (and
   `commands/changed` its commands); `PluginTools` already reads the
   manager per round, so no kernel change.
8. **Links over the wire.** `intercept.rs`: one `Interceptor` per
   declared point, two-phase per ADR-0075 §3, `intercept/cancel` on drop,
   the `HOOK` deadline per phase. `Registrar::external_interceptor` (M131)
   becomes `Contribution::Interceptors(Arc<dyn InterceptorSource>)` read
   per round like `HookSource`, each link named by its process; M131's
   tier tests move onto a one-link source. `hook/decide` and its handler
   go; `hook.rs` keeps observation only.
9. **The process tree.** `Connection::spawn` uses `bingo-process` and
   framing uses `bingo-jsonrpc` (M128); `codec.rs` goes if M128 has not
   already removed it. A test: a plugin whose entry starts a grandchild
   leaves no process behind after the host drops it (unix and Windows).
10. **Hot reload.** `--plugin-dir` writes `pluginDirs` in `cli_layer`;
    discovery adds them as the last layer. `reload.rs`: a pure
    `changed(before, after: &Stamps) -> bool` over (path, len, mtime),
    then a poll once a second on tokio time (paused in tests, M111), then stop
    → spawn → handshake → re-read → notice. A changed `providers` or
    `services` declaration is reported as needing a restart.

## Files

- `schema/plugin.json`, `schema/plugin-2.json` (new), `schema/rpc.json`
- `crates/bingo-plugin-rpc/src/{wire.rs,connection.rs,bridge.rs,hook.rs,source.rs,manager.rs,discovery.rs,config.rs,manifest.rs,schema.rs,testing.rs,lib.rs}`
- `crates/bingo-plugin-rpc/src/{host.rs,state.rs,intercept.rs,reload.rs}` (new; one noun each)
- `crates/bingo-plugin-rpc/examples/stub_plugin/{main.rs,hooks.rs}`, `tests/plugin/*.rs`
- `crates/bingo-sdk/src/{host.rs (complete),intercept.rs (InterceptorSource),plugin.rs}`
- `crates/bingo-core/src/{host.rs,plugins.rs}`, `crates/bingo-core/src/chain/mod.rs`
- `crates/bingo/src/main.rs` (`--plugin-dir`), `crates/bingo/src/plugins.rs` (who asks for `model`)
- `examples/plugins/wordcount/main.py` (protocol 2), `crates/bingo/tests/plugin_rpc.rs`
- `docs/adr/{0015,0030,0031}-*.md` (dated notes)

## Exit criteria

- [ ] `cargo test -p bingo-plugin-rpc schema::` passes, and fails when a
      field is deleted from `plugin-2.json`'s shape (shown once, pasted).
- [ ] `cargo test -p bingo-plugin-rpc --test plugin` covers every verb of
      ADR-0075 §2 through the stub plugin, `METHOD_NOT_FOUND`, the
      protocol-1 refusal and a state conflict.
- [ ] An intercept test: a stub link at `tool.call` with `after` sees the
      result; esc during its first phase sends `intercept/cancel` and the
      turn ends `Interrupted` (M116 bound).
- [ ] `cargo test -p bingo-core complete::`: usage of a side call is in
      `TurnCompleted.usage`; outside a turn it is the session's charge.
- [ ] A grandchild-leak test passes on unix and in CI's `windows` job.
- [ ] A reload test: touching the stub's manifest re-handshakes, keeps a
      `global` state value and posts one notice; no sleep in the test.
- [ ] `cargo test -p bingo --test plugin_rpc` passes with `wordcount` on 2.
- [ ] All gates of the protocol, and `cargo check -p bingo-plugin-rpc
      --all-targets --target x86_64-pc-windows-msvc`.

## Non-goals

- The TypeScript SDK, `bingo plugins validate|test`, `ts-status` — M133.
- A file watcher dependency; watching discovered (non-`--plugin-dir`)
  plugins; reloading a provider or a service in place.
- Rewinding plugin state with `/rewind`; plugin state in the journal.
- Speaking protocol 1 alongside 2.

## Risks

- Billing a side call, decided 2026-10-13: an additive
  `Event::UsageCharged { turn, plugin, usage }`, fixture first (M113).
  `TurnUsage` is not reused, because its fold bumps `round` and replaces
  `context`. The fold adds the charge to the live turn's and the
  summary's usage and leaves both alone. A test pins that
  `TurnCompleted.usage` equals the sum of the turn's `TurnUsage` and
  `UsageCharged` frames; keep one tally to make it hold.
- `host/deliver` from inside a link of the same session's turn can queue
  input behind the turn that is waiting on the link: deliver never waits
  for the delivered turn; say so in the guide and test it.
- A reload while a link holds a chain: the link fails in words and the
  chain skips it (ADR-0074 §4). The test pins this, not a race.
- Re-verify at start: ADR-0070's slot placement as M121/M123 built it
  (what `host/place` carries); M128's `bingo-process`/`bingo-jsonrpc`
  APIs and whether `codec.rs` survived; M129's shelf path and API;
  ADR-0068/M116 supervised turns (the link runs inside the turn's
  future); M131's `Interceptor`/`Next` as built; M111's shared test host
  and clock for the reload poll. Line numbers are from `979a6d27`.
