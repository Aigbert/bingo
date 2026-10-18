# bingo Ember — TUI spec (final prototype, 2026-10-18)

Source of truth: the prototype in [`prototype/`](prototype/) — its renderer is the reference, and
the captures in [`scenes/`](scenes/) are exported from it (ADR-0097, ADR-0095 as amended). This
file is the short form for the plans. Approved whole by the person on 2026-10-18; published as
https://claude.ai/artifact/QcqqBoo2DTDwJtLdpatE1w (version 4).

## Idea
The ember is bingo's attention. Where bingo is looking, something glows; when nothing glows,
nothing needs you. Three bands, one side light per turn, one edge line that carries the state.

## Bands
1. **Stream** (read): turns, bottom-anchored, blank row between turns.
2. **Dock** (act): `you` ground; its top row is the **edge line**. It morphs in place:
   input / ask / question / palette / mention / sheet / find. Cap `max(6, floor(H*.66)-2)`,
   grows one row per 14 ms, shrinks at once.
3. **Footer** (ambient, last row): left = `◆ <who> needs you` first when another session
   waits on you; away from home, whose session this is (agent + its model, or room + who is
   seated); model id (muted) + effort (dim), mode when not default, `context N%` at ≥70%
   (amber ≥80%), crew lamps + `N agents` at home, `N shells`, `wakes in X`, or a toast;
   right = keys that work right now.

## Grid
Side light column 0, mark and text column 3, verb and well column 5, step target column 13,
right margin 2, answer measure 100. Narrow terminals only cut the right side earlier.

## A turn
```
▍ 修好它，加一个回归测试                                                        4.2s
  先改 reset()，再补一个回归测试。
  ¶ read    crates/bingo-core/src/session/queue.rs                          214 lines
  * find    crates/bingo-core/tests/*.rs                                      6 files
  ± edit    crates/bingo-core/src/session/queue.rs                             +1 −1
  ⠼ run     cargo test -p bingo-core queue                                         3s
            Compiling bingo-core v0.6.3 (crates/bingo-core)

  <answer, markdown, ≤100 cols>
```
**Side light** (the band's left edge, column 0): live breathes glow↔coal 1.8 s; asking
amberGlow; newest finished turn only: done good, failed bad, stopped dim; older turns unlit. In
a teammate's session its colour replaces glow. **Clock** on the band's first row, from 1 s:
muted while live, dim when done; a stopped turn reads `stopped · 4s`; while live it keeps a
room of 10 cells so your words never rewrap under it.
**A step is one row** and stays where it happened, done or not: mark at 3, verb at 5, target
at 13 (verb + 2 when the verb is longer), outcome right. Running: the spinner (80 ms) in the
mark cell, glow (a teammate's colour for its step), elapsed from 1 s, the last output line
under the target (its leading spaces trimmed). It lands on its tool's mark, glow→ink over 420 ms; a step that changes again
(a teammate reports) lands again (`lit`). Waiting on you: `◆` breathing amber, `needs you`.
Failed: mark and verb bad, the reason right (`exit 101`). Skipped or cut: mark faint, verb and
target dim, `said no` / `stopped`. Lookups (read-only tools) read a shade quieter (mark muted,
verb dim, target muted) than tools that change things (mark fg, verb muted, target fg). A path
gives up leading folders first (`…/session/queue.rs`); a command reads in `code`; an edit
shows `+a −d`. Only auto-allowed steps say why (`accept edits`, `session rule`); a step you
changed says `changed by you`.
**Narration** sits between the steps where it was said, italic muted; the newest types out at
70 cps until something follows it. Under the steps while live: `thinking` (shimmer) when no
step runs, `working` after a 500 ms pause, the retry row. A turn that ends on a failed step
keeps that step's last 4 output lines in a well; a failure it recovered from is one red row.
Clicking a settled row (⏎ in detail) opens its **well** (one shade deeper, from column 5): what
its owner mod draws (`ToolOutput.display`) — a header-less diff (line numbers, word tints), code
numbered from where the read began, the output, or the questions and answers. Long output keeps
12 lines: a command its last, a lookup its first.
Done moment: the side light goes glow→good 320 ms and throws three sparks (700 ms, empty cells
only); the edge line flashes good 1.1 s.
**Detail** (`ctrl+o`): the thoughts (`≈ think`, first words, length right) and what you added
come back to where they happened; ↑↓ picks a row, ⏎ opens it, esc or ctrl+o folds.

## Edge line (the dock's top row)
idle faint · live comet, 2.2 s per pass, length `min(22, round(W*.22))` · ask amber breathing
1.4 s · danger bad, still · retry `mix(amber,you,.45)`, still · mode change flashes the mode
colour 900 ms · good/bad flash 1.1 s. Reduced motion: no comet (static `mix(faint,glow,.4)`).

## Acting
- Input: ember caret; placeholder by state (`Ask anything, or / for commands` · `Steer, or
  queue what comes next` · `Reply, or / for commands` · `Answer above, or type to tell bingo
  something else` · `Message <agent>` · `Steer <agent>, or queue what comes next` · `Post to
  #room, @ to call on someone`). While a turn runs: ⏎ steers (next step), tab queues (after the turn),
  ↑ on empty recalls the last queued. One esc ends the turn; queued text returns to input.
  What you add while bingo works (a steer, the reason behind a no) stays under your words on
  the `you` band, labelled right `steered` / `said no` (glow→dim 420 ms); detail puts it
  back where it landed.
  `!cmd` runs in the shell without the model. ctrl+b backgrounds a running bash.
  ctrl+c twice on an empty line exits; the exit leaves a receipt and `bingo --resume <session id>`,
  which brings back exactly this session (not the newest in the directory).
- Asks replace the input: title + why, ↑↓ list, ⏎ confirms, typing = "No, and tell it why",
  400 ms guard. Danger: bad edge, default No, red allow item, no session-wide allow.
- Questions: tabs ←→, previews beside the options, space ticks multi, `Something else`.
- `/` palette: commands and skills in one fuzzy list (prefix, contains, subsequence), ≤8 rows
  + `N of M`, shortcut right; skills show `skill · <source>`. `@` lists files with git state.
- Sheets (model, status, plugins, sessions, rewind, keys, tasks, schedule, memory, crew,
  login, mode, wake): head + list + detail in the dock; the one thing to fix sorts first.

## Crew views
Every teammate and every room is a session of its own; you look at one at a time.
- `/agents` (the crew sheet): teammates by need (asking, working, idle), then rooms; ⏎ walks
  in, tab writes `@name`. Clicking a teammate's name in the stream walks in too.
- A teammate's session looks like yours: its colour replaces glow in the side light and the
  comet; the title reads `<session> › <name>`; the footer leads with the name and its model.
- Away from home esc never stops anything: it clears the draft, then takes you home. ctrl+c
  stops the teammate's turn.
- A teammate's ask waits in its own session and never takes your dock. Everywhere else the
  footer leads with `◆ <name> needs you` and offers `ctrl+g answer`; ctrl+g walks to the next
  session that waits on you, or home when none does.
- A task bingo hands out comes home as a post: `▎` in the teammate's colour at column 0, the
  name at 3, the words at 14; consecutive posts by one voice join without a blank row.
- Rooms: a head notice names the purpose; the footer lists who is seated (the writer
  breathes). ⏎ posts. Every `@name` owes an answer: your post reads `waiting on <name>` with a
  side light breathing in their colour until they speak; a `writing` row and the comet take the
  writer's colour. `@all` calls on everyone seated. In a room `/` adds seat, unseat and close
  and hides skills. Unseating someone drops what they owed. `/close <why>` ends the room:
  nobody owes, nobody is writing, the footer reads `#room · closed`; posts and room commands are
  refused and the draft stays.

## Line breaking
No line starts with closing punctuation (`，。、；：！？）」』】》` …) or ends with opening
punctuation (`（「『【《“` …). A Latin run split only by styling moves as one unit; a unit is cut
only when it is wider than the whole line. Unit-tested (`wraptest.js`).

## Ink (tokens; light / dark)
bg #f9f4f2/#2c2a2e · fg #3a342e/#e6e2dc · ember #b04c14/#f39050 · glow #ef7a2c/#ff9b4a ·
amber #9a6200/#f2b84b · good #2b7d3c/#7fd38a · bad #b3261e/#f0706a · plan #2f62a8/#8ab4f0 ·
auto #7445c0/#b89cf5 · code #8a5520/#e9c48f · six crew colours. Derived from bg/fg: muted,
dim, faint, you, well, pick, add/del tints and words, bad/amber/glow tints.

## Marks
▍ side light / caret · ● list and footer lamp · ⠋ spinner · ◆ waits for you · ≈ a thought
(detail only) · ▔ edge · ▎ agent quote / picked row · ✓ done (toasts, plugins, tasks) · ◌ to do ·
· separator · − deleted · … cut. ASCII set: `| * \|/ * : - | + o . - ...`.

## Tool marks
The TUI names no tool. A tool's owner mod declares how its row reads, beside the tool:
`look { mark, ascii, verb, target: path | cmd | text }`; read-only and edit are the kernel's
tool traits and fail closed (an unknown tool reads in the louder tier). A mark is one character
the monospace font already has, so it carries the text's weight (SF Mono, Menlo, JetBrains Mono
and Cascadia carry every built-in one). A mark that is not one cell wide, or not ASCII in ASCII
mode, falls back to `•` (ASCII `-`); an undeclared tool reads `• <tool name in lower case>`.
fs: `¶` read (`<`) · `*` find · `/` search · `±` edit (`~`) · `+` write · `?` ask ·
bash: `$` run · `×` stop (`x`) · web: `↓` fetch (`v`) · `»` web (`>`) · pictures: `¤` look (`o`) ·
agents: `@` + the teammate's name in its colour · rooms: `@` seat / unseat / close ·
tasks: `#` plan · skills: `§` skill (`&`) · schedule: `∞` wake (`^`) · experience: `√` recall
(`!`) · mcp: `◊` + the server's name (`%`). Outside mods use the same declaration.
Mods still draw: a well is the tool's `ToolOutput.display`, and the `Ledger` site lets a mod
take over its own rows.

## Motion budget
Only three things move while working (side light, comet, spinner). Welcome: the caret writes the
wordmark (900 ms), sparks rise from it, the wordmark dissolves on the first message (520 ms).
Toast 160 / 2400 / 300 ms. Schedules breathe 2.4 s. Everything has a still form under
reduced motion.

## Scenes (36)
对话 welcome live done ledger stop steer shell image · 询问 ask-edit ask-bash caution question
mode · 面板 palette skills mention model status plugins sessions rewind keys tasks schedule
memory find · 协作 crew agent room background wake · 异常 retry signedout context · 边界 exit narrow.

## Plan
ADR-0097 records the decisions; ADR-0095 is amended (this prototype is the spec, scene by scene).
Milestones M192–M200 (`docs/plans/`): M192 ink, marks, grid, motion clock · M193 three bands ·
M194 a turn · M195 detail, wells, find · M196 the composer · M197 asks and questions · M198 every
sheet · M199 edges · M200 crew views.
