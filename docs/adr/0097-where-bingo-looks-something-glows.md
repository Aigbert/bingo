# ADR-0097 — Where bingo looks, something glows

Status: accepted · 2026-10-18 · Plans: M192–M200 · Amends: ADR-0095 (§1, §2, §6, §7 and
`TurnFold` superseded; §3 refined), ADR-0084 §1 (`Ledger`, `TurnFold`) · Spec: `docs/design/ember/`

## Context

On 2026-10-18 the person asked for the terminal to be redesigned from nothing ("完全重新设计
… 世界一流水平"). They approved the result, the bingo Ember prototype, as a whole: 36 scenes
in light and dark, ASCII, reduced motion and 80×24, plus seven decisions. They replaced the
third decision themselves: every step gets a row of its own, and each tool gets its own mark
("还是一行一行往下推吧 另外每个toolcall都应该有自己好看的icon"), drawn by mods ("TUI里面Mod还可以绘制").
The prototype's source, its short spec and 105 captures exported from it are in
`docs/design/ember/`.

The ratchet: one kernel door opens. A tool's look is declared beside its spec. If that door
stayed shut, every surface would keep its own table mapping tool names to marks and verbs. That
would say again, far from the tool, what the tool is, and every mod's tools would read as
unknown. The traits that pick a row's tier (`read_only`, `edit`) already belong to the kernel.

## Decision

1. **Attention glows; nothing else does.** There is one accent, the ember. State is drawn in
   three places only: the dock's top row (the edge line), each turn's side light, and the footer.
   When nothing glows, nothing needs you. The window title names the session (`<title> — <dir>`;
   away from home, `<session> › <name>`).
2. **Three bands.** The *stream*, which you read: turns anchored to the bottom, with a blank row
   between turns. The *dock*, where you act: one place that becomes the input, an ask, a
   question, the palette, the mention list, a sheet or find. Its top row is the edge line. It
   holds at most `max(6, ⌊H·0.66⌋−2)` rows, grows one row per 14 ms and shrinks at once. The
   *footer*, the last row: ambient facts on the left, the keys that work right now on the right.
   There is no header row, notice row, `KeyBar` row or status line, and toasts take the
   footer's left side.
3. **One grid.** The side light sits at column 0, marks and text at 3, verbs and wells at 5,
   and a step's target at 13. The right margin is 2 and the answer measure is 100. A narrow
   terminal only cuts the right side earlier. No line starts with closing CJK punctuation, and
   a Latin run split only by styling moves as one unit.
4. **A turn is your words on a band, its steps, its answer.**
   - The band carries the side light: breathing glow↔coal while live, amber while asking. Only
     the newest finished turn stays lit: good, bad, or dim if stopped. The band's clock sits at
     the right, from 1 s; a stopped turn reads `stopped · 4s`.
   - Each step is one row and stays where it happened: mark, verb, target, and the outcome on
     the right. A running step spins in its mark cell, then lands on its mark (glow→ink, 420 ms).
     Lookups read one shade quieter than changes.
   - Narration stays between the steps where it was said.
   - A turn never folds, and nothing draws a stop line.
5. **Detail opens in place.** `ctrl+o` brings a turn's thoughts and your additions back to where
   they happened. ↑↓ picks a row, and ⏎ opens its *well*, one shade deeper from column 5. A
   well shows what the tool's owner drew (`ToolOutput.display`). A command keeps its last 12
   lines, a lookup its first 12.
6. **A tool's owner says how its row reads.** `ToolSpec` gains
   `look: Option<ToolLook { mark, ascii, verb, target: path | command | text }>`, which is never
   sent to a model.
   - A mark is one cell the monospace font already has. A mark that is not one cell, or not
     ASCII under ASCII, falls back to `•` (`-`). An undeclared tool reads `• <name>`.
   - The tier comes from `ToolTraits` and fails closed: an untrusted or unknown tool reads loud.
   - No surface and no built-in drawing mod names a tool.
   - A mod still draws: `Ledger` is asked per step row, so a mod may answer for its own steps
     and pass the rest on with `next`.
7. **Acting never moves.**
   - The input stays put. While a turn runs, ⏎ steers (the words land before the next step),
     tab queues (they go after the turn), and ↑ on an empty line recalls the last queued.
   - What you added stays under your words, labelled `steered` or `said no`.
   - One esc ends the turn, and queued text comes back to the input. `!cmd` runs in the shell
     without the model.
   - Two ctrl+c on an empty line exit, leaving a receipt and `bingo --resume <session id>`.
8. **Asks, questions and sheets open in the dock.**
   - An ask replaces the input: a title and the reason, a ↑↓ list, ⏎ to confirm. Typing means
     "No, and tell it why". A 400 ms guard swallows a stray ⏎.
   - A dangerous ask draws a bad edge, starts on No, colours its allow item red, and offers no
     session-wide allow.
   - Every sheet is one shape: head, list, detail. The one thing to fix sorts first.
9. **Each teammate and each room is a session you look at.**
   - `/agents` lists them by need, and ⏎ walks in.
   - Away from home, their colour takes the side light and the comet. esc clears the draft,
     then goes home, and never stops anything; ctrl+c stops their turn.
   - A teammate's ask waits in its own session. Elsewhere the footer leads with
     `◆ <who> needs you`, and ctrl+g walks there.
   - In a room every `@name` is owed an answer until they speak.
10. **Motion has a budget.** While working, only the side light, the comet and the spinner
    move. Each animation is a pure function of the clock with a still form under reduced motion,
    and each mark has an ASCII twin. The caret rests after 10 s without a key.
11. **Colour.** M73 holds for prose: body text is the terminal's foreground, and
    muted/dim/faint are blends of the detected foreground and background. Chrome takes Ember's
    hex pairs (ember, glow, amber, good, bad, plan, auto, code and six crew colours), chosen by
    the detected background. Grounds (you, well, pick, tints) blend against that background
    (M71). 256-colour terminals take the nearest index. `NO_COLOR` keeps the glyphs and
    modifiers only.

## Consequences

- The plans each scene belongs to carry a `TestBackend` test against that scene's capture
  (ADR-0095 as amended). The capture exporter, node with playwright-core, is a docs tool and
  not a build dependency.
- `Step` carries its tool's look. `Marks` keeps only the states that belong to no tool
  (spinner, asked, cut). `TurnFold` retires, and a mod that hooks it is told so by `validate`.
  `TurnDuration` draws on the band.
- `KeyBar` keeps its props and is drawn in the footer.
- Plans M186, M188 and M189 were never started. M197 and M198 replace them.
- Check 4c extends to `bingo-surface-tui` and `bingo-mod-ledger`.
- The desktop and text surfaces get the declared marks for free.

Supersedes: ADR-0095 §1, §2, §6, §7 and its `TurnFold`. Refs: ADR-0064, ADR-0079, ADR-0082,
ADR-0084, ADR-0086, M71, M73.
