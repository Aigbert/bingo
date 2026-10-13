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

1. **Contracts first**, failing: each `Slot` round-trips golden JSON;
   `placed` skips only unreadable slots, reads signals, extensions and config
   in plugin/kind order; `with` adds `slots` to objects and leaves Null alone.
2. **SDK mechanism**: six ADR-0070 §1 slots; `Placed { plugin, kind, slot }`;
   `with`, `placed`; reuse `ActionItem`. `Mode.modes[0]` is the resting mode.
3. **Quiet calls**: `ToolTraits.quiet` defaults false; stream publication and
   external calls stamp `ItemBody::ToolCall.quiet`, omitted when false.
   M113-compatible fixtures pin both shapes. Task tools are quiet when `in` is null.
4. **Tasks**: read legacy arrays; write `{ tasks, slots }` atomically. Activity's
   verb is the first in-progress activeForm/subject. Its Panel title is the
   summary; a flat Tree carries subjects, owners and state tones. The five-row
   cut and overflow wording belong to the producer (a Stack when overflow stands).
5. **Bash**: Job keeps `ToolContext.item`; publish the table with a running
   count Status and a Row per live job saying “Running in the background”.
   Empty jobs publish Null; ended rows return to their display fold.
6. **Schedule**: wake payload keeps `{ at, note }` and places Status until `at`.
7. **Permissions**: describe places Mode with `permission` actions. Core
   `/status` and print `permissionMode` read the Mode slot's current label.
8. **TUI**: status draws Mode/Status; activity draws Activity, hanging Panel's
   child beneath a running verb and standing beneath its title between turns.
   Flat Trees use tone marks. Rail/panel draw placed cards and legacy payloads
   without `slots`; quiet calls omit rows unless failed but retain displays.
   Row replaces specified fields; item placements revise memoized blocks.
   Shift+Tab fires the next Mode's `Input::Action`; `activity_hidden` replaces
   `tasks_hidden`; animation reads future Status.until values. Delete the four
   producer readers (`tasks`, `shells`, `wake`, `permission`).
9. **Stranger**: an unknown plugin places all six slots with no registration.
10. **Records**: update TUI design §8, dated ADR-0013 §2/§4 and ADR-0019 §8 notes;
    accept ADR-0070 §§1–3,6 only. §§4–5,7 remain M123.

## Files

- `crates/bingo-sdk/src/{slot.rs,lib.rs,tool.rs,event.rs}`,
  `crates/bingo-sdk/fixtures/slots/` (new), `tests/record.rs`,
  `fixtures/record/` (quiet-call compatibility), wire schema snapshots (`schema/{rpc,plugin}.json`)
- `crates/bingo-core/src/{turn.rs,turn/stream.rs,accumulator.rs,commands/status.rs}`,
  `session/inputs.rs` (external calls also stamp quiet), turn contract tests (`turn/tests{.rs,/quiet.rs}`),
  `crates/bingo-surface-print/src/stream_json.rs`
- `crates/bingo-tasks/src/{journal,lib,create,update,get,list}.rs`
- `crates/bingo-tool-bash/src/{jobs,lib}.rs`, `crates/bingo-schedule/src/wake.rs`,
  `crates/bingo-permissions/src/lib.rs`
- `crates/bingo-surface-tui/src/{status,activity,rail,panel,transcript,blocks,input,run,ui}.rs`;
  `{tasks,shells,wake,permission}.rs` deleted; `screens/{tasks,shells}.rs`,
  `lib.rs`, `test_support.rs`, `test_lanes.rs`, `theme.rs` (token file map),
  `transcript/slots.rs` (row rendering keeps `transcript.rs` below 1,000 lines), slot tests
- `docs/design/tui.md`, `docs/adr/{0013,0019,0070}-*.md`

## Exit criteria

- [x] `cargo test -p bingo-sdk slot` passes; the failing first run is pasted.
- [x] `rg '\b(is_list|is_set)\b|tasks::quiet|wake::counting' crates/bingo-surface-tui/src`
      finds nothing.
- [x] `rg '"bingo\.(tasks|tools\.bash|schedule|permissions)"' crates/bingo-surface-tui/src crates/bingo-core/src crates/bingo-surface-print/src`
      finds only `#[cfg(test)]` code.
- [x] A bingo-tasks test reads a pre-M121 array journal and writes the object.
- [x] Each producer asserts its slots beside its record; the stranger
      test passes; shift+tab fires `Input::Action { name: "permission" }`.
- [x] `screens/` snapshots are unchanged except the ended-job row and the
      status order when a wake and a shell both stand; both diffs pasted.
- [x] `scripts/tui-smoke.sh` passes, and a tmux drive shows the task
      band, a shell row, a wake countdown and the shift+tab cycle.
- [x] Native protocol gates and eight Windows crate checks.
- [ ] Four full Windows checks: missing third-party C headers; deferred to CI below.

## Non-goals

- `Voice`, seats, memory rows, the skill row and promote (M123).
- Slots in print or channels, which draw none of these today; layout as
  a pass (M124); a `View` word for a checklist.

## Risks

- Decision (2026-10-13): user authorizes `theme.rs` file-map updates for slot renderers, preserving exact token-use assertions.

- In-progress task nodes use the plan's `Tone::Attention`; no new View word.
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

## Verified (2026-10-13)

- `cargo test -p bingo-sdk slot`: first `FAILED. 1 passed; 3 failed`;
  implemented `ok. 4 passed; 0 failed`. Quiet record test: first `FAILED`
  (flag discarded), then `ok. 1 passed; 0 failed` with `--test record quiet_tool_calls`.
- Producer `slots_` contracts first failed for absent slots/object; then
  `cargo test -p bingo-tasks -p bingo-tool-bash -p bingo-schedule -p bingo-permissions --no-fail-fast`:
  `ok` (75 tasks, 146 bash, 112 schedule, 103 permissions); task traits add one test.
- `cargo test -p bingo-surface-tui a_stranger_places`: first `missing stranger status`,
  then `ok. 1 passed; 0 failed`; mode actions, quiet displays/failures and row memo tests pass.
- Exit-criterion `rg` commands: removed symbols have no matches (exit 1);
  producer literals occur only in cfg(test). Word boundaries exclude unrelated test-name substrings.
- Visible snapshots below use prefix `bingo_surface_tui__screens__`; tmux confirms each:
  `shell_ran_80x24.snap`: “Ran in the background” → “Started in the background as job_ab12cd34”; display fallback, `slots-ended-80x24.txt`.
  `shell_ran_120x40.snap`: same wording diff at wide width; display fallback, `slots-ended-120x40.txt`.
  `shell_and_wake_80x24.snap` (new): “1 shell · wake in 4m” → “wake in 4m · 1 shell”; publisher order, `slots-running-80x24.txt`.
  `shell_and_wake_120x40.snap` (new): same status-order diff at wide width; `slots-running-120x40.txt`.
- Hidden-task snapshot keeps its original name/content. Exact theme token map passes;
  earlier cycle expectation, shell fixture and snapshot failures are resolved without weakening assertions.
- `cargo fmt --all -- --check`: exit 0; `cargo check --workspace --all-targets --locked`:
  ``Finished `dev` profile ... in 23.56s``; `cargo clippy --workspace --all-targets --locked -- -D warnings`:
  ``Finished `dev` profile ... in 12.90s``.
- `cargo test --workspace --locked -- --test-threads=1 --skip a_proxy_with_a_token_this_run_never_minted_gets_nothing --skip a_token_may_be_dialled_again_once_its_stream_has_closed`:
  exit 0; summed `5054 passed; 0 failed; 4 ignored` (existing measurement, stress and two storyboard tests).
- `cargo test -p bingo-provider-acp --test bridge --locked a_token_may_be_dialled_again_once_its_stream_has_closed -- --exact --test-threads=1`:
  `ok. 1 passed; 0 failed`; same isolated command with `-p bingo --test acp_bridge`
  and `a_proxy_with_a_token_this_run_never_minted_gets_nothing`: `ok. 1 passed; 0 failed`.
- After the discipline-only test split, `cargo test -p bingo-core -p bingo-surface-tui --lib --locked`:
  core `426 passed`; TUI `1129 passed; 0 failed; 2 ignored`; fmt/check/clippy rerun above.
- `scripts/check_discipline.sh`: `discipline ok` after splitting two oversized test files.
  `scripts/budget.sh`: `348 (max 348)` dependencies, warm core `0s`, relink `0`, debug `6 GB` (soft warning), `budget ok`.
  `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.
- Windows command, separately per touched crate: `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc`.
  sdk/core/tasks/tool-bash/schedule/agents/context/store-jsonl: ``Finished `dev` profile``, exit 0.
  permissions: `tree-sitter`/`tree-sitter-bash`, missing `stdio.h`/`stdlib.h`;
  surface-tui/surface-print/bingo: `aws-lc-sys`, missing `stdlib.h`/`windows.h`.
  These four checks remain for Windows CI under step 7; no Rust error reported, no toolchain installed.
- Workspace PTY output: `ok. 16 passed; 0 failed`, no retry. `scripts/tui-smoke.sh`: `tui-smoke ok`
  (private socket via tmux shim). `python3 target/m121-verification/slots-drive.py`: `M121 tmux drive ok`;
  captures there show active/idle task bands, running/ended rows at both widths, wake-first countdown,
  Shift+Tab acceptEdits → plan, wake removal and exit 0. Logs: `/tmp/m121-*.log`.
