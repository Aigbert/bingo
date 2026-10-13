# M146 — Parity: Claude Code's examples run in the TUI

## Goal

The Phase 7 acceptance test. Claude Code's three example mods (the
tool-call guard, the band above the prompt, the tool-calls pane) run in
process (QuickJS, M137) in the TUI. A port changes
only its import specifier, plus the tool names and inputs it matches
where bingo's differ (ADR-0079 §1), and each port's diff is pasted.
`bingo plugins test` passes a test for each. `design/mods-parity-2026-10.md`
gains a Status column, and every row names its evidence: a test, a
drive, or the non-goal that owns the gap. Every gap in its §8 is written
down as a non-goal with its reason. After M139, M141 and M142. The
desktop half was dropped on 2026-10-13 with M143–M145 (ADR-0080).

## Bricks, in build order

1. **The committed examples are rewrites, not copies.** The examples ship
   in Claude Code's authoring skill with no licence of their own, and
   Claude Code is proprietary.
   - `examples/plugins/{tool-call,band,pane}` (M141, with M142's tests)
     must be bingo's own modules, written from the behaviour in brick 2.
     Any file there that is a copy is replaced first, and its tests are
     kept. No second set is written.
   - If the person obtains terms that allow it, the originals replace
     the rewrites, with attribution, in their own commit.
2. **The behaviour, as tests.** M142's `*.test.ts` for each example,
   using `bingo/testing`, must cover the list below. Missing cases are
   added and seen failing first.
   - **tool-call:**
     - an Edit of `.env` or `config/.env.local` is denied, naming the
       plugin;
     - `.envrc` passes;
     - Bash arrives trimmed;
     - a failed run sets the status, and a success clears it;
     - a throwing hook's `.catch` denies only if `next` was not called.
   - **band:**
     - tool calls are counted per turn;
     - moving `mock.clock` past 120 s toasts once;
     - `$.ui.mount` draws `AbovePrompt`;
     - the button resets the count.
   - **pane:**
     - `session.start` registers `tool-calls` and opens the pane;
     - each call is listed, and the done ones are dim;
     - `viewport.rows` bounds the list.
3. **The port of the originals, run locally, never committed.**
   `scripts/port_examples.sh <dir> <out>` lays each original out as a
   mod:
   - `plugin.json`;
   - `hooks/<file>`, so that `'../types'` resolves;
   - `types/index.d.ts`, built from its `*-state.d.ts`.

   It rewrites the specifier `'claude-code'` to `'bingo'` in both files.
   Where bingo's tools differ, it applies the tool name and input map;
   none is believed to differ (`Edit`/`file_path`, `Bash`/`command`).
   Then it runs `validate`, and `test` against the committed tests. A
   name the original reaches for and bingo lacks is a failing port. It
   is reported to its owner (ADR-0078 §1 and §8, ADR-0079 §1, §4 and
   §5), and neither the original nor a decision is edited to make it
   pass.
4. **In the TUI.** A drive in a detached tmux session that the drive
   owns, never a terminal app and never a foreign process, runs
   `bingo --plugin-dir <mod>` with a `BINGO_FAKE_SCRIPT` that edits
   `.env`, runs Bash with padded whitespace, and fails once. The window
   is set to 160 and then 100 columns; the pane sits inline at both
   (ADR-0081 §7). Each step's `capture-pane` output is pasted.
5. **Two transports.** `tool-call` also runs out of process, under node
   through the SDK (M133), and its test gives the same results. `band`
   and `pane` are `.tsx`, and node's type stripping does no JSX.
6. **The matrix.**
   - §2 and §3 gain a Status column. Each cell reads
     `done — <test or drive>`, `gap — non-goal in M<n>`, or `n/a`.
   - §4–§6 each end with an evidence line.
   - Each §8 gap is checked against its owner's Non-goals. A gap that is
     missing there is added in a `docs(plans)` commit.

## Files

- `examples/plugins/{tool-call,band,pane}/` (rewrites checked, missing
  test cases added)
- `scripts/port_examples.sh` (new)
- `crates/bingo/tests/mods_parity.rs` (new): `bingo plugins test` on
  each example, and on `tool-call` under node
- `docs/design/mods-parity-2026-10.md` (Status)

## Exit criteria

- [ ] `bingo plugins validate` and `bingo plugins test` exit 0 on each
      `examples/plugins/{tool-call,band,pane}` (pasted).
- [ ] `scripts/port_examples.sh <Claude Code 2.1.296 examples> "$TMP"`
      passes. `diff -r` of each original against its port is pasted,
      and shows only the specifier and any tool map lines.
- [ ] `cargo test -p bingo --test mods_parity` passes, and the node run
      is not skipped on CI's Linux job.
- [ ] The tmux drive's `capture-pane` output is pasted: the denial, the
      status, the band, the pane inline at both widths, and the tool row.
- [ ] Every row in §2 and §3 has a Status cell. An `awk` count of the
      rows without one prints 0 (pasted).
- [ ] `rg` finds each §8 gap in its owner's Non-goals (pasted). All
      gates of the protocol pass.

## Non-goals

Each gap in parity §8, with its owner and its reason:
- `Client` modules and `$.ui.blit` (M140, M141). Code on a surface's
  drawing thread needs a runtime on each surface, each holding a copy
  of the mod's state.
- `prompt.edit` on each keystroke (M136). The composer is the surface's,
  and a round trip per key breaks IME composition.
- `prompt.attachment` and `prompt.mention` (M136). Surfaces resolve `@`,
  and a hook sees the result in `prompt.submit`.
- `$.ui.scroll`, `focus`, `selection` and `copy`, and `open`'s
  `isPlaced` (M140, M141). These belong to the surface
  (ADR-0013 §4), and the host has no verb toward a surface.
- `$.tool.check` without a run, `$.prompt.compose` as a read, and
  `$.session.repo` (M138). Each would need a kernel door of its own.
- `audio` (M139). No surface plays sound, and the tree has no audio
  stack.
- `classic.*` (M136). Shell hooks stay with hooks-shell, behind the
  `Hook` adapter.
- The desktop, vscode and mobile surfaces. None is in this tree; the
  GUI stays in its own repository (ADR-0080, rejected).
- A docked pane (M140). The person chose no side column (ADR-0081).
- `telemetry.*` and `attribution.text`: not applicable.

Also out: committing the originals without terms that allow it; running
JSX mods out of process; and type-checking a port against props that
only Claude Code has, such as `AbovePrompt`'s `hasSurvey`, which reads
`undefined` here, so the band draws as it would without a survey.

## Risks

- M141's examples are bingo's own rewrites under Claude Code's names
  (fixed 2026-10-13). If a copied file or a renamed field lands anyway,
  brick 1 here replaces it, and the conflict goes back to M141.
- The examples sit in a temporary skill bundle whose path changes with
  each build. The script takes the directory as an argument, and the
  evidence names build 2.1.296.
- Driving the 120 s toast means waiting 120 s. `mock.clock` proves it in
  the test, and the drive shows that a short turn draws no toast.
- Re-verify at start: M136's `tool.call` shape and `turn.complete`;
  `$.plugin` and the `$` methods (M138, M139); element, prop and
  component names (M140, M141); `'bingo'`'s exports, the `*.test.ts`
  runner, `bingo/testing` and what sits in `examples/plugins/` (M141,
  M142); bingo's `Edit` and `Bash` inputs; the parity doc's rows.
