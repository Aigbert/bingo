# M168 — One palette

ADR-0084 (`Palette`), ADR-0086 (built-in mod). Needs M167.
Approved prototype (2026-10-14, "命令面板" tab):
https://claude.ai/artifact/XWoqqyP8Jch6p5889dL1ZB

## Goal

`/`, `?` and `ctrl+r` open one list above the prompt: commands, keys and
sessions, the opener's group first. Typing searches all three; rows show current
values; command arguments open in place. The built-in `bingo-mod-palette`
replaces the terminal's completion menu, `?` help and `ctrl+g` switcher.

## Bricks

1. **A command's current value.** `Command::now(&self, &SessionView)
   -> Option<String>` (sdk, default `None`), surfaced as `now` in
   session-scoped Commands metadata (see Decisions): `/model` the model, `/think` the effort,
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
   title, last time, message count (see Decisions). `ctrl+r` is the default key of
   `/resume`; `ctrl+g` only returns to a pending ask.
7. **Drawing.** `Palette` site as a `View` tree: group headers dim,
   one `❯` row, the value right-aligned, a footer line for the query
   hint; grows up from the prompt, at most half the screen. Snapshot
   per prototype scenario at 108×30 and 80×24.
8. **Keys.** `/` or `?` on an empty prompt and `ctrl+r` open it;
   `↑↓` move, `⏎` act, `esc` back or close, typing filters (never acts).
   `shift+tab` still cycles the permission mode without opening it.
9. **Deletions.** `commands.rs`'s argument menu, the `?` help view, the
   switcher (`input/switcher.rs`, `view.rs` parts) and the TUI-local
   `/help` and `/resume` go; `/help` becomes a palette command.

## Files

- `crates/bingo-sdk/src/command.rs`; `crates/bingo-core/src/host/
  catalog.rs`, `commands/{model.rs,think.rs,rename.rs,compact.rs,
  login.rs}`; `crates/bingo-permissions/src/command.rs`.
- New `crates/bingo-mod-palette/src/{lib.rs,rows.rs,args.rs,
  recent.rs,draw.rs,keys.rs,hints.rs,lifecycle.rs}`; listed in `crates/bingo/src/main.rs`.
- `crates/bingo-surface-tui/src/{complete.rs,view.rs,input.rs,
  input/switcher.rs,commands.rs,keys.rs}` (deletions, session handler).
- `schema/`, `sdk/typescript/`, `crates/bingo-mod-palette/src/{guide.rs,guide.md}`.
- SDK `component/palette.rs`, `render.rs`; core `session/inputs.rs`, `host.rs`;
  TUI `palette.rs`, `palette/tests.rs`, `run/{palette.rs,site_keys.rs,palette_tests.rs}`,
  layer/input/key routing and deletion of `roster.rs`; binary/workspace registration.
- `crates/bingo-mods/tests/builtins.rs`, `crates/bingo/tests/cli/palette.rs`,
  `bingo-surface-print/src/lib.rs`, TUI snapshots and `scripts/tui-smoke.sh`.

## Exit criteria

- [x] Table tests for rows, args and recent; `now` fixture.
- [x] Snapshots per scenario (`/`, `?`, `ctrl+r`, `/mo`, `/think`
      args, `/rename`, `/permission` args) at both sizes.
- [x] `TestBackend`: each opener, search across groups, resume a
      session, `esc` back a level, `shift+tab` without the list.
- [x] An outside JS mod adds a command and it is listed with its keys;
      another wraps `Palette` with `next`.
- [x] `bingo-mod-palette` switched off: `/` still runs typed commands
      (the surface's fallback lists names only).
- [x] tmux drive in a temp HOME, captured and compared with the
      prototype; tui-smoke; native gates as M167.
- [ ] Windows all-target checks for every touched crate: C prerequisites block
      five crates (see Verified); the SDK, core and palette pass.

## Non-goals

Mouse; fuzzy scoring beyond subsequence matching; searching transcript text
(browse, later); the desktop palette.

## Risks

- Deleting the old menu breaks muscle memory for `ctrl+g`: the first
  run after the change shows one toast naming `ctrl+r`.
- `now` must stay cheap: it reads session state only, never I/O; a
  test runs every command's `now` under a deadline-free fake host.

## Decisions

- `SessionView` is a new borrowed view of the existing `SessionState` and provider
  auth facts: login cannot derive provider status from a session fold alone.
  The global catalogue has no session parameter. A typed `CommandCatalog` service
  overlays `now` on that catalogue for a supplied state, through the existing service
  door. No HostApi method is added and no arbitrary live session is chosen.
- Session summaries have messages, not turns or crew glyphs; show their actual count.
- Guide pages live in the owning crate and are registered through `Pages` (ADR-0054).
- Only workspace edges are new; member budget grows 413 → 414 (ADR-0086).
- The session-open callback travels back in `CommandOutcome::Surface` on the
  existing intent acknowledgement, not through a new HostApi door. Site keys use
  the M163 FIFO; M169 question/form keys retain their bypass. Query-matched render
  replies and opening IDs prevent stale keys or session-list results from acting.
- Static surface key hints join actual M167 command bindings. Existing controls
  remain truthful (no prototype-only model-cycle bindings are invented).
- Recent stores names only, never arguments; the migration toast is claimed once
  in the plugin data directory. A free-text argument is one level even with spaces.
- Palette props include folded mode signals; no separate mode cache. Model current
  values use the catalogue's qualified `provider/model` ID to select the right row.
- `complete.rs` owns `@` names/paths, so it remains. The command argument menu was
  in `commands.rs`; that is deleted, retaining the specified names-only fallback.
- Exact names, then name matches, precede contiguous description matches. The PTY
  smoke caught `/board` selecting `/agents` through a loose description match.
- Print treats a client-action acknowledgement as a completed command receipt;
  the black-box CLI test caught a headless `/help` waiting for a nonexistent turn.
- The prototype's common commands lead the raw registration order; extension
  commands follow. Otherwise the first screen can hide model and thinking controls.

## Verified — 2026-10-15

Sequential native gates, all exit 0 (output summaries):
```text
cargo fmt --all -- --check: no output
cargo check --workspace --all-targets --locked: Finished dev profile
cargo clippy --workspace --all-targets --locked -- -D warnings: Finished dev profile
cargo test --workspace --locked: 5533 passed; 0 failed; 5 ignored (aggregate)
bash scripts/check_discipline.sh: discipline ok
bash scripts/budget.sh: 414/414 dependencies; core 0s; isolation 0; budget ok
cargo deny check: advisories ok, bans ok, licenses ok, sources ok
bash scripts/tui-smoke.sh: tui-smoke ok
cd sdk/typescript && npm test: tests 31; pass 31; fail 0
```
`BINGO_UPDATE_SCHEMA=1` generator tests: SDK 193 passed; plugin-rpc, surface-rpc,
store-jsonl and mods libraries 267 passed, 1 ignored. Schema and TypeScript regenerated.
Isolated tmux/temp HOME: 16 captures at 108×30/80×24 compared with the prototype
(the seven required scenes plus model arguments); startup migration toast verified.
Windows command: `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc --locked`.
Exit 0: bingo-{sdk,core,mod-palette}. Exit 101: bingo-{surface-tui,surface-print}
stop in aws-lc-sys (missing Windows headers); bingo and bingo-mods also stop in
rquickjs-sys (MSVC flags rejected by clang); bingo-permissions stops in
tree-sitter/tree-sitter-bash (stdio.h/stdlib.h absent). Rust checking remains deferred.
No timing-flake retry was needed in the final run. Budget warned at 10 GB of debug
output; final cleanup is `cargo clean`. No live/credential tests were run.
