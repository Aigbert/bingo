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
  core session interactions, receipt recording and `host/plugin.rs` provenance adapter.
- `crates/bingo-surface-tui/src/{component/sealed.rs,activity.rs,
  input{.rs,/interaction.rs},effect.rs,view.rs,transcript.rs,blocks.rs,run/{site_keys,interactions}.rs}`, new `ask/`; fallback drawings in
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
- [x] All nine round-1 and four round-2 findings have failing-before/passing-after regressions.
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

- Round 2: interaction keys bypass the pane-action FIFO, retaining per-site order, and use the asking
  session's handle. Late opens cannot send keys for resolved asks. Escape always interrupts that owner,
  answers only if supported, and drops its queued actions.
- Options read the accepted specs in `Interaction`, the kernel's authoritative answer list.
- A denied replacement offers only Change and No. Every replacement repeats hooks and policy checks;
  approval never overrides a deny rule.
- Review: permission interactions carry the current input after hooks/interceptors. The card and editor
  use it. Only `ItemUpdated` changes transcript input: an edit answer has not passed the gate yet.
- Review: plugin handles carry kernel-stamped provenance, independent of claimed identity or activation.
  Permission answers from plugins are refused; accepted answers record their source (ADR-0086).
  The host owns one stable adapter with a weak backlink: plugin weak handles survive startup without
  forming a cycle. M171 approval forwards to the host, retaining its person-input check.
- The mod returns its chosen answer in the supervised command acknowledgement; the surface resolves it
  through the person's handle. Minted question props avoid opening a session snapshot per key.
  An event hook releases mod state on resolution/cancellation, without waiting for another key.
- Review: hold keys until the view arrives and allow one request in flight per site. Resize cannot move
  a fallback cursor; an unhandled character is folded before the next queued key is interpreted.
- Recent typing uses the surface's reception time, including asks waiting their turn, not the kernel clock.
  The reception regression supplies an explicit clock, so scheduler load cannot change its result.
- Review: Escape always ends the asking session's turn, including aside and change editing; Ctrl+G backs
  out without answering. The old prototype conflicted with AGENTS.md. Both denial and interrupt use
  the owning session's handle (ADR-0084).
- `PermissionPolicy::caution` and default-false `allows_replacement` keep shell analysis and tool names
  in permissions. Caution removes session scope and starts the cursor on No.
- Forms share `Question`; a single untitled form uses choices but returns `Answer::Form`. The mod owns
  selection, ticks and text. The old surface machinery remains only as the disabled-mod fallback.
- The guide is the registered crate-local page in `bingo-mod-ask/src/guide.rs` (ADR-0054).
- No new external dependency: the binary adds ask; mods/TUI tests add its dev edge, as M167 did for hello.
  Ask tests enable the SDK's existing testing feature. Normal-tree members grow 412 → 413 (ADR-0086).
- Receipts link their source item; the actor records question/form answers and early denials. Review:
  the turn folds the actor's canonical receipt instead of minting an unpublished duplicate. A receipt
  queued after interruption cannot publish under the closed turn. `AllowChangedSession` reports both
  the edit and installed rule. Cancellation bypasses the stray-key guard.
- Keep y/a/n alongside numbers. RPC/ACP already deserialize shared answers; their generic doors stay.

## Verified — 2026-10-15

Round 2: local/fake fixtures only; all commands below used `RUST_TEST_THREADS=4`, with gates sequential.
The four requested regressions failed before their fixes; late/failed owner opens also have coverage.

```text
cargo fmt --all -- --check -> exit 0
cargo check --workspace --all-targets --locked -> Finished dev profile; exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings -> Finished dev profile; exit 0
cargo test --workspace --locked -> 5614 passed; 0 failed; 5 ignored (168 suites)
bash scripts/check_discipline.sh -> discipline ok (advisories remain)
bash scripts/budget.sh -> dependencies 413/413; core check 1s; relink 0 crates; budget ok
cargo deny check -> advisories ok, bans ok, licenses ok, sources ok
bash scripts/tui-smoke.sh -> tui-smoke ok
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk --locked -> 192 passed
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc -p bingo-surface-rpc -p bingo-store-jsonl -p bingo-mods --lib --locked -> 267 passed; 1 ignored
cd sdk/typescript && npm test -> 31 pass; 0 fail
```

The first workspace run exposed missing M171 `approve` forwarding (`UNSUPPORTED`) in the plugin adapter.
Forwarding preserves the host's person-input check; the new guard regression and original CLI test pass.
No timing flake was observed. Regeneration left schema/TS unchanged. Budget warns at 16 GB; no new dependency.
The production provenance adapter retains its explained discipline advisory; no allowlist changed.

For each touched crate: `cargo check -p <crate> --all-targets --locked --target x86_64-pc-windows-msvc`:

- `bingo-sdk`, `bingo-core`, `bingo-mod-ask`, `bingo-hooks-shell`: `Finished dev profile`, exit 0.
- `bingo-permissions`: exit 101; tree-sitter/tree-sitter-bash cannot find Windows `stdio.h`/`stdlib.h`.
- `bingo-surface-tui`, `bingo-channel-adapter`, `bingo-surface-print`, `bingo-mods`, `bingo`: exit 101;
  aws-lc-sys cannot find `windows.h`/CRT headers; mods and bin also hit the rquickjs-sys C-toolchain failure.
  These are toolchain deferrals, not verified Windows builds; the Windows criterion remains open.
