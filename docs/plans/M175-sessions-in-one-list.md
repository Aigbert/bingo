# M175 — Sessions in one list

ADR-0089, ADR-0084, ADR-0086, ADR-0010. Needs M167, M168; M170 for the
`/rename` prefill and the generic `Composer` surface action.

## Goal

One list of every session, found by what was asked. `/sessions` and bare
`bingo --resume` open the `Sessions` site: rows with title, first ask,
`2h ago`, messages, busy or asking, children as a tree under their parent;
`tab` toggles this directory and all; a `more` row instead of a cap;
typing filters title and first ask. `⏎` resumes, `→` previews the last
exchange, `r` renames in place, `d` deletes after a confirmation
(children first), `f` forks at the last turn. A session is titled from
its first ask unless somebody titled it. The feature is the built-in mod
`bingo-mod-sessions`; the palette keeps its twenty recents and gains an
`All sessions…` row.

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

1. **One relative-time spelling.** Pure `bingo_sdk::ago(now, then) ->
   String` in new `crates/bingo-sdk/src/ago.rs` (precedent `bytes.rs`):
   `just now`, `4m ago`, `2h ago`, `3d ago`, a date past 30 days.
   Table test incl. a future `then` (clock skew → `just now`).
   `crates/bingo-surface-tui/src/welcome.rs:159` calls it instead of
   `worked::duration(age) + " ago"`; its snapshot moves once.
2. **What a person asked.** Pure `bingo_sdk::asked(&Item) ->
   Option<String>`: a person-role message's first line; a shell line or a
   wake note is not one. Table test lifted from
   `bingo-surface-tui/src/rewind.rs` (`a_shell_line_is_not_…`).
3. **First ask on the summary.** `SessionSummary.first_ask:
   Option<String>` (`crates/bingo-sdk/src/event.rs`), ≤200 chars, set
   once where `messages` is counted (`crates/bingo-sdk/src/state.rs`,
   `crates/bingo-store-jsonl/src/summary.rs`). Fixture first under
   `crates/bingo-sdk/fixtures/record/compat/`; an old summary reads `None`.
4. **Fork, contract first.** `HostApi::fork(session, at: Option<TurnId>)
   -> SessionId` (defaulted `Internal` in `crates/bingo-sdk/src/host.rs`);
   `ForkLink { session, turn }` on `SessionSpec` and `SessionSummary`
   (`forked_from`). Fixture of a forked summary and of `session/fork`
   request/response before the implementation.
5. **The cut, pure.** `crates/bingo-core/src/fork.rs::kept(frames, at)`:
   frames through the end of `at`, ids kept, session rewritten; reuse
   `rewind::dropped` for the boundary. Table test: whole, mid, unknown
   turn (`INVALID_INPUT`), a turn with a `Rewind` item in it.
6. **Fork in core and on the wire.** `crates/bingo-core/src/host.rs`
   `fork`: refused `NOT_READY` while the source runs a turn; creates the
   journal through `SessionStore::create` + `append`; children not copied.
   `crates/bingo-surface-rpc/src/methods.rs` gains `session/fork`.
   Black-box RPC test: fork, list, the fork is a root with `forkedFrom`.
7. **Auto-title, pure + link.** Pure `title_from(ask) -> Option<String>`
   in `crates/bingo-mod-sessions/src/title.rs` (first line, ≤60 chars at a
   word, `None` for a `/command` or `!shell` line). A `prompt.submit`
   observer in the mod writes `SessionChange::Title` only when the
   summary's `title` is `None`. Tests: titled once, never over `/rename`.
8. **Rows, pure.** `crates/bingo-mod-sessions/src/rows.rs::rows(summaries,
   now, query, current) -> Vec<Row>`: tree by `parent`, keyed sessions
   grouped by the key's first segment (`schedule/…`, channel adapters)
   into one collapsed row, filter on title and first ask (case-folded
   substring), first ask dim only when it differs from the title. Table
   test per sketch row.
9. **Props.** `Sessions { session, cwd, query, scope, limit, sessions,
   current }` in `crates/bingo-sdk/src/component/{sessions.rs,props.json}`
   and the `components!` list; golden contract test; schema and
   TypeScript regenerated. The surface lists with `SessionFilter { cwd
   (here) or none, limit }`; `more` raises `limit` by 50.
10. **Keys and verbs.** `crates/bingo-mod-sessions/src/keys.rs` through
    `onKey`: `↑↓`, `tab`, `⏎` → `{"resume":id}`, `→` → preview (bounded
    `open` with `max_snapshot_bytes`, last person line and the answer
    after it, then `close`; the mod holds it, render does no I/O), `r` →
    `Composer` prefill `/rename <title>` on that session's handle, `d` →
    confirmation naming children, then `HostApi::delete`, `f` → `fork`
    then resume the fork. Table test (row, key) → outcome.
11. **Drawing.** `crates/bingo-mod-sessions/src/draw.rs`: the sketch,
    at most the screen's height, one `❯`; snapshots at 108×30 and 80×24.
12. **Openers.** `/sessions` in the mod; bare `--resume` in
    `crates/bingo/src/main.rs:127-134` becomes optional-value and opens
    the site (an error under `--print`, black-box test); the palette's
    sessions group (`crates/bingo-surface-tui/src/palette/sessions.rs`)
    ends with `All sessions…` running `/sessions` and its detail uses
    `ago` instead of raw `updated_at`.
13. **Guide.** `guide.md` (<200 lines) registered by `guide.rs`, as in
    `bingo-mod-palette`.

## Files

- sdk: `ago.rs`, `event.rs`, `state.rs`, `host.rs`, `lib.rs`,
  `component/{sessions.rs,props.json,props.rs}`, `component.rs`,
  `render.rs`; fixtures; `schema/`, `sdk/typescript/`.
- core `host.rs`, new `fork.rs`; `bingo-store-jsonl/src/summary.rs`;
  `bingo-surface-rpc/src/methods.rs` and its black-box tests.
- tui: `welcome.rs`, `palette/sessions.rs`, site placement for `Sessions`.
- New `crates/bingo-mod-sessions/src/{lib.rs,title.rs,rows.rs,keys.rs,
  preview.rs,draw.rs,guide.rs,guide.md}`; listed in `crates/bingo/src/main.rs`;
  one `scripts/budget.toml` line (+1 member, no dependency); ADR-0084
  gains its amendment line.

## Exit criteria

- [ ] `ago`, `asked`, `title_from`, `kept`, `rows`, keys tables pass.
- [ ] Fixtures: summary with `firstAsk` and `forkedFrom`; old summary
      without either; `session/fork` request/response; `Sessions` props.
- [ ] RPC black-box: `session/fork` on a two-turn session; refused while
      a turn runs; NDJSON stdout pure.
- [ ] `TestBackend`: tab toggles scope; `more` grows the list; typing
      filters; `d` on a parent names its children and deletes them first;
      `f` resumes the fork; `r` prefills `/rename`; schedule rows fold.
- [ ] `bingo --resume` opens the site; `--print --resume` fails, 1 line.
- [ ] `enabledPlugins: {"bingo.mod.sessions": false}`: `/sessions` says it
      is off, `ctrl+r` and the palette work as before.
- [ ] tmux drive on an isolated socket, temp HOME, fake provider: three
      sessions, one child, one fork; captures pasted.
- [ ] `bash scripts/tui-smoke.sh`; `cargo fmt --all -- --check`; `cargo
      check --workspace --all-targets --locked`; `cargo clippy --workspace
      --all-targets --locked -- -D warnings`; `cargo test --workspace
      --locked`; `scripts/check_discipline.sh`; `scripts/budget.sh`;
      `cargo deny check`.
- [ ] `cargo check -p bingo-core -p bingo-mod-sessions -p bingo --all-targets
      --target x86_64-pc-windows-msvc --locked`.

## In-flight dependencies

- M166 (model picker): none directly; a fork keeps the source's model.
- M170 (status hub): the `Composer` surface action and `/rename` prefill.
- M173 (plugins panel): the mod's switch row and `off now`.
- M174 (stale-edit, action FIFO): `⏎`/`f` resolve against the current
  drawing (39f6f91c); queued site keys stay outside the FIFO (ADR-0084 §3).

## Non-goals

Archive; content search across directories; forking a child or a room.

## Risks

- `reconfigure` needs a live session and refuses `Title` for a log
  session: renaming a stored row opens it (revival, lock taken); a row
  held by another process answers that it is open elsewhere.
- A preview opens and closes a stored session; on a slow disk this is a
  pause, so `→` shows `loading…` and never blocks keys. A fork copies bytes.
