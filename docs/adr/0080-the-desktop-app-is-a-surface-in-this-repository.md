# ADR-0080 — The desktop app is a surface in this repository

Status: rejected · 2026-10-13 · Plans: M143, M144, M145, all dropped

**Rejected 2026-10-13.** The person keeps the GUI in its own repository
and may rewrite it from scratch, so nothing below moves in: no
`apps/desktop`, no node job, no `bingo-fold-wasm` (its `wasm-bindgen`
output is `unsafe`, which AGENTS.md forbids). The GUI stays a wire
client; ADR-0079's `desktop` table stays defined for it. Kept as history.

## Context

The desktop app lives in its own repository, `../Rei` (package
`bingo-gui` 0.1.1), built with Electron 44, React 19 and TypeScript 7.
It spawns `bingo serve --stdio` and speaks JSON-RPC protocol 1. Its
types and zod schemas are generated from `schema/rpc.json`, and a SHA
check catches drift. `foldSessionFrame` is a second copy of
`SessionState::apply`, tested against Rust fixtures. It draws all 15
`View` kinds, but not the panel lane or the live lane.

**It names plugins, as the TUI did before M123:**
- `config.plugins['bingo.permissions'].mode`, with the modes hardcoded;
- `extensions['bingo.rooms']`;
- the session-key prefixes `rooms/` and `agent/<parent>/`;
- about 40 tool names mapped to families;
- regexes over tool output text;
- the `schedule` catalog entry and four action names.

**It carries debt:** about 1.8k dead lines of the `--json-events` path, a
68 KB `useWorkspace.ts`, and two fake protocol hosts.

Phase 7 asks the desktop app to draw what the TUI draws for a mod:
elements, components, pulled panes and toasts (ADR-0079). That contract
changes in nearly every Phase 7 milestone. In another repository, a
drift check would only find the break after it had landed.

## Decision

1. **It lives at `apps/desktop`, in this repository.**
   - It is an npm package with its own lockfile, not a cargo member.
   - CI gains a `desktop` job, with node pinned. It runs the type check,
     the unit tests, the fold-twin test against the sdk's own fixtures,
     and a Playwright smoke test against a `bingo serve --stdio` built
     in the same run with `bingo-provider-fake`.
   - A schema change and the desktop break it causes fail the same pull
     request.
2. **The stack is Electron, React, TypeScript and electron-vite**, kept
   from Rei. Electron 44 runs one Chromium on all three platforms, so the
   desktop table is tested once. Its IME (CJK) and accessibility support
   is mature. Codex, Cursor and opencode ship on it; opencode left Tauri
   for it in 2026. Two alternatives were rejected:
   - **Tauri 2** draws with each OS's webview: WebKitGTK on Linux, which
     the distro supplies. It has open IME bugs that block CJK input
     (#15924, #15436), and Rei's `node-pty` would need a Rust
     replacement.
   - **GPUI** is pre-1.0, and crates.io has had only 0.2.2 since
     2025-10. It would rebuild streaming markdown, the terminal and the
     browser pane from nothing; `../bingo-gui-spike` stopped there.
3. **The fold is the sdk's, not a twin.**
   - `crates/bingo-fold-wasm` compiles `SessionState::apply` to
     WebAssembly for the desktop app, through `wasm-bindgen` (already in
     `Cargo.lock`), with JSON strings across the boundary.
   - `bingo-sdk` gains a default `host` feature, which the wasm build
     leaves off.
   - Rei's `foldSessionFrame`, a second copy of the fold, does not come
     across.
4. **It is rebuilt, not moved.** `apps/desktop` starts empty, and Rei's
   parts come across one at a time, each with its test:
   - the code generation, as one renderer: `bingo-schema-ts` (M133),
     extended to zod, replaces Rei's script; the drift test lives in
     `bingo-fold-wasm`;
   - the RPC client's hardening and the interaction panel;
   - streaming markdown and the split of the tool UI;
   - the review, browser and terminal panes;
   - the end-to-end harness.

   These do not come across: the `--json-events` path, `useWorkspace.ts`
   as one module (it is split one noun to a module), and both fake
   hosts, whose place is taken by `bingo serve` with the fake provider,
   and by in-memory streams for the cases that need a host misbehaving.
5. **It names no plugin.** The rule M123 gives the TUI applies here too.
   Production code holds no `"bingo.` literal, no tool name and no
   session-key prefix, and `scripts/check_discipline.sh` checks
   `apps/desktop/src`.
   - Modes come from the `Mode` slot.
   - A tool row is drawn from its `Row` slot and its `display`.
   - Seats, rosters and `@` targets come from `bingo-collab`'s records,
     whose schema is generated to TS beside the wire's (ADR-0070 §5).
6. **It is the `desktop` surface of ADR-0079.** It draws every element
   in the desktop table, asks `ui/render` for matched components and pulled
   views, docks panes beside the stage, and stacks toasts. It never runs
   mod code.
7. **The design language is Rev 6** (2026-09-03) of
   `../bingo-gui/bingui-gui-design.html`, copied to
   `apps/desktop/design/`. Its shape is a project sidebar, a
   centred stage and a floating composer island, with spring motion,
   glass, and a page-like feel.
8. **Every platform the release ships.** Release tags build the app for
   macOS, Windows and Linux as its own artifact, at the workspace
   version.

## Consequences

- CI grows a node job. `scripts/budget.sh` does not count npm packages,
  so `apps/desktop/package.json` carries a `dependencyReasons` map, one
  line per dependency, and a test fails a dependency without one.
- The wire's consumers are the TUI, print, rpc, channels and the
  desktop app. ADR-0007's "the client generates from the schema" now
  has its client in the tree.
- ADR-0046's client-delegated browser becomes the desktop app's own
  pane, rather than another repository's.
- Rei freezes when M145 lands. Archiving it is the person's call.

Refs: ADR-0002, ADR-0007, ADR-0013, ADR-0046, ADR-0070, ADR-0079;
design/mods-parity-2026-10.md.
Non-goals: a VS Code or mobile surface; a web build served by bingo; a
desktop app that holds session state.
