# M175 — Sessions in one list

ADR-0089, ADR-0084, ADR-0086, ADR-0010. Needs M167, M168 and M174. Inline rename needs no M170 Composer action.

## Goal

One list of every session, found by what was asked. `/sessions` and bare `bingo --resume` open the `Sessions` site: rows with title, first ask, `2h ago`, messages, busy or asking, children as a tree under their parent; `tab` toggles this directory and all; a `more` row instead of a cap; typing filters title and first ask. `⏎` resumes, `→` previews the last exchange, `r` renames in place, `d` deletes after a confirmation (children first), `f` forks at the last turn. A session is titled from its first ask unless somebody titled it. The feature is the built-in mod `bingo-mod-sessions`; the palette keeps its twenty recents and gains an `All sessions…` row.

```
 sessions · here (tab: all) · fix                              3 of 41
❯ fix the stale-edit race          2h ago   38 msg  ● asking
    └ review: action FIFO          2h ago   12 msg
  fixture for checkpoint v2        1d ago   20 msg
    "can you add a fixture for…"   dim: first ask when not the title
  schedule (4)                     ▸ collapsed: schedule/<id> sessions
  more…                            50 at a time
 ─────────────────────────────────────────────────────────────────
 ⏎ resume  → preview  r rename  d delete  f fork  tab here/all  esc
```

## Bricks

1. **One relative-time spelling.** Pure `bingo_sdk::ago(now, then) -> String` in new `crates/bingo-sdk/src/ago.rs` (precedent `bytes.rs`): `just now`, `4m ago`, `2h ago`, `3d ago`, a date past 30 days. Table test incl. a future `then` (clock skew → `just now`). `crates/bingo-surface-tui/src/welcome.rs:159` calls it instead of `worked::duration(age) + " ago"`; its snapshot moves once.
2. **What a person asked.** Pure `bingo_sdk::asked(&Item) -> Option<&str>`: full Said text; first-ask consumers take its first line. Shell output and wake notes are not Said input. Table test lifted from `bingo-surface-tui/src/rewind.rs` (`a_shell_line_is_not_…`).
3. **First ask on the summary.** `SessionSummary.history.first_ask: Option<String>` (`crates/bingo-sdk/src/event.rs`), ≤200 chars, set once where `messages` is counted (`crates/bingo-sdk/src/state.rs`, `crates/bingo-store-jsonl/src/summary.rs`). Fixture first under `crates/bingo-sdk/fixtures/record/compat/`; an old summary reads `None`.
4. **Fork, contract first.** `HostApi::fork(session, at: Option<TurnId>) -> SessionId` (defaulted `Internal` in `crates/bingo-sdk/src/host.rs`); `ForkLink { session, turn }` on `SessionSummary` only (summary `history.forked_from`, flattened as `forkedFrom`). Fixture of a forked summary and of `session/fork` request/response before the implementation.
5. **The cut, pure.** `crates/bingo-core/src/fork.rs::kept(frames, at)`: frames through the durable `TurnCompleted` of `at`, ids kept, session rewritten. Amendment: `rewind::dropped` cuts *before* a turn and drops event-only history; a fork cuts *after* the completion and retains rewind records verbatim. Table test: whole, mid, unknown turn (`INVALID_INPUT`), a turn with a `Rewind` item in it.
6. **Fork in core and on the wire.** `crates/bingo-core/src/host.rs` `fork`: refused `NOT_READY` while the source runs a turn; creates the journal through `SessionStore::create` + `append`; children not copied. `crates/bingo-surface-rpc/src/methods.rs` gains `session/fork`. Black-box RPC test: fork, list, the fork is a root with `forkedFrom`.
7. **Auto-title, pure + link.** Pure `title_from(ask) -> Option<String>` in `crates/bingo-mod-sessions/src/title.rs` (first line, ≤60 chars at a word, `None` for a `/command` or `!shell` line). A `prompt.submit` link in the mod writes `SessionChange::Title` only when the summary's `title` is `None`. Targeted `HostApi::summary` avoids a store scan; lookup/write errors are logged and never reject admission. Tests: titled once, never over `/rename`, failure preserves accepted input.
8. **Rows, pure.** `crates/bingo-mod-sessions/src/rows.rs::rows(summaries, now, query, current) -> Vec<Row>`: tree by `parent`, keyed sessions grouped by the key's first segment (`schedule/…`, channel adapters) into one collapsed row, filter on title and first ask (case-folded substring), first ask dim only when it differs from the title, `⑂ from <source title>` on forks. Selection stores a Target and each row has a stable control key. Table test per sketch row.
9. **Props.** `Sessions { session, cwd, query, scope, limit, sessions, current, now, action }` in `crates/bingo-sdk/src/component/{sessions.rs,props.json}` and the `components!` list; golden contract test; schema and TypeScript regenerated. The surface lists with `SessionFilter { cwd (here) or none, limit }`; `more` raises `limit` by 50.
10. **Keys and verbs.** `crates/bingo-mod-sessions/src/keys.rs` through `onKey`: `↑↓`, `tab`, `⏎` → `{"resume":id}`, `→` → read-only preview (last person line and following answer; the mod holds it, render does no I/O), `r` → inline title input, Enter saves by id, Escape cancels, `d` → confirmation naming children, then one `HostApi::delete` for the root, `f` → `fork` then resume the fork. Table test (row, key) → outcome.
11. **Drawing.** `crates/bingo-mod-sessions/src/draw.rs`: the sketch, at most the screen's height, one `❯`; snapshots at 108×30 and 80×24.
12. **Openers.** `/sessions` in the mod; bare `--resume` in `crates/bingo/src/main.rs:127-134` becomes optional-value and opens the site (an error under `--print` or with a positional prompt, black-box tests); remove its empty bootstrap when another session is resumed; the palette's sessions group (`crates/bingo-surface-tui/src/palette/sessions.rs`) ends with `All sessions…` running `/sessions` and its detail uses `ago` instead of raw `updated_at`.
13. **Guide.** `guide.md` (<200 lines) registered by `guide.rs`, as in `bingo-mod-palette`.

## Files

- sdk: `ago.rs`, `event.rs`, `state.rs`, `host.rs`, `lib.rs`, `component/{sessions.rs,props.json,props.rs}`, `component.rs`, `render.rs`; fixtures; `schema/`, `sdk/typescript/`.
- core `host.rs`, new `fork.rs`; `bingo-store-jsonl/src/summary.rs`; `bingo-surface-rpc/src/methods.rs` and its black-box tests.
- tui: `welcome.rs`, `palette/sessions.rs`, site placement for `Sessions`.
- New `crates/bingo-mod-sessions/src/{lib.rs,title.rs,rows.rs,keys.rs, preview.rs,draw.rs,guide.rs,guide.md}`; listed in `crates/bingo/src/main.rs`; one `scripts/budget.toml` line (+1 member, no external dependency); ADR-0084 gains its amendment line. Existing async-trait/serde/serde_json/jiff/tokio/tracing edges serve contracts, timestamps, preview and best-effort title diagnostics; TUI and built-in parity tests depend on the mod.

## Exit criteria

- [x] `ago`, `asked`, `title_from`, `kept`, `rows`, keys tables pass.
- [x] Fixtures: summary with `firstAsk` and `forkedFrom`; old summary without either; `session/fork` request/response; `Sessions` props.
- [x] RPC black-box: `session/fork` on a two-turn session; refused while a turn runs; NDJSON stdout pure.
- [x] `TestBackend`: tab toggles scope; `more` grows the list; typing filters; `d` on a parent names its children and deletes them first; `f` resumes the fork; `r` edits inline without switching; schedule rows fold.
- [x] `bingo --resume` opens the site; `--print --resume` fails, 1 line.
- [x] `enabledPlugins: {"bingo.mod.sessions": false}`: `/sessions` says it is off, `ctrl+r` and the palette work as before.
- [x] tmux drive on an isolated socket, temp HOME, fake provider: three sessions, one child, one fork; captures pasted.
- [x] `bash scripts/tui-smoke.sh`; `cargo fmt --all -- --check`; `cargo check --workspace --all-targets --locked`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo test --workspace --locked`; `scripts/check_discipline.sh`; `scripts/budget.sh`; `cargo deny check`.
- [ ] Windows checks for every touched crate; record any C-toolchain deferrals below.

## In-flight dependencies

- M166 (model picker): none directly; a fork keeps the source's model.
- M170 (status hub): no dependency; the superseded Composer action is removed.
- M173 (plugins panel): the mod's switch row and `off now`.
- M174 (stale-edit, action FIFO): `⏎`/`f` resolve against the current drawing (39f6f91c); queued site keys stay outside the FIFO (ADR-0084 §3).

## Non-goals

Archive; content search across directories; forking a child or a room.

## Risks

- Read-only preview folds stored history on demand; it runs off the keyboard path and shows `loading…`.
- Delete reserves the entire set before mutation. I/O failure during removal is reported; this is not a filesystem transaction.

## Decisions and amendments

- Remove the earlier core auto-title mint: feature ownership belongs to this mod, including the 60-character first-line rule. Disabled sessions means no auto-title. The submit link runs after admission and ignores commands, shell and note voices.
- Pagination reads `limit + 1`: the extra summary is a sentinel, never drawn. Counts say `of 50+` until exhausted; an exact total cannot be claimed from a page. Filtering covers loaded summaries; `more` remains available even with no matches.
- Letter verbs apply only with an empty filter. `/` enters search mode so a query beginning with r/d/f is possible; Escape leaves that mode before closing the site.
- Props add the minute-rounded clock and the client's minted action, as Palette does: deterministic relative dates and replies tied to the current opening, no render I/O.
- A live fork uses an actor-fixed sequence; a stored fork reads only through a completed turn. Foreign mid-turn sources refuse; an unowned incomplete tail is excluded. Root model sessions only; no completed stored turn means no fork.
- Delete confirms the full stored descendant set in postorder, refuses the currently viewed conversation, and uses the host delete verb. Preview retains no attachment.
- Guides use `crates/bingo-mod-sessions/src/guide.md` registered as a `Page`.
- Reserve the right-hand detail column before truncating titles; an 80-column terminal must retain age, count and asking/busy. Untitled rows use the final directory segment.
- Discipline exposed 18 summary fields against the 16-field bound. Group `messages`, `first_ask` and `forked_from` in `SessionHistory`, flattened by serde: the JSON contract stays additive and old records still default. Rust callers use `summary.history`; fixture round trips pin the unchanged wire shape. Host helpers are free functions, keeping inherent implementations within three files.

## Review amendments — 2026-10-15

Review found that revival appended open-hook records and retained locks; stored operations must bypass actors.

- Stored reads and forks replay without revival; stored title writes take a brief claim and append one summary, preserving activity time. Add targeted host summary/read and delete-check contracts with fail-closed defaults; no plugin bridge expansion.
- Delete preflights the entire descendant set and retains claims until removal. Busy or foreign-locked rows are labelled and block confirmation.
- Selection is a stable Target, rows have keyed plain buttons (nested onKey is invalid), missing targets never redirect a verb. Rename edits inline and leaves the viewed session unchanged. Empty, untitled bare-resume bootstrap sessions are removed on switching; ordinary or nonempty sessions remain.
- Auto-title looks up only its session and logs errors without rejecting admitted input. Shared `asked` returns borrowed full Said text from the first text part (remaining parts are attachments); first-ask consumers take its first line.
- Provenance is summary-only; Create cannot supply it. Consolidate remaining age formatters, fix labels, add fork-origin markers, reject bare-resume plus a prompt, and cover the mod in built-in parity tests.
- Rebuild commits with schema regeneration beside its owning code; verify each build and its golden tests. The UI commit must not change core fork implementation.

## Verified — 2026-10-15

Contract-first: the original fork fixture failed with six missing field/method errors before implementation; the stored-inspection fixture likewise failed before adding read/summary/rename. Both now pass. Against dev, SessionSpec is unchanged, no summary property is removed, and only optional firstAsk/forkedFrom are added.

Rebuilt history: `f87bb19c` independently passed `cargo build --workspace --all-targets --locked` (1m 07s) and `cargo test -p bingo-sdk -p bingo-surface-rpc -p bingo-plugin-rpc -p bingo-mods --locked` (all passed). `28f7574a` independently built (2m 43s); its golden tests passed in the full run below. The UI commit has no core changes. The final documentation tree also built (10.93s) and passed the same four-crate golden-test command. Regeneration used `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk --locked`, then the surface-rpc/plugin-rpc/mods library generator tests; TypeScript generators ran with these tests.

Sequential gates, credentials removed, RUST_TEST_THREADS=4; output summaries:
```text
cargo fmt --all -- --check: exit 0
cargo check --workspace --all-targets --locked: Finished dev in 1m 06s
cargo clippy --workspace --all-targets --locked -- -D warnings: Finished dev in 54.96s
cargo test --workspace --locked: 175 suites; 5766 passed; 0 failed; 5 ignored
bash scripts/check_discipline.sh: discipline ok
bash scripts/budget.sh: dependencies (unique, normal): 415 (max 415); budget ok
cargo deny check: advisories ok, bans ok, licenses ok, sources ok
SHELL=/bin/sh bash scripts/tui-smoke.sh: tui-smoke ok
cd sdk/typescript && npm test: tests 31; pass 31; fail 0
```
Budget reports 14 GB target/debug against its 5 GB soft limit; final cargo clean removes build output. Discipline advisories remain warnings. One earlier full run failed the existing ACP bridge test `a_bridged_call_posts_to_the_parent_and_is_journaled_under_the_turn` on missing “the member spoke” output under load; unchanged isolated rerun passed (1 passed, 0 failed, 0.66s), and subsequent full runs passed. The final run had no failures.

Windows: ran `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc --locked` separately for all 20 touched crates. Fifteen passed: agents, checkpoints, context, core, demo-ui, experience, mod-palette, mod-sessions, persona, rooms, sdk, skills, store-jsonl, surface-rpc, tasks (all bingo-prefixed). Five remain deferred: bingo, channel-adapter, mods, surface-print, surface-tui; each failed building aws-lc-sys with `fatal error: 'stdlib.h' file not found` / `'windows.h' file not found`. This machine lacks the Windows C toolchain; the Windows criterion stays unticked.

Isolated tmux, temporary HOME/data, fake provider; captures below show the complete list region. 108×30:
```text
 sessions · here (tab: all) ·                                                                         5 of 6
❯ cwd · current                                                                             just now   0 msg
  fixture for checkpoint v2                                                                 just now   2 msg
     "can you add a fixture for checkpoint v2?"
  fix the stale-edit race                                                                   just now   2 msg
    └ review: action FIFO                                                                   just now   2 msg
       "review the action FIFO"
  fix the stale-edit race                                                                   just now   2 msg
     ⑂ from fix the stale-edit race
  schedule (1)                                                                                             ▸
────────────────────────────────────────────────────────────────────────────────────────────────────────────
 ⏎ resume  → preview  r rename  d delete  f fork  tab here/all  esc
```
80×24 after inline rename (same selected row, list remains open):
```text
 sessions · here (tab: all) ·                                             5 of 6
  cwd · current                                                 just now   0 msg
❯ renamed inline                                                just now   2 msg
     "can you add a fixture for checkpoint v2?"
  fix the stale-edit race                                       just now   2 msg
    └ review: action FIFO                                       just now   2 msg
       "review the action FIFO"
  fix the stale-edit race                                       just now   2 msg
     ⑂ from fix the stale-edit race
  schedule (1)                                                                 ▸
────────────────────────────────────────────────────────────────────────────────
 ⏎ resume  → preview  r rename  d delete  f fork  tab here/all  esc
```
Drive assertions: `preview: source bytes and mtimes unchanged`; `rename: one record, mtime unchanged, list stays open`; `resume: empty bootstrap removed`. TestBackend golden captures cover both complete frames; integration tests also pin zero open hooks, released claims, foreign-lock refusal, busy deletion, stable targets, title error admission, built-in parity and NDJSON purity.
