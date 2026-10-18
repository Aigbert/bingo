# ADR-0095 — The frame follows the approved prototype

Status: proposed · 2026-10-16 · Amends: ADR-0084 §1, §5 · Refines: M73 · Amended by ADR-0097
(2026-10-18): §1, §2, §6, §7 and `TurnFold` superseded, §3 refined, the prototype is Ember

## Context

The person approved the interactive prototype (2026-10-14) screen by screen and,
on 2026-10-16, asked for the terminal to match it ("完全按照原型补一下"). A
drive of all 32 prototype scenarios against the real TUI found the frame, the
transcript grammar, the asks, the palette, the model picker and the status
panel all drawn differently, plus ten bugs. The prototype leaves five behaviours
open or in conflict with recorded decisions; this record settles them and names
the sites and facts the redraw needs.

The ratchet: refusing these doors would force a second representation. The
branch, a model's scope, effort, a permission's scope phrase and a step's
outcome are each known in one place today and would otherwise be re-derived by
every drawing that shows them (status, footer, header, receipts).

## Decision

1. **The frame.** One header row (`bingo▍ ~/cwd · branch … title · session id`),
   the transcript, a `─` rule with the ` ▍ ` prompt line (no box), one
   right-aligned `KeyBar` row, and an always-on footer: mode, crew, the context
   bar, model `∴ effort`, then `· this session` only when the choice is
   session-scoped; the default scope draws nothing. The footer's middle slot
   and the notice line go; what they carried moves to toasts or the `KeyBar`
   (a pending other session is a `KeyBar` pair, `ctrl+g` and its count).
   The `/` palette opens under the prompt line in place of the `KeyBar` and
   footer, and the transcript gives up the room (the person, 2026-10-16).
2. **Toasts live in the `KeyBar`'s left end**, one at a time, 2.8 s on the wall
   clock: nothing above them moves, and a toast whose moment has passed is
   dropped, never replayed.
3. **Colour.** M73 stands for prose: body text is the terminal's foreground and
   secondary text its dim, so a palette flip remaps scrollback with no query.
   The prototype's hex tokens apply to chrome only (glow, presence, ok, bad,
   mode, code, rule, ash, pick, panel, raise), each with a light pair; tints and
   rules blend against the detected background (M71); 256-colour terminals take
   the nearest index.
4. **Esc.** On an open ask: no, and the turn stops (ADR-0084 §5 unchanged). On a
   question or form: skip, the model decides, the turn goes on. On an ask set
   aside behind a draft: back to the ask. On a pager or expanded diff: close it,
   nothing else.
5. **shift+tab** cycles `default → accept edits → plan → bypass` (the person,
   2026-10-16: bypass belongs in the cycle). `dontAsk` joins the cycle only for
   a session that started in it, after bypass, so nobody is trapped out of the
   mode they chose. Mode words are plain English, never camelCase; bypass shows
   in the bad colour so it is never mistaken for default.
6. **Thinking** is not a transcript row. It folds into the turn's steps and is
   read by expanding the fold or in Browse.
7. **A stopped turn** prints `■ you stopped the turn · N of M steps had run;
   nothing after them`. The prompt stays in the transcript; the composer is
   left as the person had it.
8. **New sites** (ADR-0084 §1 named them, this record fixes their props):
   `KeyBar { pairs: [{ keys, action, emphasis }], toast }`, `Composer { tag,
   placeholder }` over the surface's editing core, `Ledger { steps }` with each
   step's outcome, `TurnFold { steps, outcome, elapsed }`. A pane declares its
   key pairs as props; no mod draws hints inside its body. Inside a turn a
   step's row is asked of `Ledger`, not `ToolUse`/`ToolGroup`; once opened,
   what it brought back is asked of `ToolResult`, a thought of `Reasoning`,
   the stop line of `Interruption`; a receipt naming its step rides in that
   row and a running step draws no progress. Outside the steps (an agent's
   call, a producer's row, an unlinked receipt) `ToolUse`, `ToolGroup`,
   `ToolProgress` and `PermissionReceipt` still answer, so none retires.
9. **New facts, no feature nouns:** the session's git branch (a context fact,
   read once per cwd change); a model choice's scope (`session | default`) and
   effort in the session summary; a permission ask's scope phrase and a
   destructive command's size and count, written by the policy plugin; a tool
   result's one-line outcome (`+1 −1`, `3 passed`); a structured question
   answer with its asker; command use counts per person for `recent`; the
   catalogue reads the session's cwd, never the process's.

## Consequences

- The TUI redraw lands as milestones M182–M189, chrome first; each screen is
  checked against the prototype captures in a harness-owned tmux before merge.
- Snapshot tests per site; prototype captures are the reference, not the code.
- Narrow widths: the `KeyBar` drops pairs from the left, the header drops the
  title, then the branch; the footer keeps mode and model last.
- The desktop and text surfaces gain `Ledger`/`TurnFold` for free; the chat
  answers none of the new sites.

Supersedes: nothing. Refs: ADR-0064, ADR-0082, ADR-0084, ADR-0086, M73, M71.

## Amendment — 2026-10-18 (ADR-0097)

The approved prototype is now bingo Ember (`docs/design/ember/`). It replaces the frame (§1),
the toasts (§2), the thinking fold (§6), the stop line (§7) and `TurnFold` (§8). §3 keeps M73
for prose and takes Ember's chrome tokens. §4, §5 and §9 stand.

- **The prototype is the spec, cell by cell.** Every scene has a capture of its characters and
  ink tokens, exported from the prototype's own renderer. Every scene also has a `TestBackend`
  test that renders the same state at the same size and moment. The test compares characters
  exactly, and inks wherever the capture names a token. A tmux drive is still owed before a
  release.
- **Change the prototype before the code.** If a scene should look different, edit the
  prototype first, rebuild it, re-export the captures, and only then change the code. The person
  approves any change to what a scene shows.
- **Sites.** `Step` carries its tool's look (mark, ASCII twin, verb) from the tool's
  declaration. `Marks` keeps only the states that belong to no tool. `TurnFold` retires, and
  `TurnDuration` draws on the band. `KeyBar` is drawn in the footer. Inside a turn, a step's
  row is asked of `Ledger` once per row, so a mod may answer for its own steps.

