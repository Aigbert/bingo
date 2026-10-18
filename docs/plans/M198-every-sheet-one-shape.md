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

   - **Model:** the right side reads `in use` in good. The foot is the effort scale (the current
     effort glow bold) and its description muted. ⏎ toasts `✓ <model> <effort>`.
   - **Status:** the context meter is 20 cells, `━` over `─` (`=`/`-` under ASCII), amber from
     80%. The plugins row names the first broken plugin in amber. ⏎ on the context row
     compacts.
   - **Plugins:** `✓` good or `·` dim, the name at 13, the description dim, and the error amber
     or the kind faint on the right. Switching toasts `<name> on · from the next turn`. ⏎ on a
     broken one opens the sign-in sheet.
   - **Sessions:** the lamp breathes amber while asking and glow while working, and is faint
     when idle. The title at 26, the directory dim, and `needs you`, `working` or when on the
     right. The head fact reads `needs you first`, or `N of M` while filtering.
   - **Rewind:** a top note reads `the conversation and the files both go back`, and each row
     reads `N files back` or `no files`. ⏎ rewinds, puts the prompt back in the input, and
     toasts `✓ rewound · N files restored`.
   - **Keys:** two columns from 90 columns, the key at 11 cells, ASCII twins under ASCII. `?`
     opens it, and so does `/help`. The head fact reads `/help lists every command`.
   - **Tasks:** `✓` good, the spinner in the owner's ink, or `◌` dim. The owner's name on the
     right, in its ink (bingo glow). `ctrl+t` opens it, and the tasks band goes.
   - **Schedule:** the lamp breathes over 2.4 s while on. The foot reads
     `/schedule every 30m <prompt> adds one`. ⏎ runs now at home.
   - **Memory:** ⌫ on a memory forgets it with `✓ forgot · <title>`. On an instruction it says
     `instructions live in files · edit <file>`.
   - **Permission mode:** the picked mode in its colour. The foot lists the session's rules in
     `code`, or the "none yet" line.
   - **Wake:** `in 10s` … `in 1h`, then `cancel the wake-up` in bad while one is set.
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

- `crates/bingo-sdk/src/sheet.rs` (new; one noun).
- `crates/bingo-mod-{models,status,plugins,sessions,schedule,memory}/src/{draw.rs,rows.rs,
  keys.rs}`; `crates/bingo-tasks` (its pages); `crates/bingo-schedule` (`/wake`).
- `crates/bingo-surface-tui/src/{rewind.rs,keys.rs,guide.rs,screens/tasks.rs,dock.rs}` and the
  permission mode sheet.

## Exit criteria

- [ ] Full frame through the harness: `model`, `model-moved`, `model-chosen`, `status`,
      `status-moved`, `status-entered`, `plugins`, `plugins-toggled`, `plugins-github`,
      `sessions`, `sessions-filtered`, `sessions-switched`, `rewind`, `rewind-rewound`, `keys`,
      `keys-closed`, `tasks`, `tasks-again`, `schedule`, `schedule-paused`, `schedule-ran`,
      `memory` and `memory-forgot`.
- [ ] The `sheet.rs` unit tests above. One test per owner shows its rows come from the builder
      (no second windowing).
- [ ] The permission mode and wake sheets have band tests against a frame built the same way.
      They have no capture.
- [ ] Gates and checks as in M192. tmux drive: each sheet opened, moved, acted on and closed at
      100×32; sessions filtered and keys at 80×24.

## Non-goals

The crew sheet (M200); the sign-in sheet's captures (M199); new rows a sheet does not have
today beyond what its capture shows; mouse in sheets beyond a click on a row.

## Risks

- **Many mods change at once.** One commit per owner, each with its own tests, so a bad sheet
  reverts alone.
- **Facts a capture shows that bingo lacks.** The cost line, `N changed` and `reads images` come
  from the snapshot and catalogue today. A missing fact draws nothing rather than a guess, and
  Verified names each one.
