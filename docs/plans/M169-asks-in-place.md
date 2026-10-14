# M169 — Asks in place

ADR-0082 §6 (sealed), ADR-0084, ADR-0086. Needs M167.
Approved prototype (2026-10-14, "询问 · 问题 · 表单" tab):
https://claude.ai/artifact/XWoqqyP8Jch6p5889dL1ZB

## Goal

An ask is answered at the step it belongs to, never in a box over the
chat. The step turns amber with `needs you`; the diff or command hangs
beneath it, then an ordinary list (`↑↓`, `⏎`, or the number):
Yes / Yes for this session / No, and tell it why / Change it first.
Questions and forms use the same list. Typing that is not an answer
sets the ask aside instead of answering it. Every answer leaves a
receipt on its step and in the turn's folded line.

## Bricks

1. **Change it first.** `Answer::AllowChanged { input }` (additive,
   `AnswerSpec::AllowChanged` offered for edit and bash permissions).
   The gate re-checks the changed input as a new call: a change that
   would itself be refused or asked about is asked again. Wire fixture;
   gate tests for both.
2. **Caution.** `InteractionKind::Permission.caution: Option<String>`
   (additive). The gate sets it from the call's `destructive` trait
   and a pure `caution(command)` table for bash (`rm -r`, `git reset
   --hard`, `git push --force`, `dd`, …); a cautioned ask carries no
   `session_scope`. Table test.
3. **Options, pure.** `ask::options(&InteractionKind) -> Vec<Opt>`:
   the permission list from `answers`, `session_scope` and `caution`;
   the initial cursor is Yes, or No when cautioned. Table test.
4. **Sealed drawing in place.** The surface's own permission drawing
   (`component/sealed.rs`) moves under its step in the live turn: the
   preview framed, the question line, the list; `No` and `Change`
   become an input on their own row (`No, because …`; the change
   prefilled with the command or the `+` lines). `ctrl+o` shows the
   whole diff. Sealed rules unchanged: a mod draws around it, the
   `KeyBar` stays the host's.
5. **Questions and forms are `bingo-mod-ask`.** The built-in mod draws
   `Question` and `Form` under their step with the same list: numbers
   answer a single choice, the last row is words of one's own, `space`
   ticks a multi choice, a form walks fields with `tab`, `←→` picks,
   `space` toggles, `⏎` sends, `esc` declines. It replaces
   `form.rs`, `dialog.rs` and `component/question.rs` drawings.
6. **Set aside.** A printable key that answers nothing, or any typing
   within 900 ms before the ask appeared, puts the draft in the
   composer and the ask on the activity line (`… is waiting for you ·
   esc back to it`); `⏎` queues the draft after the turn (existing
   queue). Pure `aside(now, last_typed, key, options)`; table test.
7. **Many askers.** Several open asks (crew members) show one at a
   time in arrival order; the activity line says `1/3 ◆ moss · then …`.
8. **Receipts.** `PermissionReceipt` and `QuestionAnswer` draw on the
   step's right (`you: yes`, `you: no, …`, `you changed it first`);
   the folded turn line lists them. `esc` on a permission is
   `Deny` and stops the turn; the fold says how many steps had run.

## Files

- `crates/bingo-sdk/src/event/interaction.rs`; `schema/`,
  `sdk/typescript/`; `crates/bingo-core/src/gate.rs`; answer mapping
  in `bingo-plugin-rpc/src/doors.rs`, `bingo-surface-print`, and the
  ACP surface in `crates/bingo` (`tests/acp_asked.rs`).
- `crates/bingo-permissions/src/{decide.rs,caution.rs}`.
- `crates/bingo-surface-tui/src/{component/sealed.rs,activity.rs,
  input.rs,fold.rs}`, new `ask/{options.rs,aside.rs}`; deletions in
  `form.rs`, `dialog.rs`, `component/question.rs`.
- New `crates/bingo-mod-ask/`; `bingo-channel-adapter/src/question.rs`
  does not offer `AllowChanged` (the chat keeps its buttons).

## Exit criteria

- [ ] Fixture and gate tests for `AllowChanged` and `caution`;
      options and aside tables.
- [ ] Snapshots per prototype scenario (edit, bash, danger, question,
      multi, form, aside, crew) at 108×30 and 80×24.
- [ ] `TestBackend`: arrow and number answers, No with a reason, change
      then run, danger starts on No, esc stops the turn, typed `x`
      sets aside and `esc` returns.
- [ ] An outside mod cannot replace the permission list (sealed test);
      it can wrap `Question` with `next`.
- [ ] tmux drive against the fake provider in a temp HOME, captured;
      tui-smoke; gates as M167 plus Windows check of permissions.

## Non-goals

Mouse answers; the desktop's ask drawing; rule editing from the ask
(`/permission` does that); changing a write's whole file in place.

## Risks

- A changed command that escapes its scope (`cargo test; rm -rf ~`):
  the re-check treats it as a new call; the test names this case.
- An ask arriving while the person types fast: the 900 ms window is a
  constant in one place, tested with a fake clock, not wall time.
