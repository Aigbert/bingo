# M144 — The desktop app draws what mods draw

## Goal

ADR-0079 on the desktop, and ADR-0080 §6. The desktop becomes ADR-0079's
`desktop` surface. It draws every element in the desktop table (all but
`Raster`). It asks `ui/render` for the components some loaded hook
matches and for pulled views. It docks panes beside the stage and stacks
toasts. It draws the `Status`, `Mode`, `Activity` and `Row` slots, and
its handlers fire the `Action` the mod runtime minted. Placed slots and
`bingo-collab`'s records reach it through the sdk's own readers,
compiled into M143's wasm, so it parses no producer's payload and names
no plugin; the records' schema is generated to TS (ADR-0070 §5). Every
name on the wire is Claude Code's, carried by the sdk's serde names
(ADR-0079 §1), and the desktop maps none. It never runs mod code. After
M140, M141 and M143.

## Bricks, in build order

1. **Contract: the table and the components, tests first.**
   - `src/renderer/elements/table.ts` maps each node of `View` in
     `schema/rpc.json` to a drawing.
   - `src/renderer/components/index.ts` maps each component to the
     desktop's own drawing, which takes that component's generated props
     type.
   - Two vitests fail first: a node with no drawing (only `Raster` is
     excused, ADR-0079 §2), and a component with none.
   - The names must equal M140's `DESKTOP` table, read through the
     generated types.
2. **The readers are the sdk's.** `bingo-fold-wasm` exports:
   - `placed()` (`slot::placed`, ADR-0070 §2);
   - `fold_view(view)`, for a node outside the table (ADR-0013 §1);
   - the readers M122 and M123 gave `bingo-collab`: seats, roster and
     `@` targets.

   `bingo-collab` takes the sdk without `host`, and a schema test writes
   `schema/collab.json`, which M143's renderer turns into TS. Native Rust
   tests come first, then a vitest twin against
   `fixtures/{placed,collab}.json`.
3. **Elements, one module and one vitest each** (ADR-0079 §7).
   - Each element draws the props subset M140 lists, under the generated
     types' names, and ignores any other (tested once). Handlers are
     `onPress`, `onInput`, `onSubmit` and `onSelect`.
   - `Box` is CSS flex, fixed sizes in `ch` and rows. `Text` draws
     `color`, `dimColor`, `bold`, `italic`, `underline`, `strikethrough`,
     `inverse` and `wrap`. `Input` fires `onInput` after IME composition
     ends. `Link` opens externally and refuses `file:`. `Image` draws the
     picture as M115 serves it.
   - `Svg` is an `<img>` data URL, never inline DOM, so its script and
     external references stay inert. `Engine { component, props }` is the
     component's own drawing with those props.
   - The fifteen existing kinds move here from M143's generic drawing.
4. **The render cache, a pure brick.**
   - `render/cache.ts` keys on `(component, requestId, props, columns)`.
     A hit draws. A miss draws the engine's own drawing and asks once. A
     change of height alone asks nothing.
   - `Event::Invalidated { requests }` drops those keys, and redraws
     coalesce per animation frame. All of this has table tests.
   - `render/client.ts` sends `ui/render` `{ component, requestId,
     surface: "desktop", props, viewport }`. `viewport` counts the body
     font's `ch` and line height, so a mod reads one unit on every
     surface. A failed request draws `Engine`.
5. **Components.**
   - The stage asks only for the components the config view lists as
     matched (`ConfigView.kernel.render`, M141).
   - Each ADR-0079 §4 component keeps its own drawing until answered,
     and the swap is not animated.
   - M143's question kind becomes the `AskUserQuestion` component's own
     drawing, and the `Mode` chip becomes `SessionMode`'s. The permission
     prompt stays the engine's.
6. **Places.**
   - **`Pane`** (`id, title, focus, close_on_escape, hold_toasts, asked,
     rows`) docks right of the stage, resizable. An `asked` pane docks at
     once. One not asked waits behind a pill on the island until the
     window fits stage and pane, a pixel rule with its own test.
     `hold_toasts` holds the toast stack while that pane is open.
   - **`Toast`** (`text, until, alert`) stacks at the top right, at most
     50. An `alert` toast also raises an OS notification while the
     window is unfocused, as the TUI does (M141).
   - **`Status`** sits under the island. **`Mode`** is the island's chip,
     and shift+tab fires the item after `current`.
   - **`Activity`** is the band above the island, pulled. **`Row`** sets
     a tool row's mark, title, badge, body and actions.
7. **Handlers.** A press, a submit or a select sends the minted
   `Action` (`<plugin>:ui`, `{handler, event, value}`) through the
   wire's action method. The desktop reads none of its fields, and a
   test asserts that it sends exactly the minted args.
8. **The acceptance spec.** `tests/mods.e2e.ts` runs `bingo serve
   --stdio` with the fake provider and M140's and M141's test mods via
   `--plugin-dir`. It shows a band above the island, a docked pane, a
   tool row that `next(e)` wraps in a `Box`, a toast and a status.
   Pressing the band's button redraws it through an `Invalidated`
   re-ask.

## Files

- `crates/bingo-fold-wasm/src/{lib,placed,collab}.rs`, `fixtures/{placed,collab}.json`
- `crates/bingo-collab/{Cargo.toml,src/schema.rs}`, `schema/collab.json` (new); `src/shared/collab.ts` (generated)
- `apps/desktop/src/renderer/elements/{table.ts,Box,Text,Button,Input,Select,Link,Image,Svg,Engine}.tsx`,
  and the existing kinds moved here
- `apps/desktop/src/renderer/components/{index.ts,<one per component>.tsx}`
- `apps/desktop/src/renderer/{render/{cache,client}.ts,actions.ts}`
- `apps/desktop/src/renderer/places/{Dock,Toasts,Status,ModeChip,Band,Row}.tsx`
- `apps/desktop/src/renderer/shell/{Stage,Island,InteractionPanel}.tsx`
- Tests beside each module; `apps/desktop/tests/mods.e2e.ts`

## Exit criteria

- [ ] `npm --prefix apps/desktop test` passes. The table test fails on a
      seeded node with no drawing, and the components test fails on a
      seeded component (both pasted).
- [ ] `cargo test -p bingo-fold-wasm` and `cargo test -p bingo-collab`
      pass, and `schema/collab.json` regenerates unchanged.
- [ ] The cache tests show that a height-only resize asks nothing and
      that an `Invalidated` frame asks once. The Svg test shows that a
      `<script>` does not run. The props test shows that an unlisted
      prop is ignored.
- [ ] `mods.e2e.ts` passes on macOS locally and on CI's ubuntu and
      Windows (pasted).
- [ ] ``rg "rooms/|agent/|['\"`]_?bingo\." apps/desktop/src --glob '!**/*.test.*'``
      finds nothing, and the desktop check passes. All gates pass, and so
      does `cargo check -p bingo-fold-wasm --target wasm32-unknown-unknown`.

## Non-goals

- `Raster` (ADR-0079 §2); `Client` modules and `$.ui.blit`; a mod
  drawing the permission prompt; styling beyond the table (ADR-0079).
- `$.ui.scroll`, `focus`, `selection` and `copy`, and `open`'s
  `isPlaced` (parity §8). The desktop's scroll, focus, selection,
  clipboard and placement are its own (ADR-0013 §4).
- Running mod code in the desktop, or caching a drawing across restarts.
- The vscode and mobile tables (parity §8): no such surface exists.
- Rei's panes, markdown and tool presenters (M145).

## Risks

- `bingo-collab` must build for wasm32 with `host` off. Any crate in the
  wasm tree that takes the sdk's defaults turns `host` back on, and
  `cargo tree -e features` names it. If the readers need tokio, stop and
  ask.
- The terminal docks an unasked pane at 144 columns, and an asked one at
  110. The desktop's pixel rule is this plan's, shown in the e2e.
- If after M115 the wire has no read for a picture by digest, `Image`
  draws its `alt` and the gap is reported.
- Re-verify at start: M140's props subset, `DESKTOP` and serialized
  `Box` names; M141's components, props types and
  `ConfigView.kernel.render`; `ui/render` and `Invalidated` as landed;
  the wire's action method; `slot::placed` with the `Pane` and `Toast`
  fields; the collab readers; M115's picture read; the test mods'
  paths.
