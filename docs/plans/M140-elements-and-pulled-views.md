# M140 — Elements and pulled views
## Goal
ADR-0079 §1–3, §5, §6 (components are M141). `View` grows nine elements
with folds and surface tables. Pulled panes render at drawn width and invalidate
through HostApi::render/invalidate (ui/render, host/ui/invalidate). Handlers return
via Input::Action as ui.press/input/select. TUI draws elements, panes and toasts.
After M121, M123–M125, M137, M138.
## Bricks, in build order

1. **Elements, tests first.** Split sdk `view.rs` into `view/{mod,read,fold,table,tests}.rs`. Add nine nodes and golden JSON/folds (Engine folds empty). Text/Button have children (strings or nested styled Text); legacy Text.text and Code.text/lang still read; Code writes source/language. Canonical JSX names reach serde unmapped.
2. **Drawn props, one table.** Box: key, flexDirection(row|column), flexGrow, flexWrap, alignItems, justifyContent, gap, width, height, margin[X|Y], padding[X|Y], borderStyle, borderColor, display. Text: color, dimColor, bold, italic, underline, strikethrough, inverse, wrap. Button: key, label, hotkey, dimColor, variant, onPress. Input: key, label, placeholder, value, onInput, onSubmit. Select: key, label, options, value, onSelect. Link: href, label. Image: source, alt, columns, rows. Raster: columns, rows, cells. Svg: source, alt, width, height. Code: source, language. Markdown: text, dimColor. Props outside the table are ignored.
3. **Legacy containers.** Stack reads as column Box; Columns as wrapping row Box, children with flexGrow 1 and widths sharing NARROW=60 including two-cell gutters. Delete Stack/Columns variants; schema kinds plus READ_ONLY pin KNOWN. Writers: experience/command, schedule/render and tools/list, demo-ui/board, tasks/journal.
4. **Tables/validation.** TERMINAL, DESKTOP, TEXT and pure outside(table, view) paths. answer_ok checks only structure: Engine matches requested component, handlers name the answering mod's `<plugin>:ui`, at most 4096 nodes. published_ok refuses Engine.
5. **Places.** Shown::View or Pull{request}; old JSON reads View. Pane{id,title,focus, close_on_escape,hold_toasts,asked,rows,view}; Toast{text,until,alert}; Activity can pull. Golden fixtures and published_ok cover each.
6. **Render pair.** sdk Component (ADR-0079's fifteen, no permission), Viewport, SurfaceName, RenderRequest{component,requestId,surface,props,viewport}. HostApi::render defaults Engine; invalidate defaults no-op. Core raises ui.render through M131's runner off the mailbox, four in flight/session; latest queued ask replaces the same (component,requestId). Bad tree/hook: Engine plus one plugin notice. Pure throttle limits invalidations to 30/s/request; ephemeral Invalidated folds a generation per request. ui/render is additive to surface RPC PROTOCOL 1.
7. **Subscriptions.** plugin-host/subscribed: subscribe(requestId,reads), changed(scope,key), forget. Both runtimes invalidate on writes to reads or $.ui.invalidate. Bind render scope to M138's state face: state.set while rendering is refused in words. Node ui.ts mints handlers; plugin schema adds host/ui/invalidate.
8. **Handlers.** CommandSpec.hidden defaults false, excluded from listing/completion. Each runtime contributes instant hidden `<plugin>:ui`. Per-request generations mint Action args {handler:"<generation>:<n>",event}; surface adds value via Input::Action.
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

- [ ] `cargo test -p bingo-sdk`: each node round-trips/folds; legacy Stack/Columns/Text/Code read; every `answer_ok` refusal.
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
- [ ] Schedule/experience/demo-ui narrow snapshots unchanged; any diff pasted and driven in tmux.
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
- 2026-10-14, person's delegate: authorize the scoped pre-release View/Shown/place/ui-render baseline amendment; protocol 2 freezes at its first release (ADR-0075).
- 2026-10-14, person's delegate: authorize print Invalidated ignore arms and all mechanical Event/Shown/element sibling mappings; record files below, stop only for new out-of-scope behaviour.
- 2026-10-14, person's delegate: authorize theme-test file additions only for fitting token roles, keep ember/accent lists and theme values unchanged; merge dev before regenerating and running final gates.
## Verified (2026-10-14)

Theme allowlist additions: text → `toast.rs`, `views/{input,select}.rs`; dim → `views/{button,flex,input,link,mod,text}.rs`; bad → `views/button.rs`; no ember/accent additions.
Mechanical match updates: `bingo-surface-print/src/{render,stream_json}.rs` (ignore Invalidated).
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
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc schema::tests::the_committed_schema_is_this_document --locked
test schema::tests::the_committed_schema_is_this_document ... ok
cargo test -p bingo-sdk view::validate --locked
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 140 filtered out
cargo test -p bingo-sdk view::validate --locked
test view::validate::tests::answers_reject_wrong_engines_foreign_handlers_and_oversized_trees ... ok
cargo test -p bingo-sdk slot::pull_contract --locked
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 142 filtered out
cargo check --workspace --all-targets --locked
error[E0004]: non-exhaustive patterns: `&bingo_sdk::Event::Invalidated { .. }` not covered
error: could not compile `bingo-surface-print` (lib) due to 2 previous errors
error: could not compile `bingo-surface-tui` (lib) due to 3 previous errors
cargo test -p bingo-surface-tui views::tests --locked
test result: FAILED. 9 passed; 3 failed; 0 ignored; 0 measured; 1090 filtered out
cargo test -p bingo-surface-tui views::tests --locked
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 1090 filtered out
cargo test -p bingo-surface-tui pulled:: --locked
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1102 filtered out
cargo test -p bingo-surface-tui pane:: --locked
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1104 filtered out
cargo test -p bingo-surface-tui toast:: --locked
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1104 filtered out
cargo test -p bingo-surface-tui pane:: --locked
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1104 filtered out
cargo test -p bingo-surface-tui toast:: --locked
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1104 filtered out
node --test sdk/typescript/src/ui.test.ts
fail 1: Escape raises close in the mod and removes only its current closable pane
node --test sdk/typescript/src/ui.test.ts
pass 3; fail 0
cargo test -p bingo-surface-tui --lib --locked
theme::tests::every_token_is_spent_where_the_design_says: assertion failed: where `text` is spent
test result: FAILED. 1104 passed; 1 failed; 2 ignored; 0 measured; 0 filtered out
```
