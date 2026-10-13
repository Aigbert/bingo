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
6. **`prompt.submit` and `turn.stop`.** `session/gate.rs::submit_hooks`'s loop and `stop_hooks`
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

- `crates/bingo-sdk/src/{intercept.rs (new), intercept/stream.rs (new), hook.rs, plugin.rs, lib.rs}`
- `crates/bingo-core/src/chain/{mod.rs,submit.rs,tool.rs,model.rs,compose.rs,compact.rs,stop.rs}` (new; one point each)
- `crates/bingo-core/src/{gate.rs,executor.rs,turn.rs,plugins.rs}`
- `crates/bingo-core/src/turn/{late.rs,stream.rs}` (`HookSet` gathers links too)
- `crates/bingo-core/src/session/{gate.rs,inputs.rs,invoke.rs,mailbox.rs,running.rs}`
- `crates/bingo-core/src/{lib.rs,host.rs,settings.rs}` and `host/registry.rs`
- `crates/bingo-core/src/{chain,session/tests}/` (new contract tests)
- `crates/bingo-core/src/turn/tests.rs` (contract test modules, mechanical HookSet field)
- `crates/bingo-sdk/tests/` (chain shape and receipt fixtures)
- `crates/bingo/tests/` (headless tier listing)
- `crates/bingo-core/src/settings/merge.rs` (`pluginTrust`, user layer)
- `crates/bingo/src/plugins.rs` (the tier column)
- `schema/plugin.json` (mechanical HookMatcher description regeneration only)
- `crates/bingo-plugin-rpc/src/deadline.rs` (alias the existing HOOK bound to sdk)
- `crates/bingo-hooks-shell/src/guide.md` (the order rules, in a person's words)
- `docs/adr/0032-hooks-cross-the-bridge.md` (dated note)

## Exit criteria

- [x] `cargo test -p bingo-core chain::` passes: a `full` link answers at
      `tool.call`, `turn.step` and `session.compact` without the kernel link
      running; a `rewrite` link's Allow over `next`'s Ask is skipped with a
      notice; a `tighten` link at `turn.step` observes only.
- [x] `cargo test -p bingo-core hook_contract::` passes: one test per rule
      in ADR-0074 §2, and each was green before brick 5 (pasted).
- [x] `cargo test -p bingo-core invoke::` shows an `AfterTool` hook seeing
      a call handed in through `HostApi::invoke`.
- [x] A test interrupts a turn while a link at `tool.check` never returns:
      the turn ends `Interrupted` within the supervised-future bound (M116).
- [x] `pluginTrust` in a project layer grants nothing: a settings test.
- [x] `bingo plugins list` prints the tier: a black-box CLI test.
- [x] Every existing hook test in core, hooks-shell, plugin-rpc,
      checkpoints, agents and rooms passes unedited
      (`cargo test --workspace --locked`); all gates of the protocol.

## Non-goals

- The wire for links (`intercept/*`) — M132. External plugins keep
  `hook/decide` until then; their tier is read but they only tighten.
- New hook points, or a chain at an observation point.
- Moving any first-party hook onto a link (checkpoints, rooms, agents keep
  their hooks; each moves when it wants a `next`).
- An `Allow` in `HookOutcome`.

## Re-verification (2026-10-13, `3a0f80d2`)

- M116 moved submit hooks into the owned `session/gate.rs` future; keep
  `validate` and `may_run_shell` there, and retain `INTERRUPTED`/TaskTracker.
- `HookSet` remains in `turn/late.rs`; `TurnConfig` is in `turn/config.rs`.
  Registry/host wiring and settings types are required alongside merge.
- M121 stamps quiet on opened calls; preserve it and permission actions.
- Receipts can use existing durable notice items; no new Event is needed.
- Settings layers currently carry only a source label; mark the user layer
  at load time, never infer trust from a project path or its position.
- Mechanical HookSet/Layer construction updates outside Files, if needed,
  follow protocol step 6; existing hook assertions remain unchanged.
- Bridged AfterTool blocks share one per-turn stop flag through Running and
  TurnMail; completion occurs at the round boundary, not by cancellation.

## Risks

- Borrowed continuations must remain inside the supervised future; never clone a turn.
- Compose before measuring: the ruler must see every link's request growth (tested).
- A rewritten or substituted tool result needs a durable receipt naming its link.
- Gather links once per round; retain a consumed continuation on failure, never rerun a tool.

## Verified (2026-10-13)

Contract first: `cargo test -p bingo-core session::tests::invoke --locked` → `FAILED. 5 passed; 2 failed`: `left: Some("echo:8")`, `right: Some("answered by the link")`; `left: []`, `right: ["bridged-7"]`.
Trust first: `cargo test -p bingo-core plugin_trust --locked` → `error[E0609]: no field plugin_trust on type KernelSettings`; implementation then passed.

| Command (run separately) | Output, trimmed |
| --- | --- |
| `cargo test -p bingo-core hook_contract:: --locked` before brick 5 and after | Both: `test result: ok. 12 passed; 0 failed` |
| `cargo test -p bingo-core chain:: --locked` | `test result: ok. 10 passed; 0 failed` |
| `cargo test -p bingo-core invoke:: --locked` | `test result: ok. 8 passed; 0 failed` |
| `cargo test -p bingo-core --locked` | `test result: ok. 462 passed; 0 failed`; `interrupt_drops_a_hung_tool_check_link ... ok`; `plugin_trust_comes_only_from_an_explicit_user_layer ... ok` |
| `cargo test -p bingo --test cli an_external_plugins_tier_is_listed_from_user_settings_only --locked` | `test result: ok. 1 passed; 0 failed` |
| `cargo fmt --all -- --check` | Exit 0, no output |
| `cargo check --workspace --all-targets --locked` | `Finished dev profile` (1m 08s) |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | `Finished dev profile` (43.29s) |
| `INSTA_UPDATE=no cargo test --workspace --locked -- --test-threads=1 --skip a_proxy_with_a_token_this_run_never_minted_gets_nothing --skip a_token_may_be_dialled_again_once_its_stream_has_closed` | Exit 0; summed result lines: 5,117 passed, 0 failed, 4 existing manual/stress ignores; PTY: `16 passed; 0 failed` |
| `INSTA_UPDATE=no cargo test -p bingo --test acp_bridge --locked a_proxy_with_a_token_this_run_never_minted_gets_nothing -- --exact --test-threads=1` | `test result: ok. 1 passed; 0 failed` |
| `INSTA_UPDATE=no cargo test -p bingo-provider-acp --test bridge --locked a_token_may_be_dialled_again_once_its_stream_has_closed -- --exact --test-threads=1` | `test result: ok. 1 passed; 0 failed` |
| `scripts/check_discipline.sh`; `scripts/budget.sh`; `cargo deny check` | `discipline ok`; `budget ok` (349 dependencies, warm core 0s, isolation 0); `advisories ok, bans ok, licenses ok, sources ok` |
| `cargo check -p <crate> --all-targets --locked --target x86_64-pc-windows-msvc`, each of `bingo-sdk`, `bingo-core`, `bingo-hooks-shell` | Each: `Finished dev profile` |
| Same Windows command, each of `bingo-plugin-rpc`, `bingo` | `aws-lc-sys v0.44.0`: `'windows.h' file not found`, `'stdlib.h' file not found`; Windows CI deferred per protocol §7; no Rust errors |
| `scripts/tui-smoke.sh` (temporary copy, absolute ROOT and unique tmux socket only) | All 18 scenarios; `tui-smoke ok`; temporary copy removed |

The first workspace run found only stale HookMatcher schema prose; mechanical regeneration changed one description and its schema test passed. Existing hook assertions stayed unedited. No new dependency, Event variant or record version. All exits met, with the explicit Windows C-header deferral above.
