# M197 — Asks open in the dock

ADR-0097 §8; ADR-0084 (the Permission site stays sealed). Spec: `docs/design/ember/`:
- `p6-dock.js`: `askLines`, `optionLine`, `cmdBody`, `diffBody`, `questionLines`, `tabRow`,
  `previewPane`, `pickLine` (the guard);
- `p4c-act.js`: `askKey`, `leaveAsk`, `askTyping`, `answerAsk`, `changeAsk`, `deny`,
  `questionKey` … `finishQuestion`;
- `p4b-data.js`: `ASK`, `QS`.

After: M193 and M194.

## Goal

When bingo needs a yes, the dock asks in the input's place:
- what it wants to do, in full, with a plain ↑↓ list of answers;
- typing means "No, and tell it why";
- a dangerous call says what it costs and starts on No;
- a question from the model has tabs, a reason for each choice, a preview beside it, and your
  own words last.

## Bricks

1. **The ask** (`askLines`). The dock body is drawn by the surface, because the site is sealed.
   - **Title:** bold, at TX, from the preview:
     - a diff reads `<Verb> <last two path parts>?`, verb from the tool's look, with `+N −M` on
       the right;
     - a command reads `Run this command?`, with `in <cwd, ~ for home>` on the right;
     - a URL reads `Fetch <host>?`;
     - anything else reads `<Verb>?`, with today's `summary` as the body.
   - **Body:**
     - a diff is the well of M195 on the `you` ground, at most 8 lines, then `… N more lines`;
     - a command is `$ ` dim and the whole command in `code`, wrapped and never cut, then the
       preview's `description` muted.
2. **The answers** (`optionLine`). They come from today's rungs, widest last:
   - `Yes`;
   - `Yes, allow <scope> for this session`, where the scope reads in words and in `code` when
     picked;
   - `No, and tell it why`, with the hint `type to tell <worker> why`;
   - `Change it first`, offered when the policy allows a replacement, with the hint
     `⏎ to edit the command`.

   The scope is read from the rule by a pure function with its own tests:
   - `Bash(cargo test:*)` reads `cargo test`;
   - a path rule reads `edits in <last three parts>/`;
   - a URL rule reads `fetches from <host>`;
   - anything else reads as written.
3. **Keys** (`askKey`).
   - ↑↓ chooses, ⏎ confirms.
   - Typing a printable character moves to `No, and tell it why` and starts its text. On
     `Change it first`, ⏎ starts with the command.
   - While typing, ⏎ sends and esc goes back to the list.
   - esc on the list says no. At home it ends the turn (M196's halt, queued text back);
     in a teammate's session it only goes home (M200).
   - **The guard:** for 400 ms after an ask opens, every key is swallowed, and the picked row's
     ground fades from `you` to `pick` over that time.
   - The 1–9 digit answers go (ADR-0097 §8: ↑↓ lists, no letter keys).
   - Pairs: `↑↓ choose`, `⏎ confirm`, `esc stop` (`esc back` away from home); while typing,
     `⏎ send`, `esc back`.
   - Allowing for the session toasts `✓ allowed <scope> for this session`.
4. **A dangerous ask** (`ASK.caution`).
   - It is a permission whose `caution` is set. The edge is bad and still (M193 reads it), the
     title reads bad, and the caution sits on the right in bad.
   - The pick starts on No. The allow item reads `Yes, <act>` in bad, and no session rung is
     offered (the gate already drops it).
   - The caution fact gains its act. `InteractionKind::Permission.caution` becomes
     `Option<Caution { reason, act: Option<String> }>`, and a bare string in an old journal
     still reads as `{ reason }`. A fixture test of both forms comes first.
   - `bingo-permissions/caution.rs` names the act and the reason for each pattern it knows.
     A force push is `force-push` and `rewrites remote history`.
   - **What it drops.** `Preview::Command` gains `stakes: Option<Stakes { line, items }>`,
     skipped when absent. The bash tool's `preview` fills it for a force push of
     `<remote> <branch>` only: the commits `git log --oneline <branch>..<remote>/<branch>`
     lists, at most 5, through the crate's existing process helper, bounded to 500 ms, and
     nothing on failure.
   - `stakes` draws as the line in bad, then each item with its first word muted and the rest
     dim, as `caution` shows.
5. **The question** (`questionLines`), owned by `bingo-mod-ask`, which draws its pulled view.
   - With more than one question, a tab row: done tabs `✓` good, the current one bold, and
     `N of M` on the right.
   - The question bold. A single question has its header dim on the right.
   - Choices are a pick list with descriptions at `labW`. A multi-select choice is ticked with
     `✓` good or `·` dim. `Something else` is last.
   - At 90 columns or more, the picked choice's preview sits on the right on `wellDeep`, at most
     10 lines, running below the list when it is longer.
6. **Question keys** (`questionKey`).
   - ←→ moves between tabs, ↑↓ between choices, and space ticks.
   - ⏎ chooses and moves to the next unanswered question; the last answer finishes.
   - Typing goes to `Something else`, and esc leaves as an ask does. The 400 ms guard applies,
     and it restarts on each new tab.
   - Pairs: `←→ questions` (several only), `space tick` (multi only), `⏎ choose`, `esc stop`.
   - The answers go back as today's `Answer` value. A finished question's step reads its answer
     (or `N answers`) on the right, and its well (M195) lists each question and answer.
7. **What goes:** `dialog.rs`'s box and its caution row, the digit keys, and `ask/aside.rs`
   where only the old box used it. `form.rs` stays for forms.

## Files

- `crates/bingo-surface-tui/src/{ask/,dialog.rs,component/,keys.rs,dock.rs}`.
- `crates/bingo-mod-ask/src/{drawing.rs,state.rs,handlers.rs}`.
- `crates/bingo-sdk/src/event/interaction.rs` (`Caution`, `Stakes`), its fixtures and the
  regenerated schemas.
- `crates/bingo-core/src/gate/prompt.rs`, `crates/bingo-permissions/src/caution.rs` and
  `crates/bingo-tool-bash/src/lib.rs` (`preview`) and `src/stakes{.rs,/tests.rs}`.
- Data side also adapts the SDK policy trait and existing preview consumers, tests old
  journals in `bingo-store-jsonl`, and records wire compatibility in `docs/guide/rpc.md`.
  The permission preview is boxed to keep the enlarged enum below Clippy’s size limit;
  its wire shape is unchanged. No dependencies are added. The synchronous preview uses the existing process-tree helper
  on a scoped thread with a Tokio runtime and one 500 ms deadline.

## Exit criteria

- [x] Full frame through the harness: `ask-edit`, `ask-edit-down`, `ask-edit-allowed`,
      `ask-bash`, `ask-bash-change`, `ask-bash-editing`, `caution`, `caution-no`, `question`,
      `question-preview`, `question-ticked`, `question-second`, `question-answered`,
      `small-ask-edit` and `ascii-ask-edit`.
- [x] Contract tests: a permission with a string caution and with a `Caution` both read, and
      `Stakes` round-trips. An old journal fixture loads.
- [x] Unit tests:
  - the guard swallows ⏎ at 399 ms and takes it at 400;
  - typing on the list starts `No, and tell it why`;
  - a caution starts on No and offers no session rung;
  - the scope words for each rule form;
  - the force-push stakes from a temporary repository, and none when git fails.
- [x] Black-box: `--print` and RPC still answer permissions as before. The added fields are
      additive in the schemas.
- [x] Gates and checks as in M192. tmux drive at 100×32 and 80×24: an edit ask, a bash ask
      changed first, a no with a reason, a force push in a temporary repository, and a two-tab
      question.

## Non-goals

Forms (`form.rs`); the sign-in sheet (M198); a teammate's ask waiting in its own session
(M200); how the model answers a no; previews for URL calls beyond the address.

## Risks

- **`caution` changes shape on the wire.** An RPC client that reads a string breaks. The
  schemas say so, and the RPC guide names the change.
- **A git call in a preview.** It is bounded, local refs only, and never touches the network.
  A stale remote-tracking ref lists what was true at the last fetch, so the line says
  `only <remote>/<branch> has`, as the capture does.

## Verified (data side)

Scope: brick 4's data only. Bricks 1–3 and 5–7, drawing, captures and tmux drives are
left for the later UI job; the mixed UI exit criteria above remain unchecked.

- [x] Contracts first: the new fixture test was written and run before implementation.
  `cargo test -p bingo-sdk --test caution --locked` initially failed with E0432/E0026
  (missing `Caution`, `Stakes`, and `stakes`); after implementation:
  `test result: ok. 3 passed; 0 failed` (both caution forms, stakes/old commands,
  and optional schema fields).
- [x] Old journal: `cargo test -p bingo-store-jsonl an_old_string_caution_journal_loads --locked`:
  `test journal::compatibility::an_old_string_caution_journal_loads ... ok`;
  `test result: ok. 1 passed; 0 failed`.
- [x] No session rung and act propagation:
  `cargo test -p bingo-core gate::tests --locked`: `8 passed; 0 failed`, including
  `a_policy_caution_keeps_its_act_and_drops_the_session_scope` and
  `destructive_input_has_caution_and_no_session_answer`.
- [x] Named patterns: `cargo test -p bingo-permissions caution --locked`:
  `3 passed; 0 failed`, including `every_pattern_names_its_act_and_reason`.
- [x] Local stakes: `cargo test -p bingo-tool-bash stakes:: --locked`:
  `7 passed; 0 failed`. Real commits and `refs/remotes/origin/dev` prove all seven
  are counted and only five shown; singular/zero, non-force commands, missing refs,
  failed spawn/exit, timeout and calls inside Tokio are covered. A partial clone with a
  missing commit remains unfetched, even though its local bare remote can supply it.
- [x] Regeneration (each command used `BINGO_UPDATE_SCHEMA=1`, run sequentially):
  `cargo test -p bingo-sdk --lib --locked`;
  `cargo test -p bingo-surface-rpc --lib --locked`;
  `cargo test -p bingo-plugin-rpc --lib --locked`;
  `cargo test -p bingo-mods dollar::tests::schema_and_types --locked`.
  Their schema/type generation tests passed. A JSON comparison against `git show dev:<file>`
  restored only the old caution type and removed the added stakes field/definitions in memory:
  `schema/{dollar,plugin,rpc}.json: unchanged after removing optional stakes and restoring the old caution shape`.

Implementation notes and plan gaps:
- The plan names an RPC guide that did not exist in this checkout; added `docs/guide/rpc.md`.
- Full Clippy initially rejected the enlarged permission variant (`large_enum_variant`,
  312 bytes versus 80). Boxing its optional preview fixes the Rust layout without changing
  JSON; existing consumers only dereference the box and read `caution.reason`.
- `Tool::preview` is synchronous. A scoped thread supplies the existing `bingo-process::tree`
  helper with its own Tokio runtime and one 500 ms deadline. It streams stdout without
  clipping the count; `--no-lazy-fetch` also fails closed on an older Git. No dependency was added.
- The generator also refreshed unrelated old amendments in frozen `schema/plugin-2.json`;
  restored that historical file from Git, retaining only current schema/type regeneration.
- The prototype, plan and caution capture agree; no capture or prototype was changed.

- [x] Windows checks attempted for every touched crate, sequentially, using
  `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc`:

  | Crate | Output / status |
  | --- | --- |
  | `bingo-sdk` | `Finished dev profile` in 7.52s; exit 0 |
  | `bingo-core` | `Finished dev profile` in 17.11s; exit 0 |
  | `bingo-tool-bash` | `Finished dev profile` in 6.12s; exit 0 |
  | `bingo-store-jsonl` | `Finished dev profile` in 0.69s; exit 0 |
  | `bingo-permissions` | tree-sitter: `fatal error: 'stdio.h' file not found`; tree-sitter-bash: `'stdlib.h' file not found`; exit 101 |
  | `bingo-channel-adapter` | aws-lc-sys: `fatal error: 'stdlib.h' file not found`, `'windows.h' file not found`; exit 101 |
  | `bingo-surface-tui` | aws-lc-sys: same missing Windows CRT/SDK headers; exit 101 |
  | `bingo` | aws-lc-sys: same missing Windows CRT/SDK headers; exit 101 |

- [ ] Full Windows verification: the last four crates remain blocked on this Mac's
  missing Windows C toolchain. The Rust target is installed. No stubs or host headers
  were substituted; these crates are not claimed Windows-verified.

Environment failure: the first full workspace test stopped at
`bingo-provider-openai::tests::a_claimed_api_key_reaches_the_provider`:
`left: Ready { from: Some(Env) }; right: Ready { from: Some(Setting) }`.
The required isolated rerun with
`cargo test -p bingo-provider-openai --lib tests::a_claimed_api_key_reaches_the_provider --locked -- --exact`
failed identically. Removing `OPENAI_API_KEY` only from the test process environment made
that exact test pass (`1 passed; 0 failed`); provider code was not changed. Full-suite
verification below uses that same environment isolation.

- [x] Permission black boxes, from the final full workspace run:

  ```text
  test asking::a_permission_a_rule_could_silence_offers_the_session_rung ... ok
  test asking::a_permission_no_rule_could_silence_offers_no_session_rung ... ok
  test stream_json::a_permission_is_asked_of_the_host_and_an_allow_runs_the_tool ... ok
  test stream_json::a_denied_permission_stops_the_tool_and_the_turn_goes_on ... ok
  test rpc::a_permission_is_answered_over_the_wire_and_the_tool_runs ... ok
  ```

- [x] Final native gates, all run sequentially after the no-fetch change:

  | Command | Output / status |
  | --- | --- |
  | `cargo fmt --all -- --check` | No output; exit 0 |
  | `cargo check --workspace --all-targets --locked` | `Finished dev profile` in 2.42s; exit 0 |
  | `cargo clippy --workspace --all-targets --locked -- -D warnings` | `Finished dev profile` in 4.37s; exit 0 |
  | `env -u OPENAI_API_KEY cargo test --workspace --locked` | Exit 0; aggregate of 120 reported test results: **6369 passed; 0 failed; 0 ignored** |
  | `scripts/check_discipline.sh` | `discipline ok`; exit 0 |
  | `scripts/budget.sh` | Output below; exit 0 |
  | `cargo deny check` | `advisories ok, bans ok, licenses ok, sources ok`; exit 0 |

  ```text
  dependencies (unique, normal): 423 (max  423)
  warm cargo check -p bingo-core: 1s (max  20s)
  relink isolation: touching the TUI recompiled 0 crates for core (must be 0)
  target/debug: 9 GB (soft max  5)
  warn: target/debug exceeds the soft limit
  test binaries: 145
  budget ok
  ```

The budget's disk limit is soft. Discipline retains non-fatal function/record/snapshot
warnings, including this appended verification record's length. The full test run also
passes all 19 existing PTY smoke tests; no Ember capture was generated or edited.

## Verified

Base `11d46a5a4`; 14 commits. Proof: `rebase -x 'cargo fmt --all -- --check && cargo check
--workspace --all-targets --locked' 11d46a5a4` printed `ok` after each of the first 13.

- [x] Captures: `m197_permission_frames_are_answered_by_keys`, `..._allowed_edit_...`,
  `m197_question_frames_are_answered_by_keys`, `..._answered_question_...`: all 15 pass full frame.
- [x] Units: `the_guard_swallows_a_key_at_399_ms_and_takes_it_at_400`,
  `a_key_before_the_guard_sends_nothing`, `change_prefills_then_submits_and_typing_starts_the_no`,
  `a_caution_starts_on_no_and_offers_no_session_rung`, four `ask::scope` tests;
  `bingo-tool-bash stakes::` 7 passed; `bingo-mod-ask` 8 passed.
- [x] Gates, one at a time: fmt 0; check 0; clippy `-D warnings` 0;
  `env -u OPENAI_API_KEY cargo test --workspace --locked`: 6626 passed, 0 failed, 133 binaries;
  `check_discipline.sh`: `discipline ok`; `budget.sh`: `budget ok`; `cargo deny check`:
  `advisories ok, bans ok, licenses ok, sources ok`.
- [x] Windows: `bingo-mod-ask` Finished, exit 0. `bingo-mods` and `bingo-surface-tui` exit 101:
  aws-lc-sys `'stdlib.h'`/`'windows.h' file not found` (no Windows SDK on this Mac).
- [x] tmux: `python3 docs/verification/m197/drive.py` printed PASS and CLEAN at 100x32 and
  80x24 (`tmux -L m197-drive`, temporary HOME/data/cwd, fake provider, server killed).
- [x] PTY smoke rewritten for the dock: `TUI_SMOKE_SOCKET=m197-drive scripts/tui-smoke.sh`
  printed `tui-smoke ok`. The sealed fixtures now prove they draw nothing over the dock.

Departures from plan or prototype:
- The kernel gates a changed call again, so `Change it first` asks once more (the prototype
  runs it). The drive answers that second ask.
- A path rule reads as the prototype's `pathTail(dir,3)`: two directories and a separator.
- Esc on a question stops the turn (ADR-0095 said skip). The guard holds esc, not ctrl+c.
- An ask opening while the person types no longer waits aside; the guard replaces that.
- A tall ask keeps its last rows (`body.slice(-cap)`), so its title can scroll off at 80x24.
- Left: form.rs fallback still shows digits; the keys help still names `1-9 · y/a/n` and
  `ctrl+e` (palette snapshots are another job's); sign-in keeps its sealed card.

