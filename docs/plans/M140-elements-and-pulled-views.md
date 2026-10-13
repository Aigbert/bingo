# M140 — Elements and pulled views
## Goal

ADR-0079 §1–3, §5, §6 (components are M141). `View` grows nine elements
in Claude Code's words, each with a fold; each surface publishes its
table. A pulled pane is asked for at its drawn width (`HostApi::render`,
`ui/render`) and again when its runtime calls `HostApi::invalidate`
(`host/ui/invalidate`). Handlers return through `Input::Action` as
`ui.press`/`input`/`select`. The TUI draws the terminal table, seats
panes, stacks toasts. After M121, M123–M125, M137, M138.
## Bricks, in build order

1. **Elements, tests first.** Split sdk `view.rs` into `view/{mod,read,fold,table,tests}.rs`.
   Add nine nodes and golden JSON/folds (Engine folds empty). Text/Button have children
   (strings or nested styled Text); legacy Text.text and Code.text/lang still read;
   Code writes source/language. Canonical JSX names reach serde unmapped.
2. **Drawn props, one table.** Box: key, flexDirection(row|column), flexGrow, flexWrap,
   alignItems, justifyContent, gap, width, height, margin[X|Y], padding[X|Y], borderStyle,
   borderColor, display. Text: color, dimColor, bold, italic, underline, strikethrough,
   inverse, wrap. Button: key, label, hotkey, dimColor, variant, onPress.
   Input: key, label, placeholder, value, onInput, onSubmit. Select: key, label,
   options, value, onSelect. Link: href, label. Image: source, alt, columns, rows.
   Raster: columns, rows, cells. Svg: source, alt, width, height. Code: source,
   language. Markdown: text, dimColor. Props outside the table are ignored.
3. **Legacy containers.** Stack reads as column Box; Columns as wrapping row Box,
   children with flexGrow 1 and widths sharing NARROW=60 including two-cell gutters.
   Delete Stack/Columns variants; schema kinds plus READ_ONLY pin KNOWN. Writers:
   experience/command, schedule/render and tools/list, demo-ui/board, tasks/journal.
4. **Tables/validation.** TERMINAL, DESKTOP, TEXT and pure outside(table, view) paths.
   answer_ok checks only structure: Engine matches requested component, handlers name
   the answering mod's `<plugin>:ui`, at most 4096 nodes. published_ok refuses Engine.
5. **Places.** Shown::View or Pull{request}; old JSON reads View. Pane{id,title,focus,
   close_on_escape,hold_toasts,asked,rows,view}; Toast{text,until,alert}; Activity can pull.
   Golden fixtures and published_ok cover each.
6. **Render pair.** sdk Component (ADR-0079's fifteen, no permission), Viewport,
   SurfaceName, RenderRequest{component,requestId,surface,props,viewport}.
   HostApi::render defaults Engine; invalidate defaults no-op. Core raises ui.render
   through M131's runner off the mailbox, four in flight/session; latest queued ask
   replaces the same (component,requestId). Bad tree/hook: Engine plus one plugin notice.
   Pure throttle limits invalidations to 30/s/request; ephemeral Invalidated folds
   a generation per request. ui/render is additive to surface RPC PROTOCOL 1.
7. **Subscriptions.** plugin-host/subscribed: subscribe(requestId,reads), changed(scope,key),
   forget. Both runtimes invalidate on writes to reads or $.ui.invalidate. Bind render
   scope to M138's state face: state.set while rendering is refused in words.
   Node ui.ts mints handlers; plugin schema adds host/ui/invalidate.
8. **Handlers.** CommandSpec.hidden defaults false, excluded from listing/completion.
   Each runtime contributes instant hidden `<plugin>:ui`. Per-request generations mint
   Action args {handler:"<generation>:<n>",event}; surface adds value via Input::Action.
   Raise ui.press/input/select in that mod alone; stale generation notices; Escape
   fires event close for a close_on_escape pane.
9. **$.ui.** All methods raise op events: status, toast(default 4000ms), notify(alert),
   open(pulled Pane, id=requestId, resolves {}, asked only under person input), close,
   panes, invalidate, resolve(table), notice/ask(existing HostApi). Components/log: M141.
10. **TUI.** Pure flex layout handles the drawn Box props; wrapping rows stack when
    they do not fit. Hand layout unless a fixture justifies taffy; paste budget either way.
    Dedicated button/input/select/link/image/raster renderers; text draws styled spans;
    Button uses Actions marks, Input the composer editor. Asked panes sit above composer
    at one-third height at every width; unasked waits in status, click opens (M151
    adds placements). Toast stack above status; until feeds next_frame, hold_toasts
    pauses it, new alert notifies an unfocused terminal. Focus::Pane(id).
    Pulled memo: (component,requestId,props,columns,generation), ellipsis until answer.
11. **Black box.** Counter mod (pane, Button, Input); serve --stdio with fake provider:
    ui/render, session/input press, Invalidated, incremented count. Isolated tmux drive.
## Files

- `crates/bingo-sdk/src/view.rs` → `view/…`; `{render.rs (new),slot,host,event,state,command,catalogue…}`
- `crates/bingo-core/src/{chain/render,host/throttle}.rs`, `crates/bingo-plugin-host/src/subscribed.rs`,
  `crates/bingo-mods/src/{handlers,ui}.rs`, runtime/chain/JS; plugin-host state/host and plugin-rpc bridge; `sdk/typescript/src/ui.ts` (new)
- `crates/bingo-surface-rpc/src/methods.rs` + dispatch; `schema/{rpc,plugin,events}.json`; brick 3's writers
- `crates/bingo-surface-tui/src/views/{flex,button,input,select,link,image,raster}.rs`, `{pane,toast,pulled}.rs`
  (new), `views/{stack,columns}.rs` (deleted), `views/{mod,text,tests}.rs`, `focus,layout,activity,status,terminal.rs`
- `crates/bingo/tests/{mods_ui.rs,fixtures/mods/counter/}` (new); `scripts/tui-smoke.sh`
- Module declarations, host/session wiring, TUI `ui,run,input,pointer,view,layout/deadline,transcript/output`,
  command catalog filtering and TypeScript exports/transport wiring for these bricks.
- Mechanical `Text`/`Code`/`Stack` and `CommandSpec.hidden` constructor updates in existing
  producers, fixtures and generators; exclude M139 world nouns, grant.rs, MCP/agents service rows.
## Exit criteria

- [ ] `cargo test -p bingo-sdk`: each node
      round-trips/folds; legacy Stack/Columns/Text/Code read; every `answer_ok` refusal.
      `rg -n 'View::(Stack|Columns)' crates` prints nothing.
- [ ] `cargo test -p bingo-core chain::render` and `cargo test -p bingo-core host::throttle`: the kernel
      link answers `Engine`; a throwing hook and a bad tree each answer
      `Engine` with one notice; 100 invalidations send ≤30 frames a second.
- [ ] `cargo test -p bingo-plugin-host subscribed::`, `-p bingo-mods`,
      `-p bingo --test mods_ui`: a read subscribes, a write invalidates,
      `set` in a render is refused, brick 11 passes.
- [ ] `cargo test -p bingo-surface-tui`: a `TestBackend` test per element;
      panes asked (inline at 200 and 80) and unasked (waiting at 200,
      clicked open); toasts held by `hold_toasts`, gone at `until`.
- [ ] The `schedule`, `experience` and `demo-ui` narrow snapshots are
      unchanged; any diff is pasted and driven in tmux.
- [ ] `scripts/tui-smoke.sh` passes with `counter`, a tmux drive pasted;
      every gate; the Windows check of `bingo-surface-tui` and
      `bingo-mods`; budget output pasted with the taffy decision.
## Non-goals

- Components/`$.ui.log` (M141), desktop rendering, non-above placement (M151),
  Client/blit, extra styling, OSC 8, isPlaced, copy/scroll/focus/selection (design §8).
## Risks

- `onInput` per keystroke crosses command/hook/write/render; coalesce per frame if slow.
- Re-verified at `5581b978`: M148 layout/canvas replaces rail/sheet; M131's
  runner/catalogue and M138's `Call.reads`/`ReadLog` exist; render guard does not.
  `globals.js` still emits `{type,props,children}`. Direct Text writers need migration.
- **Stopped (2026-10-14), protocol step 6:** ADR-0075 §1 freezes protocol 2.
  `schema::tests::protocol_two_keeps_every_frozen_shape` fails after the element change.
  The only frozen definition failing `preserves` is View: text, markdown, code,
  stack and columns alternatives. Legacy reads pass; canonical writes change.
  ADR-0079 §1 requires the new vocabulary but does not amend ADR-0075's freeze.
  Options: authorize a narrowly scoped pre-release View/Shown baseline amendment
  (as prior milestones did for their named changes), or retain the freeze and
  design protocol-3/N−1 compatibility. Do not change plugin-2.json or weaken the test.
- Partial work preserves the SDK elements/legacy reader/folds and mechanical callers;
  TUI additions currently use text fallback. Tables, render pair, subscriptions,
  handlers, pulled UI, full TUI drawing and counter acceptance remain unfinished.
  No ADR/exit criterion is accepted; after stopping only formatting/artifact regeneration ran.
  Remaining gates (clippy, workspace tests, discipline, budget, deny, Windows, PTY)
  were not run. No live/credential tests, terminal app, push, merge or PR.
## Verified (2026-10-14)
```text
cargo fmt --all -- --check
exit_code: 0
cargo test -p bingo-sdk view::contract --locked
unlearned: {"kind":"Box","children":[{"kind":"Text","children":["one"]},{"kind":"Text","children":["two"]}]}
test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 137 filtered out
cargo test -p bingo-sdk view::contract --locked
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 137 filtered out
BINGO_UPDATE_SCHEMA=1 INSTA_UPDATE=always cargo test -p bingo-sdk --lib --locked
test result: ok. 139 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
cargo check --workspace --all-targets --locked
error[E0164]: expected tuple struct or tuple variant, found associated function `View::text`
cargo check --workspace --all-targets --locked
Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.15s
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk --test record --locked
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc schema:: --locked
protocol 2 removed or retyped a frozen shape
test result: FAILED. 9 passed; 1 failed; 0 ignored; 0 measured; 120 filtered out
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-surface-rpc schema::tests::the_committed_schema_is_this_document --locked
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 13 filtered out
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc wire_ts:: --locked
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 129 filtered out
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc events_ts:: --locked
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 129 filtered out
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-mods dollar::tests::schema_and_types_are_generated_from_the_same_rows --locked
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc schema::tests::the_committed_schema_is_this_document --locked
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 129 filtered out
```
