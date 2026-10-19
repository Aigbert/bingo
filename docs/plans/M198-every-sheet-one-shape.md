# M198 — Every sheet, one shape

ADR-0097 §8 (every sheet is one shape: head, list, detail; the one thing to fix sorts first);
ADR-0086 (each sheet stays its owner mod's). Spec: `docs/design/ember/`:
- `p6-dock.js`: `sheetLines`, `pickLine`, `listWindow`;
- `p4c-act.js`: `openSheet`, `SHEETS` and its helpers (`stateMark`, `filterLine`, `meter`,
  `effortScale`, `statusRows`, `pluginList`, `sessionList`, `rewindList`, `KEYMAP`,
  `taskList`, `WAKES`, `forget`).

After: M193 and M194. Runs beside M195, M196 and M197 when they touch other files.

## Goal

Every panel opens in the dock in one anatomy:
- a bold head with one dim fact on the right;
- an optional line on top (a filter or a note);
- a ↑↓ list of at most 8 rows, then `N of M`;
- an optional foot.

The thing that needs fixing sorts first. Each sheet stays its owner mod's, and the anatomy is
written once.

## Bricks

1. **The anatomy, once** (`bingo-sdk`, `sheet.rs`). This is a pure builder every owner calls.
   - `Sheet { head, fact, top, rows, selected, foot, pick }` becomes a `View` tree at a
     viewport.
   - The rows shown: `min(max or 8, max(3, rows − 4 − top))`, through `list_window` (the picked
     row is centred, clamped at both ends), then `N of M` dim.
   - A picked row has `▎` glow at TX−2 and the `pick` ground from TX−2 to the right margin.
   - A sheet that is not a pick list (`pick: false`) scrolls without a pick.
   - Keys come as the site's hints, and the footer draws them (M193).
   - Unit tests cover the window at both ends, `N of M`, and the cap at 3 rows.
2. **The sheets.** Each row reads as its `SHEETS` entry. The owners are unchanged:

   | Sheet | Owner | Sort | Keys beyond ↑↓ ⏎ esc |
   |---|---|---|---|
   | Model | `bingo-mod-models` | catalogue | ←→ effort |
   | Status | `bingo-mod-status` | fixed rows | ⏎ opens the row's sheet |
   | Plugins | `bingo-mod-plugins` | errors first | space switches |
   | Sessions | `bingo-mod-sessions` | needs you, working, then score | typing filters |
   | Rewind | surface (`rewind.rs`) | newest prompt first | — |
   | Keys | surface | four groups | — |
   | Tasks | `bingo-tasks` (its pages) | in progress, pending, done | — |
   | Schedule | `bingo-mod-schedule` | as written | space pauses |
   | Memory | `bingo-mod-memory` | instructions, then memories | ⌫ forgets |
   | Permission mode | surface (ADR-0095 §5) | the cycle's order | — |
   | Wake | `bingo-schedule` (`/wake`) | the six delays, then cancel | — |

   - Rows, hints, effort words, state marks and meters follow the prototype helpers above.
   - Missing cost, change counts or catalogue capability facts draw nothing.
   - Model changes acknowledge ID, effort, provider and context; effort changes use EFFORT words.
   - Keys use one binding table, four KEYMAP groups and glyph-table ASCII twins.
   - Permission rows include policy meanings; wake lists six delays and a pending cancel.
3. **Opening and closing.**
   - A sheet replaces the input in the dock, and esc gives the input back with the draft intact.
   - A sheet keeps its selection while its rows change (by id, never by index), and the guard
     of M197 does not apply.
   - A command that opens a sheet runs from the palette (M196) or by its key.
4. **What goes:**
   - each mod's own chrome around its list: titles, rules, its own windowing and `N of M`;
   - letter shortcuts inside sheets;
   - the tasks band;
   - the full-screen pages these sheets replace.

   Browse and the pager stay.

## Files

- `crates/bingo-sdk/`: pure pick/sheet/glyph bricks, typed presses and wire fixtures.
- `crates/bingo-mod-{models,status,plugins,sessions,schedule,memory,browse,palette}/`.
- `crates/bingo-{tasks,schedule,collab,permissions,checkpoints,mods}/`.
- `crates/bingo{,-core,-channels,-channel-adapter,-channel-feishu,-mod-crew}/`: viewport/context consumers.
- `crates/bingo-{agents,context,experience,mcp,mod-ask,plugin-rpc,rooms,skills,tool-bash}/`: context literals.
- `crates/bingo-surface-tui/`: dock, sheets, bindings, capture fixtures and owner tests; the
  sheet stage (`run/acting/tests/sheets/`) is a kernel stand-in that holds the real owner mods.
- `scripts/tui-smoke.sh`: the model, keys and sessions steps wait for dock sheets.
- `schema/`, `sdk/typescript/`: generated viewport/placement contracts and stack-safe views.
- `docs/{plans,adr,verification}/`: M198 evidence, ADR-0097 service seam and M200 face rule.
- `bingo-tasks` adds workspace `bingo-collab` for roster-order owner inks.
- TUI promotes workspace `bingo-services` to read transient checkpoint file counts.
- Plugins enables SDK `testing` for its owner contract; no new workspace package.
- `Viewport.glyphs` defaults to Unicode; its fixture, schemas and TypeScript land together.

## Exit criteria

- [ ] Full frame through the harness: `model`, `model-moved`, `model-chosen`, `status`,
      `status-moved`, `status-entered`, `plugins`, `plugins-toggled`, `plugins-github`,
      `sessions`, `sessions-filtered`, `sessions-switched`, `rewind`, `rewind-rewound`, `keys`,
      `keys-closed`, `tasks`, `tasks-again`, `schedule`, `schedule-paused`, `schedule-ran`,
      `memory` and `memory-forgot`.
- [x] The `sheet.rs` unit tests above. One test per owner shows its rows come from the builder
      (no second windowing).
- [x] The permission mode and wake sheets have band tests against a frame built the same way.
      They have no capture.
- [x] Gates and checks as in M192. tmux drive: each sheet opened, moved, acted on and closed at
      100×32; sessions filtered and keys at 80×24.

## Non-goals

The crew sheet (M200); the sign-in sheet's captures (M199); new rows a sheet does not have
today beyond what its capture shows; mouse in sheets beyond a click on a row.

## Risks

- An open sheet hides the queued rows and the mentions, as the prototype's dock does; M196 owns those rows.
- `activity_hidden` is no longer set; it remains until M196 removes its read at the merge.
- Rewind file counts are read through the registered service only while its sheet is open; absent facts stay blank.
- **Many mods change at once.** One commit per owner, each with its own tests, so a bad sheet
  reverts alone.
- Missing capture facts draw nothing; Verified names each one.

## Verified

Round 4, on dev acb299e02. Evidence: `docs/verification/m198/checks.txt`.

- [x] Captures by key — every M198 scene opens its sheet through `Run::terminal_event` (or
  `/name` as a line) on a stage holding the owner mods, presses the capture's `after` keys, and
  compares the frame. No capture builds UI state by hand beyond the world it starts from.
  - Full frame (15): model, model-moved, model-chosen, plugins, plugins-toggled, sessions,
    sessions-filtered, sessions-switched, rewind, keys-closed, tasks, tasks-again, schedule,
    schedule-paused, schedule-ran.
  - Bands (7): keys (ctrl+g row, M200); rewind-rewound (welcome, M199); status ×3 (branch
    and "3 changed", M199; agents row, M200); memory ×2 (instruction line counts and memory
    dates: no owner, the memory service lacks them). plugins-github stays M199's.
- [x] Toast parts — `Slot::Toast`/`ToastProps` carry `TextChild` parts drawn through the one
  tone path. Owners send segments; schemas and TypeScript changed with the contract.
- [x] Faces — TUI inks come from `bingo_collab::face`; `Faces` owns the mention names.
- [x] History — fixes folded with `--fixup` and autosquash; 27 commits. `rebase -x 'cargo fmt
  --all -- --check && cargo check --workspace --all-targets --locked' dev`: 27/27.
- [x] Gates — fmt, check and clippy exit 0. Workspace tests: 133 groups, 6626 passed, 1 failed
  (`acp::the_restore_ladder_climbs_resume_then_load_then_a_file`, untouched by M198; it passed
  alone). `discipline ok`, `budget ok` (423 of 423), deny ok.
- [ ] Windows — 22 of 31 touched crates pass. Nine are blocked by aws-lc-sys, rquickjs-sys or
  tree-sitter build scripts and are owed to CI; none has a Rust error.
- [x] PTY — `tui-smoke ok`. The drive passes 14/14 at 100×32 and at 80×24, with CLEAN on both,
  on a private `m198-drive` server with a temporary HOME, data dir and cwd and the fake provider.
- Departures: a press is signed with the terminal; the sessions filter resets the cursor;
  memory paths drop `./` ("AGENTS.md", as p4b-data.js MEMORY); the stage runs
  `HookPoint::Event` observers as the kernel does; schedule fires use the real clock at
  scene build; toast parts are third in history so every commit builds; rewind file replies
  moved to `pulling.rs` (run.rs ≤1000 lines).
- Plan errors: the ↑ queued-edit row is not owed (M196 merged; it is in KEYMAP and the
  guide); the rewound welcome is M199's, not M192's; exit criterion 1 asks full frames of
  scenes whose facts are M199's and M200's.
- Still owed: window.rs (mentions M196 part 2, ask options M197), the pager, and the mod
  Rect/Overlay anchors.
