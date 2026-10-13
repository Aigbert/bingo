# M140 — Elements and pulled views

## Goal

ADR-0079 §1–3, §5, §6 (components are M141). `View` grows nine elements
in Claude Code's words, each with a fold; each surface publishes its
table. A pulled pane is asked for at its drawn width (`HostApi::render`,
`ui/render`) and again when its runtime calls `HostApi::invalidate`
(`host/ui/invalidate`). Handlers return through `Input::Action` as
`ui.press`/`input`/`select`. The TUI draws the terminal table, docks
panes, stacks toasts. After M121, M123–M125, M137, M138.

## Bricks, in build order

1. **Contract: the elements, tests first.** `view.rs` (699 lines, two
   nouns) → `view/{mod,read,fold,table,tests}.rs`. Serde names are Claude
   Code's, so the nodes M137's `h` builds arrive unmapped. Golden JSON per
   node; "Every node once" gains the nine; `every_node_folds_to_text`
   pins each fold (`Engine` folds to nothing). `Text` and `Button` hold
   `children` as JSX gives them (strings and nested `Text`: styled spans);
   old `Text { text }` reads as one child; `Code` writes `source`/
   `language` and still reads `text`/`lang` (ADR-0065). All fail first.
2. **The drawn subset, as data.** `table.rs` names per element the props
   bingo draws; surface tests and M142's `validate` read this one list; a
   surface ignores any other prop. `Box`: key, flexDirection (row|column),
   flexGrow, flexWrap, alignItems, justifyContent, gap, width, height,
   margin[X|Y], padding[X|Y], borderStyle, borderColor, display. `Text`:
   color, dimColor, bold, italic, underline, strikethrough, inverse, wrap.
   `Button`: key, label, hotkey, dimColor, variant, onPress. `Input`: key,
   label, placeholder, value, onInput, onSubmit. `Select`: key, label,
   options, value, onSelect. `Link`: href, label. `Image`: source, alt,
   columns, rows. `Raster`: columns, rows, cells. `Svg`: source, alt,
   width, height. `Code`: source, language. `Markdown`: text, dimColor.
   Out: positions, hover, flexShrink, alignSelf, row/column gaps, minimum
   sizes, per-side spacing, backgroundColor, overflow, reversed
   directions, autoFocus, role, action, Code's path/startLine/format,
   Markdown's link presses.
3. **The old words read as `Box`.** `stack` is a column `Box`; `columns`
   a wrapping row whose n children each take `flexGrow: 1, width:
   NARROW / n`, so they stack below `NARROW` (moved here from the TUI) as
   before. `View::Stack`/`Columns` are deleted; `KNOWN` = the schema's
   kinds + `READ_ONLY = ["stack", "columns"]`. Writers move to `Box`:
   `bingo-experience/command.rs`, `bingo-schedule/{render,tools/list}.rs`,
   `bingo-demo-ui/board.rs`.
4. **Tables and the structural check.** `TERMINAL`, `DESKTOP`, `TEXT` as
   data; pure `outside(table, &View) -> Vec<Path>`, which a surface folds
   (§2). `answer_ok(component, &View)` checks structure only: `Engine`
   only as the asked component, every handler `<plugin>:ui` of the
   answering mod, at most 4 096 nodes. `published_ok` refuses `Engine`.
5. **Pulled places.** In `bingo_sdk::slot` a slot's view is `Shown::View`
   or `Shown::Pull { request }`; old JSON reads as `View`. New:
   `Slot::Pane { id, title, focus, close_on_escape, hold_toasts, asked,
   rows, view }`, `Slot::Toast { text, until, alert }`; `Activity.view`
   may pull. Golden fixtures; `published_ok` over each.
6. **The render pair.** Sdk `render.rs`: `Component` (§4's fifteen, no
   permission variant), `Viewport`, `SurfaceName`, `RenderRequest {
   component, requestId, surface, props, viewport }`. `HostApi::render`
   defaults to `Engine { component, props }`, `invalidate` to nothing.
   Core's `chain/render.rs` raises `ui.render` through M131's runner (the
   kernel link answers `Engine`; a failed hook or `answer_ok` answers
   `Engine` plus one notice naming the plugin), off the actor's mailbox,
   four in flight per session, a newer ask for one `(component,
   requestId)` replacing a queued one. `invalidate` passes the pure
   `host/throttle.rs` (30/s per request) and sends the ephemeral
   `Event::Invalidated`, which `SessionState` folds into a generation per
   request for the surface's memo. `UI_RENDER` is additive: `PROTOCOL` 1.
7. **Who calls `invalidate`.** Pure `bingo-plugin-host/subscribed.rs`:
   `subscribe(requestId, reads)` from M138's read log of the render
   invocation, `changed(scope, key) -> requests`, `forget`. The answering
   runtime keeps one and calls `invalidate` on a write M138's state face
   reports, or on `$.ui.invalidate`; it binds "in render" on that face,
   so `$.state.set` in a `ui.render` hook is refused in words. plugin-rpc
   does the same for a process (the node SDK's `ui.ts` mints handlers;
   `host/ui/invalidate` joins `schema/plugin.json`).
8. **Handlers through the door.** `CommandSpec.hidden` (default `false`):
   never listed, never completed. M137's runtime contributes
   `<plugin>:ui`, `instant` and `hidden`. `handlers.rs` mints `Action {
   name: "<plugin>:ui", args: { handler: "<generation>:<n>", event } }`,
   keyed by request and replaced by each answer. A surface adds `value`
   and fires `Input::Action`; the command raises `ui.press`, `ui.input`
   or `ui.select` in that mod alone; an old generation answers a notice.
   Escape on a `close_on_escape` pane fires `event: "close"`.
9. **`$.ui`, the pulled half**, each an op event (M138), `ui.*` in M136's
   catalogue: `status` → `Status`; `toast(text, ms = 4000)` and `notify`
   (`alert`) → `Slot::Toast`; `open(…)` → `Slot::Pane` pulling its own
   `id` as `requestId`, resolving `{}`, `asked` set when the opening hook
   ran under the person's own input (a command, a key, a press); `close`,
   `panes`, `invalidate`, `resolve`; `notice`/`ask` via `HostApi`.
10. **The TUI draws the terminal table.** `views/flex.rs` lays out a
    `Box` by hand, pure in tree and width; a wrapping row whose children
    do not fit becomes a column; taffy only if a fixture needs it (budget
    pasted either way). `views/{button,input,select,link,image,raster}.rs`;
    `text.rs` draws spans; `Button` uses the `Actions` marks, `Input` the
    composer's editor. Pure `pane.rs`: `asked` docks at ≥110 columns or
    sits inline above the composer at a third of the height; unasked docks
    at ≥144 or waits (counted in `status`). `toast.rs` stacks toasts above
    the status line (`until` feeds `next_frame`; held while a `hold_toasts`
    pane shows; a new `alert` notifies the unfocused terminal).
    `pulled.rs` memoises on `(component, requestId, props, columns,
    generation)`, drawing `…` until answered; `Focus::Pane(id)`.
11. **The black box.** Fixture mod `counter` (a pane, a `Button`, an
    `Input`); `tests/mods_ui.rs` over `bingo serve --stdio`, fake provider:
    `ui/render`, a press by `session/input`, `Invalidated`, the new count.

## Files

- `crates/bingo-sdk/src/view.rs` → `view/…`; `{render.rs (new),slot,host,event,state,command,catalogue…}`
- `crates/bingo-core/src/{chain/render,host/throttle}.rs`, `crates/bingo-plugin-host/src/subscribed.rs`,
  `crates/bingo-mods/src/handlers.rs` (new), `$.ui`; plugin-rpc's bridge; `sdk/typescript/src/ui.ts` (new)
- `crates/bingo-surface-rpc/src/methods.rs` + dispatch; `schema/{rpc,plugin,events}.json`; brick 3's writers
- `crates/bingo-surface-tui/src/views/{flex,button,input,select,link,image,raster}.rs`, `{pane,toast,pulled}.rs`
  (new), `views/{stack,columns}.rs` (deleted), `views/{mod,text,tests}.rs`, `focus,layout,activity,status,terminal.rs`
- `crates/bingo/tests/{mods_ui.rs,fixtures/mods/counter/}` (new); `scripts/tui-smoke.sh`

## Exit criteria

- [ ] `cargo test -p bingo-sdk view:: table:: slot:: render::`: each node
      round-trips and folds (snapshot diff pasted); `stack`, `columns`,
      old `Text` and `Code` read; each `answer_ok` refusal. `rg -n
      'View::(Stack|Columns)' crates` prints nothing.
- [ ] `cargo test -p bingo-core chain::render host::throttle`: the kernel
      link answers `Engine`; a throwing hook and a bad tree each answer
      `Engine` with one notice; 100 invalidations send ≤30 frames a second.
- [ ] `cargo test -p bingo-plugin-host subscribed::`, `-p bingo-mods`,
      `-p bingo --test mods_ui`: a read subscribes, a write invalidates,
      `set` in a render is refused, brick 11 passes.
- [ ] `cargo test -p bingo-surface-tui`: a `TestBackend` test per element;
      panes asked (docked at 120, inline at 80) and unasked (waiting at
      120); a toast held by `hold_toasts` and gone at `until`.
- [ ] The `schedule`, `experience` and `demo-ui` narrow snapshots are
      unchanged; any diff is pasted and driven in tmux.
- [ ] `scripts/tui-smoke.sh` passes with `counter`, a tmux drive pasted;
      every gate; the Windows check of `bingo-surface-tui` and
      `bingo-mods`; budget output pasted with the taffy decision.

## Non-goals

- Components and `$.ui.log` (M141); the desktop table (M144); `Client`,
  `blit`, styling beyond the subset; OSC 8 links (no hyperlink in a
  cell); `isPlaced`, `$.ui.copy`/`scroll`/`focus`/`selection` (design §8).

## Risks

- **Spans in `Text`.** Every Rust writer of `Text` uses a constructor.
  **`Input` traffic.** `onInput` per keystroke is a command, a hook, a
  write and a render; if the drive feels slow, coalesce to one per frame.
- Re-verify at start: `Slot`/`placed` (M121); `layout()`, `next_frame`
  (M124); `Focus`/`Render` (M125); the runner (M131); the catalogue
  (M136); `h`'s node shape, the loader (M137); the read log and state
  face (M138); `view.rs`, `KNOWN`, `CommandSpec`; what `Image.source` names.
