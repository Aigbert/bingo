# M169 — Asks in place

ADR-0082 §6 (sealed), ADR-0084, ADR-0086. Needs M167.
Approved prototype (2026-10-14, "询问 · 问题 · 表单" tab):
https://claude.ai/artifact/XWoqqyP8Jch6p5889dL1ZB

## Goal

An ask is answered at the step it belongs to, never in a box over the chat. The step turns amber with
`needs you`; the diff or command hangs beneath it, then an ordinary list (`↑↓`, `⏎`, or the number): Yes /
Yes for this session / No, and tell it why / Change it first. Questions and forms use the same list.
Typing that is not an answer sets the ask aside instead of answering it. Every answer leaves a receipt on
its step and in the turn's folded line.

## Bricks

1. **Change it first.** `Answer::AllowChanged { input }` (additive, `AnswerSpec::AllowChanged` offered for
   edit and bash permissions). The gate re-checks the changed input as a new call: a change that would
   itself be refused or asked about is asked again. Wire fixture; gate tests for both.
2. **Caution.** `InteractionKind::Permission.caution: Option<String>` (additive). The gate sets it from
   the call's `destructive` trait and a pure `caution(command)` table for bash (`rm -r`, `git reset
   --hard`, `git push --force`, `dd`, …); a cautioned ask carries no `session_scope`. Table test.
3. **Options, pure.** `ask::options(&Interaction) -> Vec<Opt>`: the permission list from `answers`,
   `session_scope` and `caution`; the initial cursor is Yes, or No when cautioned. Table test.
4. **Sealed drawing in place.** The surface's own permission drawing (`component/sealed.rs`) moves under
   its step in the live turn: the preview framed, the question line, the list; `No` and `Change` become an
   input on their own row (`No, because …`; the change prefilled with the command or the `+` lines).
   `ctrl+o` shows the whole diff. Sealed rules unchanged: a mod draws around it, the `KeyBar` stays the
   host's.
5. **Questions and forms are `bingo-mod-ask`.** The built-in mod draws `Question` and `Form` under their
   step with the same list: numbers answer a single choice, the last row is words of one's own, `space`
   ticks a multi choice, a form walks fields with `tab`, `←→` picks, `space` toggles, `⏎` sends, `esc`
   declines. It replaces `form.rs`, `dialog.rs` and `component/question.rs` drawings.
6. **Set aside.** A printable key that answers nothing, or any typing within 900 ms before the ask
   appeared, puts the draft in the composer and the ask on the activity line (`… is waiting for you · ctrl+g
   back to it`); `⏎` queues the draft after the turn (existing queue). Pure `aside(now, last_typed, key,
   options)`; table test.
7. **Many askers.** Several open asks (crew members) show one at a time in arrival order; the activity
   line says `1/3 ◆ moss · then …`.
8. **Receipts.** `PermissionReceipt` and `QuestionAnswer` draw on the step's right (`you: yes`, `you: no,
   …`, `you changed it first`); the folded turn line lists them. `esc` on a permission is `Deny` and stops
   the turn; the fold says how many steps had run.

## Files

- `crates/bingo-sdk/src/event/interaction.rs`; `schema/`, `sdk/typescript/`;
  `crates/bingo-core/src/gate{.rs,/prompt.rs}`; answer mapping through existing RPC/ACP serde paths;
  fixtures in `bingo-surface-print` and `crates/bingo/tests/acp_asked.rs`.
- `crates/bingo-permissions/src/{lib.rs,decide.rs,caution.rs}`; SDK policy, state and component props;
  core session interactions and receipt recording.
- `crates/bingo-surface-tui/src/{component/sealed.rs,activity.rs,
  input.rs,view.rs,transcript.rs,blocks.rs,run/site_keys.rs}`, new `ask/`; fallback drawings in
  `dialog.rs`, `component/question.rs`.
- New `crates/bingo-mod-ask/`; `bingo-channel-adapter/src/question.rs` does not offer `AllowChanged` (the
  chat keeps its buttons).

## Exit criteria

- [x] Fixture and gate tests for `AllowChanged` and `caution`; options and aside tables.
- [x] Snapshots per prototype scenario (edit, bash, danger, question, multi, form, aside, crew) at 108×30
  and 80×24.
- [x] `TestBackend`: arrow and number answers, No with a reason, change then run, danger starts on No, esc
  stops the turn, typed `x` sets aside and `ctrl+g` returns.
- [x] An outside mod cannot replace the permission list (sealed test); it can wrap `Question` with `next`.
- [x] tmux drive against the fake provider in a temp HOME, captured; tui-smoke and all native gates as
  M167.
- [ ] Complete Windows compilation of every touched crate: blocked by missing Windows C headers in
  existing dependencies; see the command results below.

## Non-goals

Mouse answers; the desktop's ask drawing; rule editing from the ask (`/permission` does that); changing a
write's whole file in place.

## Risks

- A changed command that escapes its scope (`cargo test; rm -rf ~`): the re-check treats it as a new call;
  the test names this case.
- An ask arriving while the person types fast: the 900 ms window is a constant in one place, tested with a
  fake clock, not wall time.

## Decisions

- Review: Escape always ends the asking session’s turn, including aside and change editing; Ctrl+G
  backs out without answering. The old Escape-to-return prototype conflicted with AGENTS.md.
  A single owner-targeted operation sends both the denial and interrupt (ADR-0084).

- Options read `Interaction`: accepted answer specs are stored there, not on `InteractionKind`. This
  preserves the kernel's authoritative answer list.
- A denied replacement is asked again with only Change and No. Approval never overrides a deny rule. Every
  replacement repeats hooks and policy checks.
- Review: each permission interaction carries the gate’s current input after hooks/interceptors.
  The sealed card and editor read that input. Only `ItemUpdated` changes the transcript call;
  resolving an edit merely proposes a replacement, which has not passed the gate yet.
- `PermissionPolicy::caution` is a defaulted, synchronous policy hook: shell analysis stays in
  permissions, while the gate also reads destructive traits. A default-false `allows_replacement` hook
  keeps tool names out of the kernel.
- Keep forms on the existing `Question` site (already a vector of questions), adding optional interaction
  identity and form mode. A second site would duplicate this contract. The built-in owns its cursor, ticks
  and text.
- Register the guide in `bingo-mod-ask/src/guide.rs` with a crate-local page.
- Workspace edges: the binary adds `bingo-mod-ask`; its sdk, async-trait and serde_json dependencies
  already exist. No external dependency is added. Tests in mods and the TUI add a dev edge to ask, as M167
  did for hello. The normal-tree member count grows 412 → 413 (ADR-0086).

- Receipts gain an optional source item: the old permission receipt named only a placeholder interaction,
  which cannot locate the step after it closes. Question/form receipts are recorded by the interaction
  actor on resolution.
- Keep the surface's old question/form machinery only as the disabled-mod fallback required by ADR-0086;
  the active builtin draws and drives the new list and field layout. No new host door is needed: handlers
  return a chosen answer in the supervised action acknowledgement; the surface resolves it.
- Review: plugin handles carry unforgeable provenance; Permission answers from plugins are refused
  even with a terminal identity. ADR-0086 now states this boundary. Mod key requests contain the
  minted question props, so no per-key session snapshot is needed. An event hook releases state at
  resolution/cancellation. Tests enable the existing SDK testing feature; no new dependency.

- The existing Question site represents both forms and questions. A single, untitled form uses the choice
  list while still returning `Answer::Form`; AskUserQuestion uses that wire shape even when it asks one
  question.
- Preserve the existing y/a/n permission aliases alongside numbers. They remain explicit answers; other
  printable keys set the ask aside.
- Cancellation bypasses the stray-key guard. The actor records a denial before interruption can drop the
  caller. Review: the turn now asks for and folds that canonical receipt instead of minting a second
  unpublished item. `AllowChangedSession` preserves both the edit and the installed session rule.

- RPC and ACP already deserialize the shared answer enum; their generic doors need no branch. Regenerated
  wire contracts and existing black-box suites cover the additive answer.

## Verified — 2026-10-15

Sequential native gates; local/fake fixtures only. Command-output summaries:

```text
cargo fmt --all -- --check -> exit 0
cargo check --workspace --all-targets --locked -> Finished dev profile; exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings -> Finished dev profile; exit 0
cargo test --workspace --locked -> 5531 passed; 0 failed; 5 ignored (168 suites)
bash scripts/check_discipline.sh -> discipline ok (existing advisory warnings)
bash scripts/budget.sh -> dependencies 413/413; core check 0s; relink 0 crates; budget ok
cargo deny check -> advisories ok, bans ok, licenses ok, sources ok
bash scripts/tui-smoke.sh -> tui-smoke ok
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk --locked -> 190 passed
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc -p bingo-surface-rpc -p bingo-store-jsonl -p bingo-mods --lib --locked -> 264 passed; 1 ignored
cargo test -p bingo-surface-tui --locked -> 1200 passed; 0 failed; 2 ignored
cd sdk/typescript && npm test -> 31 pass; 0 fail
```

The 16 scenario snapshots include three child askers ordered moss → coda → fern. The final TUI run
includes the child whole-preview regression. Smoke captures in `/tmp/m169-smoke.log` show changed
input rechecked, `x` set aside, Escape return, numeric answer and receipts; the sealed fixtures also pass.
Budget reports a 9 GB target/debug soft warning; no external dependency was added. No timing retry was needed.

For each touched crate: `cargo check -p <crate> --all-targets --locked --target x86_64-pc-windows-msvc`:

- `bingo-sdk`, `bingo-core`, `bingo-mod-ask`, `bingo-hooks-shell`: `Finished dev profile`, exit 0.
- `bingo-permissions`: exit 101; tree-sitter/tree-sitter-bash cannot find Windows `stdio.h`/`stdlib.h`.
- `bingo-surface-tui`, `bingo-channel-adapter`, `bingo-surface-print`, `bingo-mods`, `bingo`: exit 101;
  aws-lc-sys cannot find `windows.h`/CRT headers; mods also reaches the rquickjs-sys C-toolchain failure.
  These are host toolchain deferrals, not verified Windows builds; the Windows criterion remains open.
