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
examples of those names do and draw in the TUI. After M138 and M140.

## Bricks, in build order

1. **Contract: component props, tests first.** Sdk `component.rs` (one
   noun: what a component draws). Each of the fifteen has an sdk type
   with Claude Code's prop names (its `claude-code.d.ts`, 9777–10340):
   `ToolUse { tool, input, isRunning, isErrored, isInterrupted, output }`,
   `AbovePrompt { hasSurvey, isWorking, maxRows, bodyColumns, … }`,
   `Pane { title, isFocused, bodyColumns }`, and so on. Each is a pure
   projection of what `SessionState` already holds, built at draw time,
   never stored. A field bingo has no source for takes its resting value
   (`hasSurvey: false`), so a port type-checks unchanged. Golden JSON per
   component, generated into `schema/rpc.json` and, through
   `bingo-schema-ts` (M133), the TypeScript types. They fail first.
2. **Matched components, published.** Core reads the matchers of the
   links at `ui.render` (M131's `LinkMatcher`, a partial of `e`) and
   publishes `{ component: [partial…] }` under `ConfigView.kernel.render`,
   again when a plugin loads, reloads or leaves. The sdk gains
   `render::matched(&ConfigView) -> Matched` and the pure
   `Matched::asks(component, &request)`, which uses the same partial
   matcher the runtime applies to `e` (moved to the sdk if M137 left it
   in `bingo-mods`: one matcher, two callers). A matcher holding a
   RegExp or a function publishes as its component alone, so a surface
   asks more, never less. Table tests.
3. **`Engine` substitution, pure.** TUI `component.rs`: `draw(component,
   props, answer: Option<&View>, own) -> Vec<Line>`. No answer draws
   `own(props)`. An answer renders through `views::render_with(view,
   width, engine)`, where each `Engine { component, props }` draws `own`
   of those props parsed back into the component's type; props that do
   not parse draw `own` of the original, with one notice. Table tests
   over a stub `own`.
4. **Each component asks, only when matched.** The module that draws it
   today calls `component::draw` when `matched.asks(…)`, and otherwise
   draws exactly as before: `transcript.rs` and `transcript/{said,
   output}.rs` (messages, tool rows and results, command output,
   `InfoNotice`), `activity.rs` (`Spinner`, `ToolProgress`,
   `AbovePrompt`), `worked.rs` (`TurnDuration`), `status.rs`
   (`SessionMode`, `PromptHint`), `form.rs` (`AskUserQuestion`), M140's
   `pane.rs` (`Pane`). Today's drawings read only those props, so a
   rewritten prop is what the person sees. `requestId` is the item's id
   (`tool_use_id` for a tool row), or the component's name for one that
   occurs once. Only rows on screen ask; M140's memo answers a row
   scrolled back to; `blocks.rs` keys its kept block on the answer's
   generation. M136's `turn.complete` text, an `Item::Notice` after
   `TurnCompleted`, draws under the answer through `InfoNotice`, and
   `TurnDuration` keeps its own row: a `TestBackend` test with hooks on
   both pins two rows, two requests, in that order.
5. **The permission prompt is not a component.** `Component` has no
   variant for it (M140), and `AskUserQuestion`'s projection answers
   `None` for the permission kind, so `dialog.rs` has no props to send.
   A test loads a hook matching `{ component: 'AskUserQuestion' }` with
   no partial, raises a permission interaction, and counts zero
   `ui/render` requests.
6. **`$.ui.log`**, an op event: `tracing` at target `mod::<plugin>`, into
   the log file bingo already keeps.
7. **The state helpers in `'bingo'`.** `sdk/typescript/src/state.ts`:
   `atom(key, initial)`, `derive(fn)` (memoised per render), `read($,
   atom)`, `update($, atom, fn)` (retries an `ifVersion` conflict three
   times, then throws), `memberOf(atom, id)` — pure TypeScript over
   `$.state` (M138). The in-process `bingo` module embeds the same file
   (`include_str!`), so both transports run one source. They land here
   because the examples use them; M142's kit tests them.
8. **The examples, bingo's own.** `examples/plugins/{tool-call,band,
   pane}/`, each a `plugin.json` naming its `module`, the source,
   `types/index.d.ts` (its `PluginState`) and a README, written fresh for
   bingo: nothing is copied from Claude Code's examples, which carry no
   licence. They import `'bingo'`, use the helpers, match on `component`
   and `requestId` and read `e.props.hasSurvey` as Claude Code's names
   are on the wire, and name bingo's own tools and inputs. `tool-call`
   denies an edit to a protected path and rewrites a shell command,
   counting in `$.ui.status`; `band` answers `AbovePrompt` with a
   `Button` that hides it; `pane` opens a pane from a command and answers
   its `requestId` with one row per tool call. M146 consumes them; M142
   adds their `*.test.ts`.
9. **The acceptance run.** `crates/bingo/tests/mods_components.rs`
   drives each example over `bingo serve --stdio` with the fake provider
   (`band` shows and hides; `pane` gains a row per call and keeps it
   across a reload; `tool-call` denies, rewrites, counts);
   `scripts/tui-smoke.sh` gains one `--plugin-dir` case per example.

## Files

- `crates/bingo-sdk/src/{component.rs (new),render.rs}`; the matcher's home (brick 2)
- `crates/bingo-core/src/chain/render.rs` (the matched components), its config-view writer
- `crates/bingo-mods/src/` `$.ui.log`, the embedded helpers; `sdk/typescript/src/{state.ts (new),index.ts}`
- `crates/bingo-surface-tui/src/{component.rs (new),views/mod.rs,transcript.rs,transcript/{said,output}.rs,
  activity.rs,worked.rs,status.rs,form.rs,dialog.rs,blocks.rs}`
- `schema/{rpc,events}.json`; the generated TypeScript types
- `examples/plugins/{tool-call,band,pane}/` (new)
- `crates/bingo/tests/mods_components.rs` (new); `scripts/tui-smoke.sh`

## Exit criteria

- [ ] `cargo test -p bingo-sdk component:: render::matched` passes: each
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
- The desktop app drawing components (M144); print and channels asking
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
- Re-verify at start: M140 as landed (`render.rs`, `pulled.rs` and its
  memo key, `answer_ok`, `Shown`); whether M131's `LinkMatcher` holds a
  partial of `e`; where M137 put the matcher; which TUI module draws
  `ToolGroup` and `PromptHint`; bingo's tool names and input fields;
  the `InteractionKind` variants; the prop list in 2.1.296's types.
