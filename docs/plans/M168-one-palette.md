# M168 — One palette

ADR-0084 (`Palette`), ADR-0086 (built-in mod). Needs M167.
Approved prototype (2026-10-14, "命令面板" tab): https://claude.ai/artifact/XWoqqyP8Jch6p5889dL1ZB

## Goal

`/`, `?` and `ctrl+r` open one list above the prompt: commands, keys and sessions, the opener's group first.
Typing searches all three; rows show current values; arguments open in place. `bingo-mod-palette`
replaces completion/help drawing and takes over live navigation without removing it.

## Bricks

1. **Current values.** `Command::now(&SessionView) -> Option<String>` defaults to `None`. Model, thinking, permission, title, tokens and signed-out providers get fixtures and session-scoped metadata.
2. **Pure rows.** Empty query shows the opener's group; search spans commands, keys and sessions. Recent commands lead `/`; hidden commands stay hidden; matched letters are marked. Table tests.
3. **Recent.** Last five distinct command names in plugin storage; never arguments, including on load. Temp-store tests.
4. **Arguments.** `ArgSpec` values open a second level with the current value selected. Free text shows what Enter runs. Escape backs one level; free text remains one level even with spaces. Table tests for each kind.
5. **Acting.** Enter runs commands or bound command lines; sessions return a surface-minted action. Live navigation emits `Effect::View`, stored resume opens a session, and the current row says it is already in view.
6. **Stored sessions.** Read `HostApi::sessions` at the root's cwd, retain roots only, newest 20. Show title, timestamp and message count. Ctrl+R defaults to `/resume`; Ctrl+G returns to a pending ask.
7. **Drawing.** A shared `View` tree above the prompt, at most half the screen: dim headings, one `❯`, right-aligned values and a query footer. Prototype snapshots at 108×30 and 80×24.
8. **Keys.** `/`, `?`, Ctrl+R open; arrows select, Enter acts, Escape backs/closes, typing filters. Shift+Tab still cycles permission. Composite and interrupt hints are dim, informational and skipped.
9. **Navigation, preserved.** Delete old argument/help/switcher drawing, not navigation. “This conversation” comes first: root, children and rooms, with status, unread and owed; the current view is marked. The crew strip opens this group. Stored roots follow. Keep native names-only navigation when disabled. Review correction: the original brick wrongly treated live navigation and the roster as disposable drawing.

## Files

- SDK `command.rs`, `component/{palette.rs,props.json}`, `render.rs`; core `host{,/catalog}.rs`,
  `session/inputs.rs`, `commands/{model,think,rename,compact,login}.rs`; permissions `command.rs`.
- New `bingo-mod-palette` crate, its registered guide page and workspace/binary registration.
- TUI palette, input, keys, focus/layout/view, crew pointer, tree/seats, run/site-key routing and tests;
  remove old command argument/help/switcher/roster drawing. Keep `complete.rs` for `@` names/paths.
- Mods built-in integration, CLI palette tests, print receipt handling, chat menu, snapshots and `scripts/tui-smoke.sh`.
- `schema/`, `sdk/typescript/`; guides belong to the owning crate's registered `Page` (ADR-0054), not `docs/guide/`.

## Exit criteria

- [x] Row/argument/recent tables and current-value fixture; every prototype scene at both sizes.
- [x] TestBackend openers, cross-group search, resume, Escape back and Shift+Tab without opening.
- [x] Outside JS command/keys appear; another mod wraps `Palette` with `next`.
- [x] Disabled `bingo.mod.palette`: typed commands, `?`, Ctrl+R and `/resume` work with names-only fallback.
- [x] Conversation roster, child/root routing, retained transcripts, store fill and picture-overlay tests.
- [x] Ask overlap, narrow-filter replacement, aliases, inert key rows and recent-secret regressions.
- [x] Isolated tmux/temp HOME captures match the prototype; tui-smoke and native gates pass.
- [ ] Windows all-target checks: SDK/core/palette pass; other touched crates await C toolchains (Verified).

## Non-goals

New mouse interactions, fuzzy scoring beyond subsequence matching, transcript search and the desktop palette.

## Risks

- Changed muscle memory: claim one startup toast in plugin data naming Ctrl+R.
- Current values must remain cheap: borrowed state/provider facts, no I/O; deadline-free fake-host tests.

## Decisions

- Global catalogues have no session. Typed `CommandCatalog` overlays `now` using a caller-supplied borrowed
  `SessionView` and provider auth facts through the existing service door. No new HostApi or hidden active session.
- The surface projects compact display facts directly from its fold; no partial `SessionState` in the mod.
  Model IDs remain qualified `provider/model`. Stored rows use actual message counts and are capped before props.
- Surface actions return in the existing intent acknowledgement. M163 FIFO remains the action path;
  M169 question/form keys retain their bypass. Opening IDs and query-matched replies reject stale results.
- The topmost palette owns keys even when an ask arrives after 900 ms; closing it restores ask focus.
  Waiting uses the same render predicate as drawing, so a narrower replacement cannot queue keys forever.
- Static surface hints join real M167 bindings; no prototype-only bindings are invented. Aliases are searchable.
  Exact names and name matches precede descriptions: PTY smoke caught `/board` selecting `/agents`.
- Common prototype commands lead registration order so model/thinking controls stay on the first screen.
- Specs lack a surface restriction: terminal-family commands stay out of chat menus and carry a one-line
  fallback in their receipt. Print considers that receipt complete; TUI consumes its supported action.
- Refresh Plugins once after acknowledgements until plugin switches emit `CatalogChanged`; other catalogues follow that event.
  Current-value lookup is linear. Ctrl+C retires the owner through the action FIFO, including late render replies.
- Only workspace edges are added; dependency budget grows 413 → 414 (ADR-0086).

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

## Verified — 2026-10-15 (review fixes)

Rebased onto dev `dbdc5952`; the ready-provider fixture first failed with E0533, then passed.
Red runs before fixes: focus 2 failed; resume/props 3; minor rows/recents 4; linear lookup 1;
child routing 1; catalogue refetch 1. All pass in the final suite, including restored roster tests.
Sequential native gate output summaries (all exit 0):
```text
cargo fmt --all -- --check: no output
cargo check --workspace --all-targets --locked: Finished dev profile (4.49s)
cargo clippy --workspace --all-targets --locked -- -D warnings: Finished dev profile (5.10s)
RUST_TEST_THREADS=4 cargo test --workspace --locked: 5644 passed; 0 failed; 5 ignored (aggregate)
bash scripts/check_discipline.sh: discipline ok
bash scripts/budget.sh: 414/414 dependencies; core 0s; isolation 0; budget ok
cargo deny check: advisories ok, bans ok, licenses ok, sources ok
bash scripts/tui-smoke.sh: tui-smoke ok
cd sdk/typescript && npm test: tests 31; pass 31; fail 0
```
`BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk --locked`: 196 passed.
The same flag with `cargo test -p bingo-plugin-rpc -p bingo-surface-rpc -p bingo-store-jsonl
-p bingo-mods --locked --lib`: 267 passed, 1 ignored. Schema/TypeScript regenerated.
TUI: 1145 passed, 2 ignored; palette mod: 14 passed. Fresh isolated tmux/temp HOME captures:
16 scenes at 108×30/80×24, inspected for layout, wording, selection and current values.
Windows command: `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc --locked`.
Exit 0: bingo-{sdk,core,mod-palette}. Exit 101: bingo-{channels,surface-tui,surface-print}
in aws-lc-sys (missing Windows headers); bingo and bingo-mods in aws-lc-sys/rquickjs-sys
(MSVC flags rejected by clang); bingo-permissions in tree-sitter/tree-sitter-bash
(missing stdio.h/stdlib.h). Those six Rust checks remain deferred; the criterion stays unticked.
The first workspace run failed in M165's local origin fixture: inherited `OPENAI_BASE_URL`
redirected its dummy key outside the mock and returned AUTH_REQUIRED. No real key was passed.
A command-environment regression failed before the fixture cleared endpoint variables, then passed;
the origin test passed against its local server. Final gates/captures also cleared provider key
and endpoint variables. This was not a timing flake; no wall-clock retry was needed.
Budget warned about 12 GB of debug output. Final cleanup is `cargo clean` after this record's commit.

## Verified — 2026-10-15 (final small round)

- Plugins: two regressions failed before restoring its single acknowledgement refresh; all nine palette routing tests pass afterward. Reload updates the waiting count; no core plugin event changes.
- Ctrl+C: two run-loop regressions failed before the fix, then passed. Idle drafts survive; busy turns interrupt; real mod commands clear cursors, including late render replies, through the FIFO.
- Shared bin fixture: its environment regression failed before isolation, then passed 20 times across 20 bin test targets. Sync, Tokio, PTY and copied-binary launchers clear both providers' key/base-URL variables.
Requested gates ran sequentially with provider variables unset and `RUST_TEST_THREADS=4` (all exit 0):
```text
cargo fmt --all -- --check: no output
cargo clippy --workspace --all-targets --locked -- -D warnings: Finished dev profile (6.09s)
cargo test -p bingo-surface-tui -p bingo-mod-palette -p bingo --locked: 1622 passed; 0 failed; 2 ignored
bash scripts/check_discipline.sh: discipline ok
bash scripts/tui-smoke.sh: tui-smoke ok
```
The smoke used its own isolated tmux socket and temporary HOME/data. No timing-flake retry was needed.
Final cleanup is `cargo clean` after committing this verification record.
