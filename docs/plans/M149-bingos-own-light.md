# M149 — bingo's own light

## Goal

[tui-identity-2026-10.md](../design/tui-identity-2026-10.md) §3, L1–L8,
the person's direction of 2026-10-13 (ember, own accent). The surface's
face stops being Claude Code's: the caret `▍` is bingo's mark and voice,
embers are the token meter, the opening is the wordmark, the accent is
ember amber, the words are bingo's own. The grammar of the hands —
cards, keys, the esc stack, `⏺` tool rows, `⎿` results — is unchanged.
No new dependency, no sdk change.

## Re-verification (2026-10-14, `ed98a22f`)

M108–M136, M147 and M148 are present; M125 and M148 satisfy the hard
dependencies. The worktree started clean on `m149-bingos-own-light`.
`render::Render` scopes the look; `Focus::Intro(Playing)` owns the opening.
`layout/deadline.rs::next` collects `activity::next_change`; `view.rs`
draws the foot on the air row. `blocks.rs::welcome` composes the opening.
The named palette, glyph, motion, transcript and code functions still exist;
Markdown fences already share `views/code.rs`. No separate fence renderer.
M121/M123 replaced private task reads with `Slot::Activity { verb, view }`:
brick 1's earned Bingo row needs a boundary decision (see Risks).
ADRs checked: 0001, 0002, 0070 and 0081; no acceptance change is due here.

## Bricks, in build order

1. **Failing tests first** (TUI):
   - `theme`: the palette snapshot holds `presence #f0883e / glow
     #ffc278` (dark) and `#a84d12 / #e07a2a` (light), ANSI unchanged;
     the readable-on-ground test passes for both.
   - glyphs: the mark `▍`, the thinking mark `▏`, ASCII `|` for both; no
     `✻ ✢ ✶ ✽` in the glyph tables.
   - `motion`: the caret blinks at 530 ms before the first byte, is
     steady `glow` while words arrive, breathes at 2.2 s while a tool or
     an agent holds the turn, dims when stopping (L2).
   - embers (L3): a pure function of the token count's crossings of 64
     and `now`: at most four on the air row, each gone 0.6 s after it
     left, none under `BINGO_MOTION=off`, none while the foot holds the
     row, and the band's height unchanged in every case.
   - opening (L6): the wordmark rows are exactly the site's bitmap in
     half-blocks with the caret at the end; under 40 columns or 16 rows
     the mark is `bingo▍`; frames at 0.0, 0.5, 1.2, 2.0 and 2.4 s.
   - worked row (L7): `▍ Bingo · 5 tasks done in 4m 12s` when the turn
     ends with every task on the list done and one finished in it;
     otherwise `▍ Worked for …` as today.
2. **The accent** (`theme.rs` `DARK`, `LIGHT`). Ramps derive from the
   two tokens as before; nothing else in the palette moves.
3. **The mark.** `theme.rs` glyph tables: `mark` replaces `sparkles`;
   `spark()`/`sparkle()` and `SPARKLE_MS` go. The window title
   (`run.rs`), the worked row (`transcript.rs`, `worked.rs`) and
   thinking (`transcript/thinking.rs`, `▏ Thinking…`, `▏ Thought for
   2s`) take it.
4. **The caret's states** (`activity.rs` `working`, `breathing`): the
   table of L2, one function from the turn's phase to the caret's
   style at `now`.
5. **Embers** (`embers.rs`, new): the pure function of brick 1 and its
   drawing on the activity band's air row. What the surface keeps is the
   instants of the last four crossings, per turn, cleared at its end.
6. **bingo speaks with the caret** (`transcript.rs` `assistant`): the
   model's block opens with `▍`, bold `text` at rest, `presence` while it
   streams; the comet tail is unchanged. Tool, child and post rows keep
   their bullets.
7. **The opening** (`welcome.rs`, `opening.rs`, `opening/*`): no box.
   The wordmark, then the facts row (`model · effort · mode · cwd`), then
   the resume row when this cwd has an earlier session (read off the
   loop; the row is held either way so nothing jumps), then M63's update
   row. The piece: the caret types the mark a column every 16 ms, blinks
   twice, throws three embers, rests; the facts come in on the beam. A
   frame is a function of one second, as §11 has it. `GREETING`, `HELP`
   and the `/login codex` line go.
8. **Words** (`activity.rs` verbs; `worked.rs`): Kindling, Stoking,
   Smouldering, Foraging, Tracking, Burrowing, Sniffing around,
   Pouncing, Sifting, Tinkering; the Bingo row of brick 1.
9. **The code band** (L8; `views/code.rs`, `markdown.rs` fences): the
   block on the `raised` tint its own width, the language dim at the
   band's right end; no lone language row.
10. **Records.** tui.md §1 (the face is bingo's, the hands Claude
    Code's), §4 tokens and glyphs, §6 motion (caret, embers), §11
    rewritten for the wordmark, §10 dated entry; tui-identity §3 ticked.

## Files

- `crates/bingo-surface-tui/src/{theme,activity,embers,transcript,worked,
  welcome,opening,run,markdown,motion}.rs`, `opening/*`,
  `transcript/thinking.rs`, `views/code.rs`, `render.rs`
- TUI wiring: `lib.rs`, `ui.rs`, `view.rs`, `blocks.rs`, `layout.rs`,
  `layout/deadline.rs`, `run/opening.rs`; `focus.rs` only if opening
  deadlines need adjustment. Resume data must be read on the host-call lane.
- TUI tests: `screens/*.rs`, `painted.rs`, `motion/*`, `layout/tests.rs`;
  mechanical removed-glyph expectations in `select.rs` and `search.rs`
- `crates/bingo-surface-tui/src/{screens,snapshots,…}/**` (snapshots)
- `crates/bingo/tests/pty/**` only where a needle names a removed glyph
- `scripts/tui-smoke.sh` (`* Hello in ascii.` becomes `| Hello in ascii.`)
- `docs/design/tui.md`, `docs/design/tui-identity-2026-10.md`
- M137's plugin host/runtime, plugin-rpc, manifests and schemas are excluded.

## Exit criteria

- [ ] Each brick-1 test failed first (output pasted) and passes now.
- [ ] `rg '✻|✢|✶|✽|d97757|f2a07c|Welcome to bingo|Simmering' crates/`
      finds nothing.
- [ ] Changed snapshots listed by family (welcome, activity, worked,
      thinking, model block, code) with a count; no other family moved.
- [ ] A tmux drive at 120×40, 80×24 and 38×14, dark and light (fake
      OSC 11 ground), `NO_COLOR`, `BINGO_ASCII=1`, `BINGO_MOTION=off`:
      the opening, a streaming answer with embers, a tool holding the
      turn, a stop, the worked row and a Bingo row; captures pasted.
- [ ] `scripts/tui-smoke.sh` passes.
- [ ] All gates of the upgrade protocol, plus the Windows cross-check
      for the TUI.

## Non-goals

- Agents' sigils and hues, the crew strip, rooms as chat (M150).
- The site's `tokens.ts` and hero (`../bingo-site`) and the GUI, each in
  its own repository: the person is told the two tokens to carry over.
- Inline code's backticks (M73's choice stands).

## Risks

- About 200 snapshots move. A diff that moves a row, not a glyph, is a
  defect of this milestone: §3's *nothing jumps* holds through the
  opening and the embers, and the band tests pin heights.
- `▍`, `▏` and the half-blocks are ambiguous width, like the borders the
  frame already draws; `BINGO_ASCII` has none of them.
- Re-verified: the opening still uses `beat`, `cells`, `frame`, `lap` and
  `storyboard`; replacing the box removes the perimeter lap. Keep `Playing`
  and the 2.4 s deadline. The compact mark needs viewport height passed
  through the layout/rows path; welcome currently receives width only.
- **Stopped at protocol step 6 (2026-10-14), before tests or code.** L7
  cannot be implemented within the stated TUI-only/no-sdk-change boundary
  without violating ADR-0070 §§2,5,7 or ADR-0001's dependency direction.
  `bingo-tasks/src/journal.rs:58–106` publishes only an Activity view:
  counts are prose in its Panel title, completed nodes may be omitted,
  and no field identifies a completion in the last turn. `task.rs:15`
  has no completion turn/time. `bingo-sdk/src/slot.rs:19` has only `verb`
  and `view`; `state.rs:76`'s LastTurn has no plugin completion fact.
  `transcript.rs:909` renders the worked row from LastTurn alone. No
  task-completion contract exists in `bingo-services` or `bingo-collab`.
  Reading private payloads/tool names or inferring completion from view
  prose would restore the coupling M121/M123 removed; importing the
  tasks plugin is forbidden. Retaining task state in Ui is not a remedy.
  **Options:** (A, recommended) expand M149 with a producer-owned,
  turn-scoped completion contract and its consumer, documented boundary,
  contract-first tests and any schema/fixture generation; coordinate
  shared schema work with M137. (B) explicitly defer the earned Bingo
  row and retain the generic worked row. Neither option is applied.
  All exit criteria remain unticked; no gates or live tests were run.
