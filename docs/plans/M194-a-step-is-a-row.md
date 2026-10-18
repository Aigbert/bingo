# M194 — A step is a row

ADR-0097 §4, §6, §10; ADR-0095 as amended; ADR-0084 §1. Spec: `docs/design/ember/`
(`p4-model.js` `TOOLS`/`markOf`/`verbOf`; `p5-stream.js` `turnLines` … `answerLines`).
After: part A nothing; part B M192, M201 and part A, rebased onto M193.

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
   - **Clock:** the `TurnDuration` site on the band's first row from 1 s, read from M201's
     `clock` for every band alike, so time spent waiting for you is left out; muted while live,
     dim once done; a stopped turn reads `stopped · 4s`. While live it keeps 10 cells free so
     your words never rewrap.
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
   - **Why:** from M201's facts: `accept edits` or `bypass` for a mode, `session rule` for a rule
     added this session (a settings rule or a hook says nothing), and `changed by you` for a
     call allowed after you changed it.
4. **Narration and the live row** (`narrLines`, `liveLines`, `stalled`). Narration is italic
   muted between the steps where it was said, and grows as it streams (the prototype's 70 cps
   stands in for the stream). The live row ends a live turn and is the `Spinner` site, so a mod
   still draws it:
   - `thinking` shimmers while a thought is open and nothing runs (a `Running` call or an open
     ask), with its clock on the right from 1 s;
   - `working`, or the placed task's verb as written, once no item has started or ended for
     500 ms while the turn is live, no answer has begun (narration with a step after it is not
     an answer) and no retry shows; the frame wakes when the 500 ms is due;
   - `stopping` from the esc frame until `TurnCompleted`, also while a step runs or the answer
     streams (ADR-0097 §4; the prototype stops at once).
   The dock's working row (` ▍ burrowing · 7s`, `activity.rs`), its verbs and the
   `TurnProgress` site retire, and a mod naming it gets a load error; queued rows stay until M196.
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
- TUI dev-dependencies reuse workspace tool-fs, tool-bash, tool-web and skills to load the real test catalogue; no new package.
- A tool look belongs beside its spec; a Look fixture in test_support would duplicate it.

## Exit criteria

- [x] **A:** contract, resolution and catalogue tests; provider encodings unchanged; schemas
      regenerated; each built-in tool's look pinned by one test per crate.
- [x] **B:** the stream band of `live`, `done`, `stop`, `steer`, `steer-landed`, `ascii-live`,
      `ascii-done`, `reduced-live`, `small-done` and `narrow` passes the harness.
- [x] **B:** motion budget on `live`: between frames 500 ms apart, `moved` reports only the side
      light, the spinner cell and the comet.
- [x] **B:** M194 lands after M193, so it also passes the full frame of `live`, `done`,
      `ascii-live`, `ascii-done`, `reduced-live`, `small-done`, `mode` and `mode-later`.
- [x] **B:** the live row: `working` at 500 ms and not before, its deadline exact; the placed
      verb; `thinking` and its clock; `stopping` on the esc frame while a step runs, gone at
      `TurnCompleted`; a mod's `Spinner` answer redraws the row; `TurnProgress` refused at load.
- [x] `--print` and RPC are unchanged except the additive `look`; their black-box tests pass
      untouched. Today's turn snapshots are deleted or re-pinned, each deletion named in Verified.
- [x] Gates and checks as in M192; tmux drive of a live, a done and a stopped turn (Windows toolchain limits below).

## Non-goals

Detail, wells and find (M195); queued rows and keys (M196); asks (M197); the teammate's name in
its colour and agent rows (M200); Browse; the retry row's facts (M199).

## Risks

- **Old journals:** a resumed session whose tools are gone reads `•` rows. Accepted: the look
  lives with the tool, not in the journal.
- **Per-call traits:** bash's read-only verdict depends on the command, so the baseline tier
  reads every command as a change, as the prototype shows.
- **Part A's TUI touch** stays in `turns::step`, so it does not collide with M192's theme split.

## Verified

- [x] **A1–A2, A5:** `cargo test --workspace --locked` (provider credential variables removed): `6420 passed; 0 failed; 0 ignored`; contract, 19 row fixtures, nine ACP kinds, typed catalogue accessors and fail-closed tier pass.
- [x] **Provider encodings:** same command: both `request::tests::a_tool_look_never_reaches_the_model ... ok`.
- [x] **A3:** same command: all pinned plugin looks pass; `checked_look` checks verbs, mark/ASCII resolution and target-schema properties, including SendFile, MCP and HelloProgress.
- [x] **A4:** same command: `malformed_tool_looks_refuse_loading_in_words ... ok`; `a_registered_tool_is_prefixed_and_offered_by_the_catalogue ... ok` (JS registration through Tools catalogue).
- [x] **A4, A6 schemas:** `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk`, then `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-mods -p bingo-plugin-rpc -p bingo-surface-rpc`: `282 passed` (SDK), `386 passed` (wire crates), `0 failed`; regeneration after rebase leaves no diff.
- [x] **TS SDK:** `node --test sdk/typescript/src/*.test.ts`: `tests 33; pass 33; fail 0`.
- [x] **A5–A6 rows:** `cargo test -p bingo-surface-tui tool_looks_refresh_when_the_tools_catalogue_changes`: `1 passed; 0 failed`; screen changes `fixture needle` → `find needle`. Removing the reset: `0 passed; 1 failed` (`cached rows: left 1, right 0`); restored: `1 passed; 0 failed`.
- [x] **Failed start:** `cargo test -p bingo-plugin-rpc a_failed_start_announces_only_the_plugins_catalogue`: `1 passed; 0 failed`; before fix: `[Plugins, Commands, Tools] != [Plugins]`.
- [x] **Print/RPC:** unchanged black-box tests in workspace run: CLI `306 passed`, RPC `104 passed`, PTY `19 passed`; all `0 failed`.
- [x] **Post-rebase native gates:** `cargo fmt --all -- --check`: exit 0, no output; `cargo check --workspace --all-targets --locked`: `Finished dev profile`.
- [x] `cargo clippy --workspace --all-targets --locked -- -D warnings`: `Finished dev profile`; `cargo test --workspace --locked`: `6420 passed; 0 failed; 0 ignored`.
- [x] `scripts/check_discipline.sh`: `discipline ok`; `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.
- [x] `scripts/budget.sh` (approved workspace unicode-width edge; no new package):
  ```text
  dependencies (unique, normal): 423 (max  423)
  warm cargo check -p bingo-core: 0s (max  20s)
  relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
  target/debug: 17 GB (soft max  5)
  warn: target/debug exceeds the soft limit
  test binaries: 232
  budget ok
  ```
- [x] **Drive:** `docs/verification/m194a/drive.sh`: `100x32: live, done, stopped passed`; `80x24: live, done, stopped passed`; asserts `✓ run cat note.txt`. `TUI_SMOKE_SOCKET_PREFIX=m194-drive scripts/tui-smoke.sh`: `tui-smoke ok`.
- [ ] **Windows:** `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc`: `14 passed, 10 failed` ([output](../verification/m194a/windows-review.txt)). `bingo`, `bingo-channels`, `bingo-mcp`, `bingo-mods`, `bingo-plugin-rpc`, `bingo-provider-anthropic`, `bingo-provider-openai`, `bingo-surface-tui`, `bingo-tool-fs` and `bingo-tool-web` remain owed: aws-lc-sys lacks `stdlib.h` / `windows.h`; mods also rejects MSVC `/DWIN32_LEAN_AND_MEAN` flags. No native build bypassed.

- [x] **Rebase:** `git rebase dev` onto `49450a61`: `Successfully rebased and updated refs/heads/m194a-tool-look`; no conflicts.

Departures and outstanding scope:

- Extra looks: BashOutput `$`/output; ShowPage `?`/show; OpenRoom `@`/open; Listen `@`/listen; ListModels `@`/models; SendFile `@`/send (path `path`); HelloProgress `#`/progress.
- Agent verbs at `@`: SpawnAgent spawn, ListAgents list, SendMessage send, StopAgent stop, DismissAgent dismiss, SetThinking thinking.
- ACP: Read `¶`/`<`, Edit `±`/`~`, Delete `×`/`x`, Move `→`/`>`, Search `/`, Execute `$`, Think/Other `•`/`-`, Fetch `↓`/`v`; kind verb, title target as text.
- A2's unspecified “short”: nonempty single-line ≤120 Unicode scalars, JSON object order. Approved unicode-width edge justified under Files.
- Files omitted channels/hello specs and ACP catalogue-to-spec copy. A6 also needs asks, pager and child rows; A5 needs actual JS/RPC mutation notifications. All included; no prototype conflict.
- No snapshots deleted; re-pinned `screens__message_tools_{120x40,80x24}` and `screens__bridged_call_while_streaming_{120x40,80x24}` for lowercase fallback names without inserted spaces.
- Part B, theme/row_mark, six Looks::in_ui sites and transcript::called splitting remain Part B's work; no Ember captures edited. Combined gates criterion stays open for the Windows native-toolchain blocker.
- Inherited provider credentials break an unrelated auth test (reproduced alone); verification removes the four provider key/base-URL variables.
- Existing CLI recovery test hung once after rebase; terminated only its owned child. `cargo test -p bingo --test cli agents::a_resumed_root_is_told_which_child_was_mid_turn_when_the_process_ended -- --exact`: `1 passed; 0 failed` unchanged.

## Verified — Part B

- [x] B1–B8, live-row decisions and regressions: `cargo test --workspace --locked`: **6535 passed; 0 failed; 0 ignored**; TUI **1427 passed**. No wall-clock retry was needed.
- [x] Capture harness in that run: `ember_stream_bands_match_the_prototype` (all ten), `ember_full_frames_match_the_prototype` (all eight), and `ember_live_motion_is_only_side_light_spinner_and_comet ... ok`; M193's criterion links here.
- [x] Motion samples 1400→1900 ms: only side light (0,19), declared spinner (3,25), and edge comet (row 28) may move; caret rests at 10 s. Off-screen rows and inline-card-hidden Spinner requests/deadlines are pinned.
- [x] Before→after: plain inline code, numeric/missing table cells, finished displays and independent answer/why fail before their fixes, then pass. Seven reverted mutations each give `0 passed; 1 failed`: breath period, fade default, why omission, Spinner bounds, raw tail deadline, skill guard, reduced-motion blend.
- [x] Exact breath ink/deadline, EaseOut weight, linear fades, reduced/finished answer ink, 102/57-cell prose extents, running child, visible/off-screen Spinner keys and both retired-site engine refusals pass in the workspace run.
- [x] `--print`/RPC black-box tests: CLI **309 passed**, RPC **104 passed**, PTY **19 passed**; `protocol_two_keeps_every_frozen_shape ... ok`. Schemas/TS regenerated with `BINGO_UPDATE_SCHEMA=1` at both contract addition and retirement; no hand merges.
- [x] `cargo fmt --all -- --check`: exit 0; `cargo check --workspace --all-targets --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings`: `Finished dev profile`; `cargo test --workspace --locked`: above.
- [x] `scripts/check_discipline.sh`: `discipline ok`; `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.
- [x] `scripts/budget.sh`: `dependencies (unique, normal): 423 (max 423)`; `warm cargo check -p bingo-core: 0s (max 20s)`; `relink isolation: touching the TUI recompiled 0 crates for core`; `target/debug: 23 GB (soft max 5)` (warning); `test binaries: 204`; `budget ok`.
- [x] Windows: `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc`; this round sdk passes, TUI stops at aws-lc missing stdlib.h/windows.h. Unchanged preceding checks: mod-ledger/skills pass, mods stops at rquickjs-sys MSVC flags, plugin-rpc at aws-lc Windows headers. Accepted native-toolchain stops, built by CI’s windows job; [output](../verification/m194b/windows.txt).
- [x] `TUI_SMOKE_SOCKET_PREFIX=m194b-drive scripts/tui-smoke.sh`: `tui-smoke ok`; `bash docs/verification/m194b/drive.sh /tmp/m194b-r6/drive`: **100x32** and **80x24: read upshot, clock, working, live, done, stopping, stopped passed**. Finished output and all forbidden rows absent; server killed; captures stay outside the repository.
- [x] No snapshots changed in this round. Every earlier re-pin/deletion is in [snapshots](../verification/m194b/snapshots.md); every test retirement/rename in [retired tests](../verification/m194b/retired-tests.txt). Immutable scenes unchanged; only dev's eight `assertion_line` entries remain. Plain backticks restored; ANSI code ink exists, so this correction leaves `ascii_idle` unchanged.
- [x] Deleted `theme::row_mark`, old tool output/row/fold drawing, duplicate markdown/highlighter paths and the cache reset. Looks decode once per frame; `look::mark` owns the mark rule. Remaining connector owners and syntect cells: [audit](../verification/m194b/audit.md).
- Drawing scans: History/Answers, Roles and Blocks each visit the transcript once per frame; band/narration/failure/additions read indexed turn items. Sparks scan once per paint; child summaries once per child; detail/browse/rewind scans are on demand; emptiness checks and indexed Blocks reads are O(1).
- Fade declarations easing out are exactly SDK `component/ledger/draw.rs` (landing, 420 ms) and TUI `transcript/band.rs` (done light, 320 ms); steered/said-no notes and absent curves stay linear. Arrival, not time, advances the answer tail.
- Departures: tables retain M11e numeric alignment and missing `–` over `tableLines` because no scene captures a table; why separators occur only between present parts, unlike `whySegs`' unconditional prefix.
- Departures: D2 passes `stop_asked` through Rows, recomputed every sync; four TUI dev-dependencies load each tool's authoritative look beside its spec, avoiding a duplicate fixture; no new package. Dev's inherited plan already exceeds 150 lines; Part A is preserved and this section stays under 40.
- M201 clock/reasons are integrated. Steers still derive from turn items (no receipt); said-no uses receipt facts. Progress remains the call's tail. M202 owns the actual Running-frame start/order; the receipt/start approximation is commented.
- M195 opens a step's display in its well and moves pictures there; closed finished steps hang nothing. M196 owns shell/image/chip/queued rows; M197 typed question answers; M198 overlaid layers/login; M199 retry facts; M200 child rows and view ink for landing, answer caret/blend and live glyph.
- Shims checked at `e6daad25` (now `9429a320`): Step `why`/`elapsed_ms`/`landed_at` are None in TUI `tool_rows.rs:61–63`, `turns.rs:334–336`, and mod-ledger `draw.rs:103–105`; TUI `views/ember_tests.rs` Fade declarations at 149/341/471 set `curve: None` at 150/342/472. Old helpers/sites and six Marks fields survive through commit 6; commit 7 removes them.
- [x] Shared inline code: plain_prompt_and_steered_code_keep_backticks_and_their_own_ink: before 0 passed; 1 failed, after 1 passed; helper tests pin plain/default ink. Production foreground("code") appears only in markdown/inlines.rs::code; prompt note predicate is evaluated once.
- [x] Removed markdown::answer/alias test, retained literal table lines in an_empty_cell_keeps_its_dash_beside_a_numeric_column; existing table tests pass unchanged. Cells render once; running duration and quiet-call predicates each have one helper. Measure docs and frame-local mode_labels visibility/docs corrected.
- [x] Clock assertions pin 7150 ms/next 9850 ms, 650 ms/no next, 500 ms/no next through both entries; a shared +1 s mutation fails (8150 != 7150) and is restored. Running-call test pins zero clamp, 999 ms and exact next second; elapsed-ms docs regenerated into schemas/TS.
- Range-diff against 295bd047: commits 1/2/3/6/10 change intentionally; 7 differs only in the surrounding function name (running_for); 4/5/8/9 are unchanged. No new design departure. A preliminary fmt completion was collected after starting the targeted test; all final gates/proof jobs are serial.
- Per-commit proof uses detached `cargo check --workspace --all-targets --locked`, then returns to `m194b-step-rows`; each result below is exit 0 (`Finished dev profile`).
- `327835de` markdown/code: exit 0 (`Finished dev profile`).
- `32d5b057` clock core: exit 0 (`Finished dev profile`).
- `9429a320` additive SDK contract: exit 0 (`Finished dev profile`).
- `99eadc07` skill looks: exit 0 (`Finished dev profile`).
- `bf933324` mod-ledger adapter: exit 0 (`Finished dev profile`).
- `4f2e85c6` TUI switch: exit 0 (`Finished dev profile`).
- `05cfdfc5` retirement: exit 0 (`Finished dev profile`).
- `c0f3b37c` production scanner: exit 0 (`Finished dev profile`).
- `301005be` drive: exit 0 (`Finished dev profile`).
- This docs commit: exit 0 (`Finished dev profile`).
