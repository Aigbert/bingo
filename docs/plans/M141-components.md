# M141 — Components

## Goal

ADR-0079 §4: the surface's own drawings become components a `ui.render`
hook may answer — a tool row, a message, the spinner, the band above the
prompt. The host publishes which components some loaded hook matches;
the TUI asks only about those, draws its own drawing until the answer
arrives, and draws an `Engine` node as its own drawing with the props
the hook gave it. The permission prompt is never a component. `$.ui.log`
finishes the pulled half of `$.ui`, and `'bingo'` gains its state
helpers (ADR-0078 §8). The proof is three examples of bingo's own,
`tool-call`, `band` and `pane`, which do what Claude Code's authoring
examples of those names do and draw in the TUI. After M138 and M140; see the re-verification below.

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

- `$.ui.copy`, `scroll`, `focus`, `selection` and `open`'s `isPlaced`:
  gaps in design §8, since a surface's scroll, focus, selection,
  clipboard and placement are its own.
- A desktop drawing components (ADR-0080); print and channels asking
  (`text` folds); vscode and mobile.
- `bingo plugins test`, `$.ui.mount` and the examples' tests (M142);
  `prompt.edit`.

## Risks

- **Props carry what `own` needs.** A component's drawing reads only its
  props, or a rewritten prop changes nothing. Where today's drawing
  reads more — local expansion, the width cut, the theme — that moves
  into the props (`isExpanded`, `bodyColumns`) or the `Render` context
  (M125), never into a second copy of the item.
- **Height changes on arrival.** A row's own drawing is replaced by its
  answer once per (request, generation); if the drive shows a jump, the
  first ask happens before the row is first painted.
- **Cost under `ToolUse`.** Every visible row asks; the four-in-flight
  bound and the memo keep scrolling cheap, as the drive measures.

## Re-verified (2026-10-14, ba70fdf6)

- Landed prerequisites: M137 `c1d95e8c`, M138 `5581b978`, M139 `686ad289`,
  M140 `ba70fdf6`; the TUI baseline includes M147 `6e3b1b03`, M148
  `fddb62f3`, M149 `b2bee6f4`, M150 `a2f77d96`. No new dependency.
- `render::Component` has fifteen variants including `Question` (ADR-0079's
  dated amendment); `InteractionKind::Permission` must never project one.
- `LinkMatcher` already has `prefilter`; sdk `catalogue::prefilter` is
  conservative, while both runtimes share TypeScript `on.ts::matches`.
  Mod metadata currently exports patterns only; publish serializable hook
  partials, widening unsupported predicates rather than dropping matches.
- `session.rs::refresh_config` writes config; catalogue completion handles
  the reload boundary. M140's `answer_ok`, `Shown::Pull`, `pulled::Memo` and
  core render queue are present. Memo currently drops off-screen requests;
  M141 must retain component answers for scrolling and revise block caches.
- `transcript::called` combines the tool heading/result; no ToolGroup
  drawing exists yet. `status::middle` owns PromptHint; `worked` supplies
  transcript closing rows. Ember/crew and removed rail/sheet remain intact.
- Examples use `Edit.file_path` and `Bash.command`. User/explicit mods are
  rewrite, project mods tighten; only user-layer trust grants full (§5).
- M142 owns kit/mount/type emission and example `*.test.ts`; M151 owns
  pane placement/viewport geometry. Shared render/status edits stay scoped.
- Protocol 2 is unreleased (latest tag v0.6.5): scoped schema amendments
  are authorized with one dated ADR-0075 line and generated with
  `BINGO_UPDATE_SCHEMA=1`. Mechanical exhaustive arms outside Files are
  listed in Verified. Discipline 4c stays unchanged; sdk/core name no tool.
