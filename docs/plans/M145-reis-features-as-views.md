# M145 — Rei's features as views

## Goal

ADR-0080 §4 completes. The parts of `../Rei` worth having come across.
Each is re-homed as a drawing of what the stream already carries
(slots, records, components and `display` views), never as code keyed
on a plugin or tool name. They are:
- the review pane (F10);
- the browser pane (ADR-0046);
- the terminal pane;
- streaming markdown;
- the tool UI, split;
- the sessions, rooms (F08) and agents views.

What Rei's 68 KB `useWorkspace.ts` held arrives as one module per noun.
Rei's end-to-end harness comes with its specs, re-pointed at
`bingo serve` and the fake provider. Rei freezes when this lands;
archiving it is the person's call. After M144.

## Bricks, in build order

1. **The noun map, then the state.** Read `useWorkspace.ts` and list its
   nouns: projects, connections, sessions, selection, drafts, panes,
   preferences (re-verify by reading it). Each noun becomes a module in
   `src/renderer/state/`, and its tests come first: the cases from Rei's
   two `useWorkspace*.test.ts` that describe behaviour the app still
   has. A case that tested `--json-events`, a fake host or a plugin name
   is dropped, with the reason in the commit body.
2. **Streaming markdown.** `markstream-react`, with `katex` and
   `mermaid` (Rei's `media/` modules), draws the `Markdown` element and
   assistant text as it streams. An unclosed fence or table mid-stream
   draws without a jump. Rei's `media.test.tsx` comes, with one
   mid-stream case for each.
3. **The tool UI, split.**
   - `ToolCallFrame` is the chrome: state, duration, expand. Its body is
     the item's placed `Row`, else its `display`, else its text.
   - `ToolActivityGroup` groups consecutive calls by their traits
     (read-only and `quiet`, ADR-0070 §3), not by name.
   - `toolPresentation.ts` and the eight family presenters do not come.
     Whatever one drew that its producer's `display` does not say goes
     into that plugin's `display`, in its own commit (for example,
     Bash's exit status as a `Badge`).
   - Rei's `tool-presenters` and `tool-activity` specs, re-pointed, are
     the test.
4. **Sessions, rooms and agents.**
   - The session list comes from `session/list`. The tree is the sdk's
     tree fold (M120), exported by `bingo-fold-wasm` as `tree()`, so
     Rei's `sessionTree.ts` does not come.
   - Rooms follow F08: a channel list, the conversation, and the `@`
     composer. Rooms and the agents roster read `bingo-collab`'s records
     through M144's readers.
   - A session's kind comes from those records and the tree, never from
     a `rooms/` or `agent/` key prefix.
5. **The review pane (F10).** It docks as a mod's pane does. Main runs
   `git diff` for the project root (Rei's `review/git.ts`), with bounded
   output and `git.exe` resolved on Windows. The pane draws M144's
   `Diff` element. Rei's `ReviewPanel.test.tsx` comes with it.
6. **The browser pane (ADR-0046).**
   - Main spawns `bingo serve` with `BINGO_BROWSER_MODE=client`. Pages
     are hosted in a `WebContentsView` (Rei's `panels/browser.ts`): no
     node integration, one partition per page, and navigation kept to
     the page's origin.
   - The URL arrives typed. `bingo-tool-web`'s page progress places a
     `Link` element (M140), and a login interaction carries its URL as a
     field.
   - Only a loopback `Link` in a running call's progress opens a page by
     itself. Rei's regex over progress text (`agent-browser.ts`) does not
     come.
7. **The terminal pane.** `node-pty` runs in main (Rei's
   `panels/{terminal,terminals,validation}.ts`), and `@xterm/xterm` in
   the renderer. Each tab is one pty in the project root. Closing a tab
   ends its process tree, tested on unix and on ConPTY.
   `prepare-native.cjs` rebuilds `node-pty` for Electron's ABI, per
   platform and arch.
8. **The e2e harness.** `tests/helpers/electron.ts` comes with its test,
   and M143's smoke and M144's spec move onto it. These Rei specs come,
   re-pointed: desktop, multisession, collaboration, tool-activity,
   tool-presenters, media, selection, and workbench-visual (with fresh
   baselines). The rest are listed as dropped in the commit body, each
   with its reason: a fake host, `--json-events`, or covered by the
   release smoke.

## Files

- `apps/desktop/src/renderer/state/<noun>.ts` (one per noun), with tests
- `apps/desktop/src/renderer/media/{Markdown,MarkdownMath,MarkdownDiagram}.tsx`
- `apps/desktop/src/renderer/tools/{ToolCallFrame,ToolActivityGroup}.tsx`
- `apps/desktop/src/renderer/views/{Sessions,Rooms,RoomConversation,Agents}.tsx`
- `apps/desktop/src/renderer/panes/{Review,Browser,Terminal,TerminalTabs}.tsx`
- `apps/desktop/src/main/panes/{review,browser,terminal,terminals,validation}.ts`
- `apps/desktop/src/{preload/panes.ts,shared/panes.ts}`
- `apps/desktop/{scripts/prepare-native.cjs,package.json,package-lock.json}`
- `crates/bingo-fold-wasm/src/tree.rs`, `fixtures/tree.json`
- `crates/bingo-tool-web/src/page.rs` (progress as a `Link`), and each
  producer whose `display` gains what a presenter drew (brick 3)
- `apps/desktop/tests/{helpers/electron.ts,*.e2e.ts}`

## Exit criteria

- [ ] `rg "toolPresentation|sessionTree|json-events" apps/desktop/src`
      finds nothing, and `rg "RegExp|exec\(" apps/desktop/src/main/panes`
      finds no URL match (pasted).
- [ ] ``rg "rooms/|agent/|['\"`]_?bingo\." apps/desktop/src --glob '!**/*.test.*'``
      finds nothing, and the desktop check passes.
- [ ] Each module in `src/renderer/state/` is under 400 lines and has a
      test file (`wc -l` pasted).
- [ ] `npm --prefix apps/desktop test` passes, including the mid-stream
      markdown cases and the tree twin against `fixtures/tree.json`.
- [ ] Every re-pointed spec passes on macOS locally and on CI's ubuntu
      and Windows. On each, the terminal spec closes a tab and finds no
      orphan process.
- [ ] The browser spec opens a page for a loopback `Link` in a fake
      run's progress, and none for the same URL in plain text.
- [ ] `cargo test -p bingo-tool-web` passes, as do the tests of every
      producer touched. All gates of the protocol pass.

## Non-goals

- Onboarding (F01–F04), Settings (F11), and the Skills and Automations
  pages. Rei's versions name the `schedule` catalog; a later plan draws
  them from slots.
- Rei's pencil workspace, scratch workspace, reference export and
  `zh-CN`.
- Editing `../Rei` or archiving it. Rei is frozen once this lands; the
  rest is the person's call.
- A desktop-only kind of view. Anything new comes through a slot, a
  component or an element.

## Risks

- `node-pty` is native. Electron 44's ABI needs a rebuild for each arch,
  and the macOS x64 build is made on arm64. Without a prebuild, CI
  builds it from source on each runner.
- `markstream-react` and `mermaid` are heavy. The renderer bundle's size
  is printed before and after, and `mermaid` loads only when a diagram
  is drawn.
- `WebContentsView` hosts a page someone else wrote. It gets no preload,
  no node and its own partition, as Rei's `security.ts` and its test
  require.
- Rei's state tests total about 116 KB. Porting only what still lives
  can drop a case that mattered, so each drop is named in its commit.
- Changes to a producer's `display` touch plugins, never the kernel, and
  each is its own commit in that crate's scope.
- Re-verify at start: Rei's paths (frozen, read only); M120's tree fold
  and M122/M123's reader names; ADR-0070 §3's traits;
  `bingo-tool-web`'s page progress; the login interaction's URL field;
  `node-pty`'s support for Electron 44.
