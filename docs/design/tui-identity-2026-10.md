# bingo's own face — the terminal surface, 2026-10

> Written 2026-10-13 at `2c5c44ec`, from a drive of the TUI in a
> harness-owned tmux (fake provider; 120×40, 100×30, 80×24) and a reading
> of [tui.md](tui.md). The person's direction, the same day: give the
> surface bingo's own character (**ember** for the look, **crew** for the
> feel), with its own accent instead of Claude's, and change any part of
> the UI that does not earn its place — "the board, or whatever it is" by
> name. Plans: M147–M150. [tui.md](tui.md) stays the design language; this
> file is the change of taste it records, and §10 there gets the dated
> entries when each milestone lands.

## 1. Where the surface stands

The bones are good and stay: one frame we own, nothing jumps, one bright
border, the esc stack, cards that answer to one key, `⎿` for what came
back, folds that open one rung at a time, the status line as the one line
of furniture. tui.md §1 took Claude Code's grammar whole on 2026-08-30, and
the forms were aligned with it again on 2026-09-04 (M57, user-directed);
that was right for the hands and it is kept.

The face is not bingo's. `✻` and its sparkle, `⏺`, the accent `#d97757`
(Claude's own), the working verbs (Claude Code's list), the bordered
`✻ Welcome to bingo!` box: put beside Claude Code, nothing on the screen
says which is which. What only bingo has is nowhere on it:

- **The mascot**: a fox-eared figure in the dark, lit by one glowing caret
  `▍`, embers drifting off it (`bingo-site/public/mascot-full.png`).
- **The wordmark**: lowercase `bingo` on a cell grid, already drawn for the
  site (`bingo-site/src/hero/art.ts`).
- **The crew**: agents, rooms, teams, schedules — the things Claude Code
  does not do — drawn today as `⏺` rows like any tool call, and as raw
  plugin tables in a column on the right.

## 2. What the drive found

Rough edges — each one a defect against tui.md as written, fixed in M147
without a change of taste:

| # | seen | tui.md says |
|---|---|---|
| [x] R1 | `Glob` answers `/var/folders/…/cwd/src/main.rs` | paths relative inside the cwd |
| [x] R2 | `Bash` repeats `$ <command>` under its own heading and ends `[Exited with code 0]` | a clean exit says nothing |
| [x] R3 | `⎿  allowed` after the output of a call the person approved once | the card was the record |
| [x] R4 | `Do you want to bash sleep 3; echo done?`; the call's `description` unshown; no preview | `Do you want to…?` over a preview |
| [x] R5 | `↓ 0.0k tokens`; three formatters for one kind of count | — |
| [x] R6 | `drawing took 157ms; a debug build or a slow terminal does this`, in the accent, to every person | a notice is what is true for the person |
| [x] R7 | `/status`: `13812 of 191808 tokens` | numbers are read, not parsed |
| [x] R8 | `?` lists every command again, with `[mode]` and `<command>` in the description column, `/exit` and `/quit` both, and a clipped key | one list per question |
| [x] R9 | the welcome's cwd is `/…/T/tmp.X/cwd`: an absolute path loses its first segment | middle-elided paths |
| [x] R10 | the session list says `55.0k tokens` (every request's input, cache reads included) where the status line says `13k/191k` (the window) | one number, one meaning |

Inline code keeps its backticks in a table cell as it does in prose: that
is M73's choice (`markdown.rs`, `an_inline_code_span_keeps_its_backticks…`),
not a defect. A fence's language on a lone dim row is a matter of look and
moves to M149 (L8).

Designs that do not earn their place — changed in M148 and M150 with a
change of taste:

- **The rail and the panel sheet** (the person's "board"). Past 120 columns
  any plugin that publishes a live signal or a panel takes a 24-column
  card on the right: `owed` / `room owed` / `#design reviewer`, `❯ Board`
  / `id state`, a `members` tree. Each is a plugin's raw view under its
  `kind`, the column stands mostly empty for the whole session, and what
  the cards say is already said elsewhere — who owes an answer is on the
  session list's row (`owes an answer · 22m`), a room's members are rows of
  that list. `ctrl+p` lists the same things as `bingo.schedule · wake` over
  a body of `–`. A second representation of one fact, in internal names.
- **The welcome box**: Claude Code's furniture, holding a help line the
  status line already says and a cwd it elides badly; at 80×24 twelve
  empty rows stand over it.
- **`?`**: thirty commands repeated from the `/` menu under the keys.

## 3. The look: ember

**L1. The caret is bingo's mark.** `▍` (U+258D) takes `✻`'s places: the
activity row, the worked row, the welcome. It is a terminal's own glyph,
the thing the mascot is looking at, and one cell wide. `▏` (U+258F), the
caret before it speaks, marks thinking: `▏ Thinking…`, `▏ Thought for 2s`.
ASCII: `|` for both. The sparkle cycle `✻ ✢ ✶ ✽` goes.

**L2. The caret says what the turn is doing**, which the sparkle never did:

| state | caret |
|---|---|
| waiting for the first byte | blinks at a terminal's 530 ms |
| words arriving | steady, `glow`, embers rising (L3) |
| a tool or an agent holds the turn | breathes at the blocked pace (2.2 s) |
| stopping | dims, steady |
| at rest (worked row) | `dim`, steady |

**L3. Embers are tokens.** While words or thoughts arrive, an ember
(`·` `∙` `˙`) leaves the caret into the band's reserved air row above it
and fades over 0.6 s: one per 64 tokens received, at most four on the row.
So the motion is a meter — a fast model throws sparks, a stalled one throws
none — and it obeys tui.md §6: it reports a state change. The air row is
already held for the activity band (§3, *nothing jumps*); the foot
(`↓ N lines below`) outranks embers on it. `BINGO_MOTION=off`: no embers.

2026-10-14: M149 uses one token per four arriving characters for ember timing because usage is reported only after a round; billed token counters keep reported usage.

**L4. bingo speaks with the caret.** The model's `⏺` becomes `▍`: bold
`text` at rest, `presence` while the block streams (the comet tail is
unchanged). Tool rows keep `⏺` and its state colours — the work is the
grammar a person already reads; the voice is bingo's own.

**L5. The accent is ember amber**, a hue apart from Claude's terracotta and
twice as far from `bad` (hue 25° against 5°, where `#d97757` stood at 15°):

| token | dark | light | ANSI |
|---|---|---|---|
| `presence` | `#f0883e` (7.4:1 on ground) | `#a84d12` (5.0:1) | yellow |
| glow | `#ffc278` | `#e07a2a` | bright yellow |

Grounds, tints, `good`, `bad` and `mode` are unchanged. The site's
`tokens.ts` and the GUI follow in their own repositories; the person
carries the two tokens over.

**L6. The opening is the wordmark.** The site's grid, four rows of
half-blocks, with the caret at its end:

```text
█      ▀
█▀▀▀▀█ █ █▀▀▀▀█ █▀▀▀▀█ █▀▀▀▀█
█▄▄▄▄█ █ █    █ █▄▄▄▄█ █▄▄▄▄█ ▍
                ▄▄▄▄▄█
gpt-5.4 · high · accept edits · ~/work/bingo
↺ fix the esc race · 2h ago · /resume
```

No box. The caret types the mark left to right (a column every 16 ms,
`glow` under the caret, `text` behind it), blinks twice, throws three
embers, and rests; the facts row comes in on the existing beam. 2.4 s, the
same budget, the same rules as tui.md §11 (a frame is a function of one
number; nothing outside these rows moves; off under `NO_COLOR`, ANSI,
`BINGO_ASCII` and `BINGO_MOTION=off`, which draw it at rest). Under 40
columns or 16 rows the mark is the one line `bingo▍`. The facts row is
what is true: model, effort, mode, cwd elided from the middle; the resume
row only when this cwd has an earlier session; `/login` only when no
provider can serve a turn; the update row as M63 has it.

**L7. Words of its own.** The working verbs come from the ember and the
fox, not Claude Code's list: Kindling, Stoking, Smouldering, Foraging,
Tracking, Burrowing, Sniffing around, Pouncing, Sifting, Tinkering. When a
turn ends with every task on its list done, the worked row says so in the
name's own word: `▍ Bingo · 5 tasks done in 4m 12s`. It is earned by a
true state, said once, and never otherwise.

2026-10-14: M149 defers the earned Bingo variant to a later milestone’s producer-owned completion contract; the worked row keeps `Worked for …` and takes the caret mark.

**L8. A code block stands on the raised tint**, the bar a `>` line already
stands on, the width of the block; the fence's language rides dim at the
band's right end instead of on a row of its own. Line numbers past eight
lines are unchanged.

## 4. The feel: crew

**C1. Every agent has a face**: a sigil and a hue, both derived from its
name by one pure function, so the same agent looks the same across runs,
sessions and surfaces, and nothing is stored. Sigils `◆ ● ▲ ■ ◗ ✦`; five
hues — lime, teal, violet, orchid, pink (`#c2d86a #5cc8b4 #b39cf2 #dc8cf0
#f08cbf` dark, `#566d0f #1f7a6b #6a4fc2 #8e3aa8 #b03a7a` light), each
≥ 4.5:1 on its ground and ≥ 36° of hue from `presence`, `good`, `bad` and
`mode`; thirty faces. The ANSI look draws the sigil in `text`. The hue says *who*, never *what*: state stays in the words
(`running`, `needs you`) and the motion (a live sigil pulses), so
`NO_COLOR` loses nothing. A collision is harmless; the name is always
beside it.

**C2. Where the face appears**: the child row (`◆ reviewer(review the
manifest)` in place of `⏺`), the session list's dot column, a room post's
author, an `@name` in any prose (the named agent's hue), and the crew
strip.

**C3. The crew strip** is the status line's middle while any agent under
this session is alive: `◆ reviewer  ● scout owes #design  +2`. It replaces
the `N running` count — the same fact, now with names. One that needs the
person pulses; below 100 columns the strip is sigils only. A notice takes
the slot for its four seconds and gives it back; `ctrl+g` opens the list.

**C4. A room reads as a chat.** Consecutive posts by one author draw the
name once; a post addressed to the reader is marked `→ you`; `#room` and
`@name` are in their colours. The room's members stand on one dim row
under its first message instead of in a rail card.

**C5. The screen is the person's.** Revised 2026-10-13 at the person's
word: no rail and no panel sheet. The transcript takes the width at every
size; `ctrl+p` is unbound. A plugin is seen only where it placed itself
(`Status`, `Activity`, `Mode`, `Row`, an asked `Pane`); a payload without
`slots` is not drawn by the TUI. The rooms plugin's `owed` and `members`
are data: C3 and the session list say both. demo-ui's board is a `Row`.
M148 implements the slot and removal rules above (2026-10-14).
A mod's pane, once asked for, sits where it declares and fits (`above`,
`side`, `overlay` or a rect; ADR-0081 §7, M151).

## 5. Subtractions

The welcome box and its border; `/help for help` (the status line says
`? for shortcuts`); the command list in `?` (the `/` menu is that list);
`/quit` as a second row (an alias of `/exit`); the sparkle cycle; the
`N running` words; the rail, its pins and the `ctrl+p` panel sheet; the
rooms plugin's `owed` and `members` cards; the debug-build drawing notice
outside a debug build.

## 6. Milestones

| M | Title | After |
|---|---|---|
| 147 | The rough edges | 123 |
| 148 | The screen is the person's | 124, 147 |
| 149 | bingo's own light | 125, 148 |
| 150 | The crew on screen | 135, 149 |

M147 is R1–R10, no change of taste; most of it lives outside the TUI
(tool-fs, tool-bash, core, permissions, the command specs), and what is in
the TUI touches no file M124 or M125 restructures beyond a line. M148 is
C5 and waits for M124, so that the rail and the sheet leave the layout
pass rather than the old frame. M149 is the look (L1–L8) and waits for M125,
which moves the theme into the render context. M150 is the crew (C1–C4)
and waits for M135, which names the agent states it draws.

## 7. Risks

- `▍`, `▏` and the half-blocks are East Asian ambiguous width, as every
  box border on the screen already is; a terminal set to draw ambiguous
  characters wide breaks both alike. No new class of risk, and the
  `BINGO_ASCII` look has none of them.
- Snapshots: most of the 274 change in M149 (154 hold the welcome, 157 a
  `✻`). Each plan lists them by family and confirms them by a tmux drive;
  in M147 only the snapshots a defect is in change.
- Five hues on two grounds is a narrow space. M150 pins the hues by a
  contrast test (≥ 4.5:1 on both grounds) and a distance test against the
  four accents.
- Embers on the air row must never move a row (tui.md §3). The band test
  that pins the air row's height covers it.
