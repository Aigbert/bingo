# M131 — Every dispatch is a chain

## Goal

ADR-0074: seven dispatch points become chains of links with the kernel's
own behaviour as the last link, the `Hook` trait becomes an adapter whose
combination rules are written as tests, external links get a trust tier
the person grants in the user layer, and `HostApi::invoke` runs `AfterTool`.
No behaviour a person sees changes for anyone who installs no link: the
existing hook tests pass unedited. Audit §3, Mods gap 1.

## Bricks, in build order

1. **Contract and failing tests.** In `bingo-sdk`: `Interceptor` (one
   defaulted method per point, each `next.run(input)`), `Next<'_, I, O>`
   consumed by value, `LinkPoint` (the seven catalogue names of
   ADR-0074 §1, serialised as `tool.check` etc.; M136 makes it the
   catalogue), `LinkMatcher` (points + tool name,
   `hook_applies` semantics), `Tier { Tighten, Rewrite, Full }`,
   `Contribution::Interceptor` and `Registrar::external_interceptor(name,
   link)`. Tests that fail on the tree: a link at `tool.call` answers for
   the engine and the tool never runs; an invoked call is seen by an
   `AfterTool` hook (`session/invoke.rs` runs none today).
2. **The adapter's contract, pinned before it moves.** One test per rule
   of ADR-0074 §2, written against today's call sites so they pass now and
   must still pass after: submit rewrite order, submit
   `Deny`/`Block`/`Redirect` end the run, `Ask` at submit is `Continue`;
   `BeforeTool` first `Deny`/`Block`/`Ask` ends the run, `Redirect` is
   `Continue`, a hook `Ask` still lets the policy deny; `AfterTool` every
   matched hook sees every result, any `Block` ends after the round;
   `Stop` first `Block` adds a `hook:<id>` piece.
3. **`tightens(before, after) -> bool`**: the pure order Deny > Ask >
   Allow, and `admits(tier, point, act) -> bool` for what a tier may do at
   a point (ADR-0074 §3). Table tests, no I/O.
4. **The chain runner**: `chain::run(links, kernel_link, cx, input)` —
   matcher filter, tier check per link, the `HOOK` deadline on an external
   link, skip-with-notice on error, timeout or overstep. Generic over the
   point; one module.
5. **`tool.check` and `tool.call`.** `gate_call` becomes the kernel link
   (policy then person) behind plugin links; the `BeforeTool` adapter is
   `run_hooks` moved, not rewritten. `execute`/`execute_one` become the
   `tool.call` kernel link; `after_tool_hooks` becomes the `AfterTool`
   adapter. The turn's `gate`, `run_tools` and the bridged door
   (`Bridged::gated`, `serve`) all call the two chains (§5).
6. **`prompt.submit` and `turn.stop`.** `submit_prose`'s hook loop and `stop_hooks`
   become adapters over their chains; the actor's dispositions (reject,
   redirect, queue, start) are the `prompt.submit` kernel link.
7. **`turn.step`, `prompt.compose`, `session.compact`.** The provider stream
   in `turn/stream.rs`, `assemble_request` and `Turn::compact`'s call to
   the strategy become kernel links. Only in-process links exist until
   M132; the tier check already applies.
8. **`pluginTrust`.** A kernel key beside `enabledPlugins`,
   `{"<name>": {"tier", "uses"}}` (`uses` is read from M137 on), from the
   user layer only; a value in a project or CLI layer is ignored with a
   notice. `bingo plugins list` prints each external plugin's tier.

## Files

- `crates/bingo-sdk/src/{intercept.rs (new), hook.rs, plugin.rs, lib.rs}`
- `crates/bingo-core/src/chain/{mod.rs,submit.rs,tool.rs,model.rs,compose.rs,compact.rs,stop.rs}` (new; one point each)
- `crates/bingo-core/src/{gate.rs,executor.rs,turn.rs,plugins.rs}`
- `crates/bingo-core/src/turn/{late.rs,stream.rs}` (`HookSet` gathers links too)
- `crates/bingo-core/src/session/{inputs.rs,invoke.rs}`
- `crates/bingo-core/src/settings/merge.rs` (`pluginTrust`, user layer)
- `crates/bingo/src/plugins.rs` (the tier column)
- `crates/bingo-hooks-shell/src/guide.md` (the order rules, in a person's words)
- `docs/adr/0032-hooks-cross-the-bridge.md` (dated note)

## Exit criteria

- [ ] `cargo test -p bingo-core chain::` passes: a `full` link answers at
      `tool.call`, `turn.step` and `session.compact` without the kernel link
      running; a `rewrite` link's Allow over `next`'s Ask is skipped with a
      notice; a `tighten` link at `turn.step` observes only.
- [ ] `cargo test -p bingo-core hook_contract::` passes: one test per rule
      in ADR-0074 §2, and each was green before brick 5 (pasted).
- [ ] `cargo test -p bingo-core invoke::` shows an `AfterTool` hook seeing
      a call handed in through `HostApi::invoke`.
- [ ] A test interrupts a turn while a link at `tool.check` never returns:
      the turn ends `Interrupted` within the supervised-future bound (M116).
- [ ] `pluginTrust` in a project layer grants nothing: a settings test.
- [ ] `bingo plugins list` prints the tier: a black-box CLI test.
- [ ] Every existing hook test in core, hooks-shell, plugin-rpc,
      checkpoints, agents and rooms passes unedited
      (`cargo test --workspace --locked`); all gates of the protocol.

## Non-goals

- The wire for links (`intercept/*`) — M132. External plugins keep
  `hook/decide` until then; their tier is read but they only tighten.
- New hook points, or a chain at an observation point.
- Moving any first-party hook onto a link (checkpoints, rooms, agents keep
  their hooks; each moves when it wants a `next`).
- An `Allow` in `HookOutcome`.

## Risks

- `Next` borrowing the remaining links across an `.await` may fight the
  borrow checker in the turn, which holds `&mut self`; fall back to
  `Arc<[Arc<dyn Interceptor>]>` plus an index, not to cloning the turn.
- `prompt.compose` and the ruler interact: a link that grows the request
  after measurement would overflow. Compose runs before the ruler
  measures; say so in the trait's doc and test it.
- A `tool.call` answer for the engine must still journal a receipt naming
  the link, or a rewritten result would be indistinguishable in `/rewind`.
- Per-round cost on long turns: gather links once per round, as hooks are.
- Re-verify at start: M116 (ADR-0068) will have made the turn one
  supervised future and moved `racing` — the chain runs inside it, not
  beside it; M118 may have moved `TurnConfig`; M111's shared test host
  replaces the core `test_support` fakes these tests would have used;
  `gate.rs`, `turn.rs` and `session/invoke.rs` line references above are
  from `979a6d27`.
