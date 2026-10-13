# M121 — Slots, the mechanism

## Goal

ADR-0070 §1–3 and §6. The TUI places four plugins by reading their
records by id: tasks, background shells, the wake and the permission
mode. Each producer now places its own view through `bingo_sdk::slot`,
and the TUI draws slots without naming any of them. The `is_list` and
`is_set` exceptions and `tasks::quiet` go. Core's `/status` and print's
`permissionMode` stop naming `"bingo.permissions"`. No `HostApi` verb,
no new dependency.

## Bricks, in build order

1. **Contract tests first** (`bingo-sdk/src/slot.rs`), failing: each
   `Slot` variant round-trips to golden JSON; `placed` skips a slot it
   cannot parse and keeps the rest; `placed` reads signals, extensions
   and `config.plugins` in plugin id, then kind, order; `with` adds
   `slots` to an object and leaves `Null` as `Null`.
2. **`bingo_sdk::slot`**: `Slot` (the six of ADR-0070 §1),
   `Placed { plugin, kind, slot }`, `with` and `placed`. `ActionItem` is
   the one in `view.rs`. `Mode.modes[0]` is documented as the resting
   mode, which a surface may leave unsaid.
3. **`ToolTraits.quiet`**, default `false`. A tool cannot be asked on
   resume, so `turn/stream.rs` stamps `ItemBody::ToolCall.quiet` (omitted when
   false) from `traits(&input)` when it opens the item. A fixture pins
   both shapes (after M113). The four task tools are quiet when `in` is
   null.
4. **Tasks place.** `journal::write` publishes `{ tasks, slots }`, a
   record-shape change; `tasks_of` reads the old array too, pinned by a
   fixture. The slot is `Activity`: `verb` is the first in-progress
   task's `activeForm` or subject; `view` is `Panel { title: summary,
   child: Tree }`, one flat node per task (`label` subject, `badge`
   owner, `tone` state). The five-row cut and `… +n pending` move with
   it as producer words.
5. **Bash places.** `Job` keeps the `ItemId` of the call that started it
   (`ToolContext.item`). The jobs signal becomes `slot::with(table,
   slots)`: `Status { text: "1 shell" }` while any run, and one
   `Row { item, body: Text("Running in the background") }` per running
   job. An empty set is `Null`, and the row falls back to the display's
   fold.
6. **Schedule places.** `wake::payload` becomes
   `slot::with({ at, note }, [Status { text: "wake", until: at }])`.
7. **Permissions places.** `describe` adds `Mode { current, modes }`,
   each item `Action { name: "permission", args: mode }`. Core
   `commands/status.rs` `mode` and print `stream_json.rs`
   `permission_mode` read the `Mode` slot's `current`.
8. **The TUI draws slots.**
   - `status.rs`: `Mode` on the left unless it rests; `Status` slots in
     the middle, one with `until` counting `"{text} in {span}"` down.
   - `activity.rs`: `Activity`, its `Panel.child` hung under the verb
     row while a turn runs and standing under `title` between turns. A
     flat `Tree` draws one mark per node by tone.
   - `rail.rs`, `panel.rs`: `Rail` and `Panel` slots, plus any payload
     with no `slots`, as today.
   - `transcript.rs`, `blocks.rs`: a quiet item draws no row unless it
     failed; a `Row`'s `body` replaces the display under its item; a
     block's memo is revised by the rows placed on its item.
   - `input.rs`: `cycle_mode` fires the item after `current` as
     `Input::Action`; `ui.tasks_hidden` becomes `activity_hidden`.
   - `run.rs`: `animating` asks whether a placed `Status.until` is ahead.
   - `tasks.rs`, `shells.rs`, `wake.rs`, `permission.rs` are deleted.
     What they drew that is not a producer word moves to `activity.rs`
     and `status.rs`.
9. **A stranger appears.** A TUI test's fake plugin, named nowhere in
   the TUI, places one of each slot and is drawn in every place.
10. **Records.** `docs/design/tui.md` §8 loses the tasks and shells
    exceptions and gains the slot places. ADR-0013 §2, §4 and ADR-0019 §8
    get dated notes. ADR-0070 is accepted for §1–3 and §6.

## Files

- `crates/bingo-sdk/src/{slot.rs,lib.rs,tool.rs,event.rs}`,
  `crates/bingo-sdk/fixtures/slots/` (new), `tests/record.rs`,
  `fixtures/record/` (quiet-call compatibility), wire schema snapshots
- `crates/bingo-core/src/{turn.rs,turn/stream.rs,accumulator.rs,commands/status.rs}`,
  `session/inputs.rs` (external calls also stamp quiet), turn contract tests,
  `crates/bingo-surface-print/src/stream_json.rs`
- `crates/bingo-tasks/src/{journal,lib,create,update,get,list}.rs`
- `crates/bingo-tool-bash/src/{jobs,lib}.rs`, `crates/bingo-schedule/src/wake.rs`,
  `crates/bingo-permissions/src/lib.rs`
- `crates/bingo-surface-tui/src/{status,activity,rail,panel,transcript,blocks,input,run,ui}.rs`;
  `{tasks,shells,wake,permission}.rs` deleted; `screens/{tasks,shells}.rs`,
  `lib.rs`, `test_support.rs`, `test_lanes.rs`, `theme.rs` (token file map), slot rendering tests
- `docs/design/tui.md`, `docs/adr/{0013,0019,0070}-*.md`

## Exit criteria

- [x] `cargo test -p bingo-sdk slot` passes; the failing first run is pasted.
- [ ] `rg 'is_list|is_set|tasks::quiet|wake::counting' crates/bingo-surface-tui/src`
      finds nothing.
- [ ] `rg '"bingo\.(tasks|tools\.bash|schedule|permissions)"' crates/bingo-surface-tui/src crates/bingo-core/src crates/bingo-surface-print/src`
      finds only `#[cfg(test)]` code.
- [x] A bingo-tasks test reads a pre-M121 array journal and writes the object.
- [ ] Each producer asserts its slots beside its record; the stranger
      test passes; shift+tab fires `Input::Action { name: "permission" }`.
- [ ] `screens/` snapshots are unchanged except the ended-job row and the
      status order when a wake and a shell both stand; both diffs pasted.
- [ ] `scripts/tui-smoke.sh` passes, and a tmux drive shows the task
      band, a shell row, a wake countdown and the shift+tab cycle.
- [ ] All gates of the upgrade protocol, plus the Windows cross-check
      for the five producer crates and the TUI.

## Non-goals

- `Voice`, seats, memory rows, the skill row and promote (M123).
- Slots in print or channels, which draw none of these today; layout as
  a pass (M124); a `View` word for a checklist.

## Risks

- Decision (2026-10-13): user authorizes `theme.rs` file-map updates for slot renderers, preserving exact token-use assertions.

- `Tone::Attention` reads "wants a person"; an in-progress task is not
  that. If a reviewer refuses the stretch, the alternative is a
  `Checklist` word, which amends ADR-0013 §1. Decide before brick 4.
- Old task arrays/wakes have no slots until the next write; resumed wakes are dead.
- An ended job falls back to "Started in the background as …"; record this in the commit.
- Re-verified at `50a885ed`: consumers and `ActionItem` still match; M112
  folds through `Transcript`, M113 pins records in sdk `tests/record.rs`.
  Call creation lives in the accumulator, publication in `turn/stream.rs`,
  and external calls in `session/inputs.rs`. No Decision changes.
- Mechanical quiet constructors/fixtures (step 6): `bingo-agents/src/{serial,tests}.rs`,
  `bingo-context/src/fixtures.rs`, `bingo-core/src/{context,host/catalog}.rs`,
  `bingo-sdk/src/state.rs`, `bingo-store-jsonl/src/summary.rs`,
  `bingo-surface-print/src/lib.rs`, TUI `graphics/picture.rs` and
  `transcript/pictured.rs`. All paths are under `crates/`.
- Producer fixtures: `bingo-tasks/fixtures/pre-m121.json`; bash job test
  constructors in `src/{kill,read,notify}.rs` mechanically gain an item id.
- Step 6 fixture update: `crates/bingo/tests/cli/board.rs` reads the task array
  from the new payload's `tasks` member; all owner/board assertions stay unchanged.

## Verified (2026-10-13; in progress)
- `cargo test -p bingo-sdk slot`: first `FAILED. 1 passed; 3 failed`;
  after implementation `ok. 4 passed; 0 failed` (with/placed assertions).
- `cargo test -p bingo-sdk --test record quiet_tool_calls`: first `FAILED`;
  after implementation `ok. 1 passed; 0 failed` (quiet flag was discarded).
- Producer `slots_` tests first failed for absent slots/object; then
  `cargo test -p bingo-tasks -p bingo-tool-bash -p bingo-schedule -p bingo-permissions --no-fail-fast`:
  `ok` (75 tasks, 146 bash, 112 schedule, 103 permissions unit tests).
- `cargo test -p bingo-surface-tui a_stranger_places`: first `missing stranger status`;
  after conversion `ok. 1 passed; 0 failed`.
- `cargo test -p bingo-surface-tui --lib`: `FAILED. 1120 passed; 5 failed; 2 ignored`.
  Failures: theme file map (stop), permission-cycle expectation, old shell-signal
  fixture, ended-shell wording, renamed hidden-scene snapshot. Full gates,
  Windows, PTY/tmux and remaining criteria are unverified. Consumer edits remain
  uncommitted; ADR-0070 remains proposed. Nothing pushed, merged or opened as a PR.
