# M194 — A step is a row

ADR-0097 §4, §6, §10; ADR-0095 as amended; ADR-0084 §1. Spec: `docs/design/ember/`
(`p4-model.js` `TOOLS`/`markOf`/`verbOf`; `p5-stream.js` `turnLines` … `answerLines`).
After: part A nothing; part B M192 and part A. Part B runs beside M193.

## Goal

A turn reads as its captures read: your words on a band (side light at column 0, clock on the
right), each step one row with its own tool's mark, narration where it was said, and the answer
within 100 columns. Nothing folds, no stop line is drawn, no activity row remains. A tool's owner
says how its row reads, and no surface names a tool.

## Part A — the look (SDK, tools, catalogue; starts at once)

1. **The contract.** `bingo-sdk` gains, beside `ToolSpec`:
   ```rust
   pub struct ToolLook { pub mark: char, pub ascii: Option<char>, pub verb: String,
                         pub target: Option<Target> }
   pub struct Target { pub field: String, pub reads: Reads }   // a top-level input key
   pub enum Reads { Path, Command, Text }
   ```
   `ToolSpec.look: Option<ToolLook>` is skipped when absent and never sent to a model. Fixture
   and schema tests come first; the anthropic and openai request tests assert that a spec with a
   look encodes exactly as the same spec without one.
2. **One resolution, in the SDK.**
   `look::row(name, input, declared, ascii) -> RowLook { mark, verb, target: Option<(String, Reads)> }`.
   - **Mark:** the declared mark (its ASCII twin first under ASCII) if it is one cell wide
     (unicode-width) and, under ASCII, ASCII; otherwise `•`, or `-` under ASCII.
   - **Verb:** the declared verb, else the tool name in lower case.
   - **Target:** the declared field of the input, else the first short string value, as text.
   - **ACP calls** (`{agent}:{kind}`) take the look `CallKind` declares beside itself in
     `agent.rs`: the verb is the kind, the target the call's title.
   - One fixture test per case; the case list is in the doc comment.
3. **Declarations** sit beside each spec, as SPEC.md "Tool marks" lists them:

   | Crate | Tool | Mark | Verb | Target |
   |---|---|---|---|---|
   | fs | Read | `¶` (ASCII `<`) | read | path `file_path` |
   | fs | Glob | `*` | find | path `pattern` |
   | fs | Grep | `/` | search | text `pattern` |
   | fs | Edit | `±` (`~`) | edit | path `file_path` |
   | fs | Write | `+` | write | path `file_path` |
   | fs | AskUserQuestion | `?` | ask | — |
   | bash | Bash | `$` | run | command `command` |
   | bash | KillShell | `×` (`x`) | stop | — |
   | web | WebFetch | `↓` (`v`) | fetch | path `url` |
   | web | WebSearch | `»` (`>`) | web | text `query` |
   | tasks | every tool | `#` | plan | — |
   | skills | every tool | `§` (`&`) | skill | — |
   | schedule | every tool | `∞` (`^`) | wake | — |
   | experience | every tool | `√` (`!`) | recall | — |
   | mcp | every tool | `◊` (`%`) | the server's name | — |
   | rooms | each verb | `@` | seat, unseat or close | — |
   | agents | every tool | `@` | its verb | — |

   A tool the table leaves out (BashOutput, ShowPage, ListModels …) takes a one-cell mark of the
   same family, listed in Verified for review. `bingo-pictures` registers no tool, so `¤` waits.
4. **Outside mods.** `look` rides the plugin wire's `ToolSpec` (regenerate `schema/plugin.json`
   and `wire.ts` with `BINGO_UPDATE_SCHEMA=1`); JS mods pass it to `$.tool.register` and the TS
   type gains it. A malformed look (empty verb, a mark that is not one character) is refused at
   load with a message, never drawn.
5. **The catalogue carries it.** `host/catalog.rs` `tool_entry` puts `look` into the entry's
   meta. A surface reads looks from the Tools catalogue and refreshes on `CatalogChanged{Tools}`.
6. **`Step` carries the look.** Ledger props `Step` gain `mark: char`, `reads: Option<Reads>`
   and `tier: Tier::{Lookup, Change}`. The tier comes from the catalogue's baseline
   `ToolTraits.read_only` and fails closed (unknown or untrusted is `Change`). `turns::step`
   fills `Step` from `look::row`; `turns::verb` and `summarize`'s key list go. The golden
   `component/props.json` and the RPC schemas regenerate.

## Part B — the turn (TUI; after M192 and part A)

1. **No fold.** A running and a finished turn draw the same rows. Retired: `Role::Fold` and
   fold-hiding (`turns.rs` holder, `line_of`), `TurnFoldProps`, `Run`, the `TurnFold` site with
   mod-ledger's `fold`, the ▸/▾ marks, `input/turn.rs`, the turn branch of `pointer.rs`, and the
   stop line (`transcript/steps.rs`). An outside mod naming `TurnFold` gets a load error saying
   the site retired (ADR-0097), never a silent no-op. `Marks` keeps only `asked`.
2. **The band** (`bandRows`, `promptSegs`, `clockSegs`, `sideInk`).
   - **Side light:** breathes glow↔coal over 1.8 s while live, amberGlow while asking. Only the
     newest finished turn stays lit: good, bad, or dim when stopped.
   - **Clock:** the `TurnDuration` site on the band's first row from 1 s; muted while live, dim
     once done; a stopped turn reads `stopped · 4s`. While live it keeps 10 cells free so your
     words never rewrap.
3. **Step rows** (`stepRow` … `whySegs`, `tailLine`). Mod-ledger draws them from `Step`, each row
   asked of `Ledger` on its own, with M192's view vocabulary; what moves is declared as motion.
   - **Layout:** mark at 3, verb at 5, target at `max(13, 5 + verb + 2)`, outcome right, RM 2.
   - **Tiers:** a lookup has a muted mark, dim verb, muted target; a change an fg mark, muted
     verb, fg target. A command reads in `code`; a path is shortened by `short_path`.
   - **Running:** the spinner in the mark cell in glow, elapsed time from 1 s, the last output
     line under the target. Once done, the mark lands glow→ink over 420 ms.
   - **Asked:** `◆` breathes amber, reading `needs you`. **Failed:** mark and verb bad, the
     reason on the right. **Skipped or cut:** faint mark, dim verb and target, `said no` or
     `stopped` on the right.
   - **Why:** the reason a step ran without asking (`accept edits`, `session rule`) comes from
     the policy; a step you changed reads `changed by you`.
4. **Narration and live rows** (`narrLines`, `liveLines`, `stalled`). Narration is italic muted
   between the steps where it was said; the newest types out at 70 cps until something follows.
   While nothing runs, `thinking` shimmers, and after a 500 ms pause it reads `working`. The
   working line (` ▍ burrowing · 7s`, `activity.rs`) goes; queued rows stay until M196.
5. **Ends** (`failsTurn`, `failTail`, `errorLines`, `sparks`). A turn ending on a failed step
   keeps its last 4 output lines in a well; a recovered failure is one red row. Done: glow→good
   over 320 ms and three sparks (700 ms, empty cells only). The edge flash is M193's.
6. **What you added** (`saidLines`, `youLine`). A steer, or the reason behind a no, stays under
   your words on the `you` band, labelled `steered` or `said no` (glow→dim, 420 ms).
7. **The answer** (`mdLines`, `blockLines`, `codeLines`, `tableLines`): markdown at column 3
   within the measure, `•` bullets with a hanging indent, code blocks on `well` with the
   language on the right.
8. **Check 4c** scans `bingo-surface-tui` and `bingo-mod-ledger` too, matching quoted tool names
   only (a type such as `Seat` passes) and skipping `#[cfg(test)]` modules.

## Files

- **A:** `crates/bingo-sdk/src/{model.rs,look.rs,agent.rs,component/ledger.rs,component/props.json}`;
  the specs of `crates/bingo-tool-{fs,bash,web}` and `bingo-{tasks,skills,schedule,experience,
  mcp,rooms,agents}`; `crates/bingo-core/src/host/catalog.rs`; `crates/bingo-plugin-rpc`;
  `sdk/typescript`; the provider request tests.
- bingo-sdk takes the workspace's unicode-width for the one-cell test of a declared mark; no new package.
- **B:** `crates/bingo-surface-tui/src/{turns.rs,turns/,blocks.rs,transcript.rs,transcript/,
  activity.rs,fold.rs,input/turn.rs,pointer.rs,markdown.rs,views/code.rs,component/closing.rs}`;
  `crates/bingo-mod-ledger`; `crates/bingo-sdk/src/{render.rs,component/ledger.rs,
  component/row.rs}`; `scripts/check_discipline.sh`.

## Exit criteria

- [ ] **A:** contract, resolution and catalogue tests; provider encodings unchanged; schemas
      regenerated; each built-in tool's look pinned by one test per crate.
- [ ] **B:** the stream band of `live`, `done`, `stop`, `steer`, `steer-landed`, `ascii-live`,
      `ascii-done`, `reduced-live`, `small-done` and `narrow` passes the harness.
- [ ] **B:** motion budget on `live`: between frames 500 ms apart, `moved` reports only the side
      light, the spinner cell and the comet.
- [ ] **B:** whichever of M193 and M194 lands second also passes the full frame of `live`,
      `done`, `ascii-live`, `ascii-done`, `reduced-live` and `small-done`.
- [ ] `--print` and RPC are unchanged except the additive `look`; their black-box tests pass
      untouched. Today's turn snapshots are deleted or re-pinned, each deletion named in Verified.
- [ ] Gates and checks as in M192; tmux drive of a live, a done and a stopped turn.

## Non-goals

Detail, wells and find (M195); queued rows and keys (M196); asks (M197); the teammate's name in
its colour and agent rows (M200); Browse; the retry row's facts (M199).

## Risks

- **Old journals:** a resumed session whose tools are gone reads `•` rows. Accepted: the look
  lives with the tool, not in the journal.
- **Per-call traits:** bash's read-only verdict depends on the command, so the baseline tier
  reads every command as a change, as the prototype shows.
- **Part A's TUI touch** stays in `turns::step`, so it does not collide with M192's theme split.
