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
  `crates/bingo-tool-bash/src/lib.rs` (`preview`).

## Exit criteria

- [ ] Full frame through the harness: `ask-edit`, `ask-edit-down`, `ask-edit-allowed`,
      `ask-bash`, `ask-bash-change`, `ask-bash-editing`, `caution`, `caution-no`, `question`,
      `question-preview`, `question-ticked`, `question-second`, `question-answered`,
      `small-ask-edit` and `ascii-ask-edit`.
- [ ] Contract tests: a permission with a string caution and with a `Caution` both read, and
      `Stakes` round-trips. An old journal fixture loads.
- [ ] Unit tests:
  - the guard swallows ⏎ at 399 ms and takes it at 400;
  - typing on the list starts `No, and tell it why`;
  - a caution starts on No and offers no session rung;
  - the scope words for each rule form;
  - the force-push stakes from a temporary repository, and none when git fails.
- [ ] Black-box: `--print` and RPC still answer permissions as before. The added fields are
      additive in the schemas.
- [ ] Gates and checks as in M192. tmux drive at 100×32 and 80×24: an edit ask, a bash ask
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
