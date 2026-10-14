# M168 — One palette

ADR-0084 (`Palette`), ADR-0086 (built-in mod). Needs M167.
Approved prototype (2026-10-14, "命令面板" tab):
https://claude.ai/artifact/XWoqqyP8Jch6p5889dL1ZB

## Goal

`/`, `?` and `ctrl+r` open one list grown up from the prompt: commands,
keys and sessions in three groups, the opener's group first. Typing
searches all three; each row says its current value; picking a command
with arguments steps into its arguments in the same list. The feature
is the built-in mod `bingo-mod-palette`; the terminal's completion
menu, `?` help and `ctrl+g` switcher are deleted.

## Bricks

1. **A command's current value.** `Command::now(&self, &SessionView)
   -> Option<String>` (sdk, default `None`), surfaced as `now` in
   `catalog(Commands)` meta: `/model` the model, `/think` the effort,
   `/permission` the mode, `/rename` the title, `/compact` tokens used,
   `/login` how many providers are signed out. Key-set test, fixture.
2. **Rows, pure.** `rows(opened, query, commands, bindings, sessions,
   recent) -> Vec<Group>`: empty query shows the opener's group (`/`:
   recent then commands; `?`: keys; `ctrl+r`: sessions); a query
   searches all three, the opener's group first, matched letters
   marked. Hidden commands never listed. Table tests.
3. **Recent.** The mod keeps the last five commands run from the
   palette in its plugin store; `recent` reads them. Test with a
   temp store.
4. **Arguments in place.** A command whose `ArgSpec` lists values opens
   a second level: the values, the current one selected (`now`); a free
   text argument (`/rename`) shows an input row with what ⏎ will do.
   `esc` goes back one level. Table tests over `ArgSpec` kinds.
5. **Acting.** ⏎ on a command runs it (with its argument); on a key
   runs its bound command line (M167 bindings); on a session asks the
   surface to open it (a handler the surface mints; ADR-0086
   Consequences). Opening the session already in view says so.
6. **Sessions.** `HostApi::sessions` for this workspace, newest first:
   title, last time, turns, crew glyphs. `ctrl+r` is the default key of
   `/resume`; `ctrl+g` is freed.
7. **Drawing.** `Palette` site as a `View` tree: group headers dim,
   one `❯` row, the value right-aligned, a footer line for the query
   hint; grows up from the prompt, at most half the screen. Snapshot
   per prototype scenario at 108×30 and 80×24.
8. **Keys.** `/` or `?` on an empty prompt and `ctrl+r` open it;
   `↑↓` move, `⏎` act, `esc` back or close, typing filters (never acts).
   `shift+tab` still cycles the permission mode without opening it.
9. **Deletions.** `complete.rs`'s command menu, the `?` help view, the
   switcher (`input/switcher.rs`, `view.rs` parts) and the TUI-local
   `/help` and `/resume` go; `/help` becomes a palette command.

## Files

- `crates/bingo-sdk/src/command.rs`; `crates/bingo-core/src/host/
  catalog.rs`, `commands/{model.rs,think.rs,rename.rs,compact.rs,
  login.rs}`; `crates/bingo-permissions/src/command.rs`.
- New `crates/bingo-mod-palette/src/{lib.rs,rows.rs,args.rs,
  recent.rs,draw.rs,keys.rs}`; listed in `crates/bingo/src/main.rs`.
- `crates/bingo-surface-tui/src/{complete.rs,view.rs,input.rs,
  input/switcher.rs,commands.rs,keys.rs}` (deletions, session handler).
- `schema/`, `sdk/typescript/`, `docs/guide/`.

## Exit criteria

- [ ] Table tests for rows, args and recent; `now` fixture.
- [ ] Snapshots per scenario (`/`, `?`, `ctrl+r`, `/mo`, `/think`
      args, `/rename`, `/permission` args) at both sizes.
- [ ] `TestBackend`: each opener, search across groups, resume a
      session, `esc` back a level, `shift+tab` without the list.
- [ ] An outside JS mod adds a command and it is listed with its keys;
      another wraps `Palette` with `next`.
- [ ] `bingo-mod-palette` switched off: `/` still runs typed commands
      (the surface's fallback lists names only).
- [ ] tmux drive in a temp HOME, captured and compared with the
      prototype; tui-smoke; gates as M167.

## Non-goals

Mouse; fuzzy ranking beyond subsequence; searching transcript text
(browse, later); the desktop palette.

## Risks

- Deleting the old menu breaks muscle memory for `ctrl+g`: the first
  run after the change shows one toast naming `ctrl+r`.
- `now` must stay cheap: it reads session state only, never I/O; a
  test runs every command's `now` under a deadline-free fake host.
