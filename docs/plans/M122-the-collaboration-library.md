# M122 — The collaboration library

## Goal

ADR-0071 §1–3. Agents and rooms share one name rule, one set of room
record types and one writer for a room. A chase, a patience deadline and
an agent's report-back survive a restart, and a resumed session shows no
countdown for a wake that died. Two library crates join the tier:
`bingo-collab` (names, room records, pure folds) and `bingo-alarm`
(owned, keyed timers). No kernel door, no feature noun in sdk or core,
no new external dependency. The TUI's copies are M123's.

## Bricks, in build order

1. **Contracts first:** Name case/reserved-name table, existing room JSON,
   paused-clock arm/fire/cancel/replace/reconcile/drop, RoomPost wire JSON,
   and the parent report obligation fixture. Observe each missing API fail.
2. **Collab:** library tier; sdk, services, serde/json, jiff, existing schemars
   for Listener's schema. Own Name validation/equality, room constants and
   records, members/listeners/ears/cursors, mentions/owed/seen pure folds.
3. **Rooms reads through collab:** retain I/O, remove local name module,
   import shared records/folds; existing rooms assertions remain unchanged.
4. **Agents reads through collab:** keep roster resolution, use shared name
   comparisons, remove rooms.rs and M109's two-crate comparison fixture.
5. **RoomPost:** typed `rooms.post` contract and both wire adapters in
   services. Rooms owns closed/serial/draft/bounce and the landed write;
   SendMessage delegates and declares `uses`. Rooms keeps using team::KEY.
6. **Alarm:** library tier; sdk, tokio, jiff. Keyed owned tasks support arm,
   cancel, reconcile and owner-drop cancellation; no external dependency.
7. **Rooms restore:** alarms keyed by debt and `(seat, room)`, derived from
   the owed fold/cursors. Startup lists persisted open rooms and reconciles;
   overdue chases once. Re-read debt before another patience. Remove lazy
   announce-time overdue. Keep the initial owed card publication.
8. **Schedule:** replace the wake loop with alarms, keep process-local
   wakes. Session Start with no held wake clears the whole extension,
   including M121's status slot. Declare hook:schedule.
9. **Reports:** parent extension `bingo.agents`/`report:<child>` carries
   `{child, turn}`; clear with Null after delivery. Own follower per child,
   cancel on parent End. Start re-arms: completed turn reports now, missing
   child clears with a note, running child is followed to its recorded end.
10. **Records:** accept ADR-0071 §§1–3; add dated amendments to ADR-0019,
    0021, 0022, 0025 and 0029, and list both libraries in ARCHITECTURE.md.

## Files

- `crates/bingo-collab/{Cargo.toml,src/{lib,name,room,ear,cursor,mentions,owed,seen}.rs,tests/,fixtures/}` (new)
- `crates/bingo-alarm/{Cargo.toml,src/lib.rs,tests/contracts.rs}` (new)
- `crates/bingo-services/src/{lib.rs,room_post.rs}`, `fixtures/room_post/`
- `crates/bingo-rooms/src/{lib,room,cursor,ear,mentions,owed,chase,deadline,post,hook}.rs`,
  new `serial.rs` and service owner; `name.rs` deleted;
  other rooms modules mechanically import the shared name/types
- `crates/bingo-agents/src/{lib,names,message,spawn,watch,hook}.rs`;
  `{rooms,serial}.rs` deleted; new report owner/record module
- `crates/bingo-schedule/src/{lib,wake,wakes,schedules}.rs`, a new `hook.rs`;
  preserve M121 wake slots, clear the complete extension on stale resume
- Affected crates' test helpers and restart tests; `crates/bingo/tests/`
  (M109's name test removed; constructors updated; M31's restart assertion
  allows reopening for an owed report while preserving the journal prefix)
- Affected plugin `Cargo.toml`s, `Cargo.toml` (workspace edges), `Cargo.lock`,
  `scripts/budget.toml` (+2 members), `ARCHITECTURE.md`,
  `docs/adr/{0019,0021,0022,0025,0029,0071}-*.md`

## Exit criteria

- [x] `cargo test -p bingo-collab -p bingo-alarm` passes; the failing
      first run is pasted.
- [x] `rg 'fn same|fn is_holder|"bingo\.rooms"' crates/bingo-agents/src`
      finds nothing; rooms' `name.rs` and agents' `{rooms,serial}.rs` are gone.
- [x] An agents test refuses `scout` beside `Scout`, and `Parent`.
- [x] `rg 'tokio::spawn' crates/bingo-rooms/src/{chase,deadline}.rs crates/bingo-agents/src/spawn.rs crates/bingo-schedule/src/wakes.rs`
      finds nothing.
- [x] Restart tests, a second plugin starting over a store the first
      wrote: a debt is chased once; a patient seat behind is nudged once;
      a stale wake is cleared; a report owed by a finished child is
      delivered, and one owed by a running child when it finishes.
- [x] The `SendMessage` room tests pass unchanged against rooms' writer,
      and check 4e passes with `rooms.post` declared.
- [x] All gates of the upgrade protocol, plus the Windows cross-check
      for every touched crate (including both new libraries and services).

## Non-goals

- `Voice::Aside` on nudges (ADR-0071 §4), which needs
  `Origin.voice` (M123).
- The TUI's `seats.rs`, `roster.rs`, `mentions.rs` and `HOLDER` (M123).
- Wakes that outlive their process (ADR-0019 §8 keeps them in memory).

## Risks

- Startup reads each room and its parent. Parent-first restoration prevents
  competing opens from changing `--continue` selection.
- Start hooks enqueue owned recovery: awaiting their own actor would deadlock.
- M31 permits appended report recovery frames; its journal prefix stays intact.
- Windows binary validation remains with CI for the C-header limit below.

## Re-verified (2026-10-13, `80f966a7`)

- M109 already compares names case-insensitively; move its tests and rule.
- M119 provides typed team/MCP adapters; RoomPost follows that contract.
- M116's tracker is actor-local; plugin owners must retain their own tasks.
- M121 `wake::payload` owns the status slot; stale clearing uses its Null.
- `schedules.rs` keeps its supervisor wrapper; `wakes.rs` owns the alarms.
- `Listener` derives `JsonSchema`; moving it adds the existing `schemars`
  edge to collab. Report tasks use the new `bingo-alarm` edge in agents.
  These and the planned library edges add no external crate.
- Rooms/agents currently have unit plugin constructors; ownership needs
  fields, so composition and test constructions are mechanical updates under
  protocol step 6: `bingo/src/main.rs`, `bingo/tests/thinking.rs`.
  `bingo/tests/room_posts.rs` runs the moved tests unchanged against rooms;
  its existing agents helper is included by path. No sdk/core change is needed.

## Verified (2026-10-13)

Contracts were committed red before implementation; trimmed compiler output:
```text
cargo test -p bingo-collab -p bingo-alarm
error[E0432]: unresolved imports `bingo_alarm::Alarm`, `bingo_alarm::Alarms`
cargo test -p bingo-collab
error[E0432]: unresolved imports `bingo_collab::Name`, `bingo_collab::room`
RoomPost (43bf22e0): error[E0425]: cannot find function `wire` in this scope
Report (03ae5109): error[E0425]: cannot find type `Obligation` in this scope
cargo test -p bingo-collab -p bingo-alarm --locked → 1 + 2 passed; 0 failed
cargo fmt --all -- --check → exit 0
cargo check --workspace --all-targets --locked → Finished `dev` profile
cargo clippy --workspace --all-targets --locked -- -D warnings → Finished `dev` profile
a=a_proxy_with_a_token_this_run_never_minted_gets_nothing
b=a_token_may_be_dialled_again_once_its_stream_has_closed
INSTA_UPDATE=no cargo test --workspace --locked -- --test-threads=1 --skip $a --skip $b
→ exit 0; 5096 passed, 0 failed, 4 pre-existing manual/stress ignores
cargo test -p bingo --test acp_bridge --locked $a -- --exact --test-threads=1 → 1 passed
cargo test -p bingo-provider-acp --test bridge --locked $b -- --exact --test-threads=1 → 1 passed
scripts/check_discipline.sh → discipline ok (including service-usage check 4e)
scripts/budget.sh → 352 dependencies; core 1s; relink 0; budget ok
→ target/debug 6 GB: soft-limit warning (5 GB)
cargo deny check → advisories ok, bans ok, licenses ok, sources ok
cargo check -p <crate> --all-targets --locked --target x86_64-pc-windows-msvc
→ bingo-collab, bingo-alarm, bingo-services, bingo-rooms, bingo-agents, bingo-schedule: Finished `dev` profile
→ bingo: aws-lc-sys, tree-sitter, tree-sitter-bash: fatal error: stdlib.h/stdio.h/windows.h file not found
TMUX_TMPDIR=<isolated temporary directory> scripts/tui-smoke.sh → tui-smoke ok
```
The binary Windows check is left to Windows CI under protocol step 7; no toolchain installed.
Both exit-criterion `rg` commands returned 1 (no matches); all three old modules are absent.
The workspace run includes name refusal, restart recovery, all 14 unchanged room-post assertions,
and 16 PTY tests. Initial startup/CLI failures were fixed within scope; the CLI rerun passed 252/252.
ADR-0071 §§1–3 are accepted; §4 and the TUI consumer migration remain M123. No stop condition remains.
