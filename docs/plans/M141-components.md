# M141 — Components

## Goal

ADR-0079 §4: matched `ui.render` hooks answer the surface's own drawings.
Typed props feed Engine substitution; unmatched sites keep their drawing.
Permission prompts stay private. Add `$.ui.log`, shared state helpers and
original tool-call/band/pane examples, proved over RPC and in the TUI.
After M138 and M140; landed dependencies are recorded below.

## Bricks, in build order

1. **Contracts first.** Sdk `component.rs`: fifteen typed props using the
   reference's `RenderPropsOf` names, golden JSON per component, generated
   RPC/TypeScript schemas. Missing source fields take resting values.
   Props are projections, never stored alongside session items.
2. **Matched components.** Publish `{ component: [partial…] }` under
   `ConfigView.kernel.render` at load/reload/removal. Sdk `render::matched`
   and `Matched::asks` share the catalogue matcher; regex/functions widen
   to their component alone. Table tests for nested/scalar/opaque matchers.
3. **Engine substitution.** Pure TUI `component::draw(component, props,
   answer, own)` draws `own(props)` until answered. `views::render_with`
   resolves nested Engine nodes with parsed rewritten props; invalid props
   fall back to the originals with one notice. Stub-own table tests.
4. **Draw sites.** Messages, tool use/result/group/progress, command output,
   notices, spinner, duration, mode, prompt hint, AbovePrompt, Pane and
   Question ask only when matched and visible. Request ids are item ids,
   tool-use ids, or singleton component names. Rewritten props feed own
   drawing; memo generations revise block caches and survive scrolling.
   TestBackend coverage per component; unmatched sites make zero requests.
   A turn-completion notice and duration remain two rows/two requests in order.
5. **Permission exclusion.** Question projection refuses Permission;
   even an unrestricted Question hook causes zero render requests for it.
6. **`$.ui.log`.** One op event writing to bingo's existing tracing log,
   attributed to `mod::<plugin>`.
7. **State helpers.** Shared `state.ts`: `atom`, per-render memoized `derive`,
   `read`, `update` (three ifVersion conflict retries, then throw), `memberOf`.
   QuickJS embeds the same source; M142's kit consumes it.
8. **Original examples.** `examples/plugins/{tool-call,band,pane}/` each has
   plugin.json, module source, types/index.d.ts (PluginState) and README.
   Tool-call denies a protected edit, rewrites a shell command and counts in
   status; band answers AbovePrompt with a hide Button; pane opens from a
   command and shows one row per call. Use helpers and component/requestId
   matchers, read `hasSurvey`; copy no unlicensed example source.
9. **Acceptance.** `mods_components.rs` drives all three over stdio RPC with
   fake provider: denial/rewrite/count, band show/hide, pane rows/reload.
   Add all three to tui-smoke; isolated tmux pane open/two calls/escape drive.

## Files

- `crates/bingo-sdk/src/{component.rs (new),component/,render.rs,lib.rs,intercept.rs,catalogue/matcher.rs}`
- `crates/bingo-core/src/{chain/render.rs,session.rs,session/catalogue.rs}` (config publication)
- `crates/bingo-mods/src/` `$.ui.log`, the embedded helpers; `sdk/typescript/src/{state.ts (new),index.ts}`
- `crates/bingo-surface-tui/src/{component.rs (new),views/mod.rs,transcript.rs,transcript/{said,output}.rs,
  activity.rs,worked.rs,status.rs,form.rs,dialog.rs,blocks.rs,pane.rs,pulled.rs,view.rs,run/pulling.rs,lib.rs}`
- `crates/bingo-{surface-rpc,plugin-rpc}/src/schema.rs`; `schema/{rpc,events,plugin,plugin-2}.json`; generated TypeScript
- `sdk/typescript/src/{on.ts,plugin.ts}` (shared matcher metadata); `docs/adr/0075-plugin-protocol-2.md`
- `examples/plugins/{tool-call,band,pane}/` (new)
- `crates/bingo/tests/mods_components.rs` (new); `scripts/tui-smoke.sh`

## Exit criteria

- [ ] `cargo test -p bingo-sdk component::` and `cargo test -p bingo-sdk render::matched` pass: each
      component's golden JSON, and `asks` over scalar, nested and
      RegExp-only matchers.
- [ ] `cargo test -p bingo-surface-tui component::` passes: `draw` with
      no answer, with a `Box` wrapping `Engine`, and with props that do
      not parse; one `TestBackend` test per component; a run that matches
      nothing makes zero `ui/render` requests (a counting host double).
- [ ] The permission test (brick 5) and the `turn.complete` test (brick
      4) pass; their names are pasted.
- [ ] `cargo test -p bingo --test mods_components` passes (brick 9).
- [ ] `scripts/tui-smoke.sh` passes with all three examples; a tmux drive
      of `pane` (open, call two tools, escape) is pasted.
- [ ] Every gate, plus `cargo check -p bingo-surface-tui --all-targets
      --target x86_64-pc-windows-msvc`.

## Non-goals

Clipboard/scroll/focus/selection APIs, isPlaced, desktop/print/channel component
pulls, prompt.edit, M142's kit/mount/example tests and M151's pane placement.

## Risks

Props must drive their own drawing; memo generations must account for changed
height without scroll jumps. Only visible rows ask, through the bounded queue.

2026-10-14 delegate decision: M141 owns additive band Action hit-testing in `pointer.rs` and unclaimed hotkeys in `input.rs`; composer keeps focus (no new Focus variant), and M151 owns pane placement and hit-testing.

## Re-verified (2026-10-14, ba70fdf6)

- Landed prerequisites: M137 `c1d95e8c`, M138 `5581b978`, M139 `686ad289`,
  M140 `ba70fdf6`; the TUI baseline includes M147 `6e3b1b03`, M148
  `fddb62f3`, M149 `b2bee6f4`, M150 `a2f77d96`. No new dependency.
- `Component::Question` is the landed name; Permission is excluded.
- Sdk `catalogue::prefilter` is conservative; both runtimes share `on.ts`.
  M141 adds per-hook render metadata without duplicating dispatch links.
- Config comes from `session.rs`; catalogue completion is the reload boundary.
  M140 provides `answer_ok`, Shown/Pull and the bounded core render queue.
  Its memo evicts off-screen answers; M141 retains them and revises blocks.
- Tool headings/results share `transcript::called`; no ToolGroup drawing
  exists. PromptHint is `status::middle`; closing rows use `worked`.
- Examples use `Edit.file_path`/`Bash.command`. User/explicit mods are rewrite,
  project mods tighten; only user-layer trust grants full (ADR-0078 §5).
- M142 owns kit/mount/type emission and example tests; M151 owns placement.
- Protocol 2 is unreleased (v0.6.5): scoped amendments are authorized with a
  dated ADR-0075 note and `BINGO_UPDATE_SCHEMA=1`. Discipline 4c is unchanged.

## Verified (2026-10-14, in progress)

- `cargo test -p bingo-surface-rpc component_props_are_published_for_wire_clients --locked`: expected FAIL, missing QuestionProps, before schema registration.
- `cargo test -p bingo-sdk component:: --locked`: PASS, 2 tests.
- `cargo test -p bingo-sdk render::matched --locked`: expected FAIL before selection; PASS, 1 test after implementation.
- `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-surface-rpc --locked`: PASS, 16 unit + 41 wire tests; 1 existing ignored test.
- `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc wire_ts:: --locked`: PASS, 1 test.
- `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc schema:: --locked`: PASS, 10 tests.
- `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc --lib --locked`: PASS, 130 tests.
- `cargo test -p bingo-core chain::render --locked`: PASS, 1 test (before publication test was added).
- `cargo test -p bingo-surface-tui component:: --locked`: expected FAIL, 2 tests before substitution; PASS, 2 tests after implementation, before band contract.
- `cargo test -p bingo-surface-tui above_prompt_button_dispatches_its_handler_from_the_drawn_band --locked`: FAIL, 0 passed/1 failed; button drawn, effects: [].
- `cargo fmt --all`: PASS.
- `cargo fmt --all -- --check`: PENDING.
- `cargo check --workspace --all-targets --locked`: PENDING.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: PENDING.
- `cargo test --workspace --locked`: PENDING.
- `scripts/check_discipline.sh`: PENDING.
- `scripts/budget.sh`: PENDING.
- `cargo deny check`: PENDING.
- `cargo check -p bingo-sdk -p bingo-core -p bingo-mods -p bingo-surface-rpc -p bingo-plugin-rpc -p bingo-surface-tui -p bingo --all-targets --target x86_64-pc-windows-msvc`: PENDING.
- `scripts/tui-smoke.sh`: PENDING; no live/credential run.
- `git diff --name-only ba70fdf6 -- crates/bingo/tests/catalogue.rs`: mechanical LinkMatcher constructor only (render: None); no new exhaustive variant arms.
- `git diff --name-only ba70fdf6 -- crates/bingo-surface-tui/src/{pointer,input}.rs`: authorized additive band routes; pane routing unchanged.
