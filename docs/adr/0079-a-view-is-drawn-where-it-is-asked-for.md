# ADR-0079 — A view is drawn where it is asked for

Status: accepted for M140 (§§1–3,5–7) · 2026-10-14; §4 component integration remains proposed for M141 · Amends: ADR-0013 §1 §3, ADR-0070 §1

**2026-10-14:** §3’s future pane docking is subject to ADR-0081 §7, still proposed for M151 (including §5’s viewport). M148 implements ADR-0081 §§1–6 only: no rail or panel sheet; it does not implement panes.

## Context

A Claude Code mod draws by answering `ui.render` for a component: a tool
row, a message, the band above the prompt, a pane it opened. It answers
with an element tree from the asking surface's own table
(`$.ui.resolve(e)`), or with `next(e)`, the engine's own drawing, with
props it may have rewritten. Handlers stay in the mod and come back as
`ui.press`, `ui.input` and `ui.select`. A drawing that read `$.state` is
asked again when that value changes. (Summary and line references:
[design/mods-parity-2026-10.md](../design/mods-parity-2026-10.md).)

bingo has the vocabulary (`View`, ADR-0013) and the places (slots,
ADR-0070), but every view is pushed, drawn at a width its plugin never
saw; nothing wraps a surface's own drawing, and a press only runs a
command.

**The ratchet.** `HostApi::render` and its pair `invalidate` are new
verbs. Without them each surface runs mods itself, with a synced copy of
the state the host holds (ADR-0078 §6). This door opens, no other.

## Decision

1. **The vocabulary is the element table, in Claude Code's words.**
   `View` gains the elements a mod draws with. Every name a mod meets is
   spelled as Claude Code spells it: element and prop names
   (`flexDirection`, `dimColor`, `onPress`), components, event fields.
   The sdk types carry the serde names, so JSX reaches the host unmapped.
   bingo draws a subset of each element's props, listed by M140;
   `validate` names a prop outside it, and a surface ignores it. Each
   element has a fold (ADR-0013 §1):
   - `Box`: direction, grow, wrap, align, justify, gap, padding, margin,
     border, width, height. `Stack` reads as a column `Box`, `Columns`
     as a wrapping row; nothing writes them any more (ADR-0065).
   - `Text` gains `color`, `dimColor`, `bold`, `italic`, `underline`,
     `strikethrough`, `inverse` and `wrap`. `Text` and `Button` hold
     `children`; `Code` writes `source` and `language`. Old shapes are
     still read (ADR-0065); a reader knowing only them cannot read these.
   - `Button`, `Input`, `Select`, `Link`, `Image`, `Raster`, `Svg`.
   - `Engine { component, props }`: what `next(e)` resolves to, the
     surface's own drawing; only in a render answer, never in a slot.

   A handler (`onPress`, `onInput`, `onSubmit`, `onSelect`) is an
   `Action` the runtime minted (§6). A port changes its import specifier
   and the tool names and inputs it matches (bingo's tools' own).
2. **Each surface has a table.** A surface publishes the elements it
   draws, under a name: `terminal` (everything but `Svg`), `desktop`
   (everything but `Raster`), `text` (print and channels: folds only).
   `$.ui.resolve(e)` returns the table for `e.surface`. A node outside
   the asking surface's table draws its fold, as an unknown `View` does.
3. **A place is pushed or pulled.** A slot (ADR-0070) says where. Its
   view is either data, as today, or `Pull { request }`: "ask me".
   - `$.ui.open` publishes `Slot::Pane { id, title, focus,
     close_on_escape, hold_toasts, asked, rows }` with a pulled view.
     `asked` is set when the opening hook ran under the person's own
     input (a command, a key, a press); a pane not asked waits for room.
   - The band above the prompt is the `Activity` slot, pulled.
     `$.ui.status` writes `Status`; `$.ui.toast` and `$.ui.notify` write
     `Slot::Toast { text, until, alert }`. Placement stays the surface's.
4. **Components.** A component is one of the surface's own drawings that
   a render hook may answer: `UserMessage`, `AssistantMessage`,
   `ToolUse`, `ToolResult`, `ToolGroup`, `ToolProgress`, `CommandOutput`,
   `Spinner`, `TurnDuration`, `InfoNotice`, `SessionMode`, `PromptHint`,
   `AbovePrompt`, `Pane` and `Question`.
   - Its props are an sdk type: the item, `viewport`, `focused`,
     `expanded`.
   - The config view lists the components some loaded hook matches; a
     surface asks only about those, drawing its own until answered.
   - The permission prompt is not one: the engine alone draws it.
5. **`HostApi::render(session, RenderRequest) -> View`**, wire
   `ui/render`, request `{ component, requestId, surface, props,
   viewport }`.
   - The host raises the `ui.render` chain (ADR-0078); the kernel link
     answers `Engine { component, props }`.
   - The answer is checked for structure only: `Engine` appears only as
     the asked component, every handler was minted by the answering mod,
     and the tree is within a size cap. A tree that fails, or a hook
     that fails, is answered `Engine`, with one notice line naming the
     plugin. Elements are not checked against the table (§2).
   - Inside a render, `$.state.set` is refused.
   - A surface caches by `(component, requestId, props, columns)` and
     asks again when one changes or on `Event::Invalidated { requests }`
     (live, never journaled). A change of height asks nothing.
   - **`HostApi::invalidate(session, requests)`**, wire
     `host/ui/invalidate`, is the door's other half. The runtime that
     answered calls it when a `$.state` value a render read changes, or
     when a mod calls `$.ui.invalidate`. Core sends the frame, at most
     30 a second per request.
6. **Interaction goes through the existing door.** A mod's runtime
   contributes one hidden command, `<plugin>:ui` (`CommandSpec.hidden`:
   never listed, never completed). A handler is the `Action { name:
   "<plugin>:ui", args: { handler, event, value } }` that the runtime
   minted when it built the tree. Firing it submits `Input::Action`
   (ADR-0013 §3); the command raises `ui.press`, `ui.input` or
   `ui.select` in that mod. A handler from before a reload answers with
   a notice, not an error.
7. **Tested where it is drawn.** Each surface tests its table once per
   element; a plugin asserts its tree with `assert_eq!`.

## Consequences

- `View` grows by nine nodes with folds, typed from `schema/rpc.json`.
  `desktop` is a table for wire clients; no surface here draws it.
- The TUI's layout pass (M124) lays out a `Box`: direction, gap,
  padding, grow, fixed sizes and wrap, where a row whose children do not
  fit stacks them. Whether a layout crate pays for itself is M140's
  decision, recorded with `scripts/budget.sh`.
- A component no hook matches costs nothing. A surface's scroll, focus,
  selection and clipboard stay its own, so `$.ui.scroll`, `focus`,
  `selection`, `copy` and `open`'s `isPlaced` are gaps (design §8).

Refs: ADR-0002, ADR-0013, ADR-0065, ADR-0070, ADR-0078, ADR-0080; design/mods-parity-2026-10.md.
Non-goals: `Client` modules, meaning code that runs on a surface's
drawing thread, and `$.ui.blit`. A mod drawing the permission prompt.
Styling beyond the table.

2026-10-14, person's delegate: pre-release, M140: the AskUserQuestion component is Question; the kernel names no tool (check_discipline 4c).
