# M196 — Acting never moves

ADR-0097 §7; ADR-0095 §4, §5 stand. Spec: `docs/design/ember/`:
- `p7-input.js`: `COMPOSER`, `submitInput`, `queueInput`, `recall`, `escChain`, `ctrlC`,
  `typeChar`, `shellKey`;
- `p4c-act.js`: `halt`, `pasteImage`, `backgroundBash`;
- `p6-dock.js`: `queueRows`, `fuzzy`, `paletteOf`, `paletteRows`, `pickLine`, `listWindow`,
  `mentionOf`, `mentionRows`.

After: M193 and M194.

## Goal

The input never moves, and every key in it does one predictable thing:
- while a turn runs, ⏎ steers, tab queues, and ↑ on an empty line takes the last queued text
  back;
- one esc ends the turn and hands queued text back;
- `!cmd` runs in the shell;
- `/` and `@` open their lists in the dock, over the input.

## Bricks

1. **Sending.**
   - ⏎ with no turn running sends.
   - ⏎ with a turn running steers: `Delivery::Wake`, as today. The text waits above the input
     reading `at the next step` until it lands.
   - tab with a turn running queues: `Delivery::Hold`, reading `queued`, or `queued N` once
     there are two or more.
   - A line that is a known command runs it, and skills go through the palette.
   - Queued rows (`queueRows`) move from `activity.rs` into the dock above the input. A new row
     fades glow→muted over 260 ms.
2. **↑ and ↓** (`recall`).
   - ↑ on an empty input takes back the last queued entry (`Effect::Withdraw`, `took()`) before
     anything else.
   - Otherwise ↑ walks history, but only from an empty input or while already walking. Today's
     `walk_back` fires on the first line and overwrites a draft; that goes.
   - ↓ walks forward, and past the end gives the empty input back.
3. **esc** (`escChain`; the table in `keys.rs` stays the one place the policy is written).
   - A running turn at home: stop it (`InterruptScope::Head`). Everything queued, steers
     included, comes back into the input, joined by newlines, unless the input already holds
     a draft. A toast reads `stopped · what you queued is back in the input`.
   - Else a draft: clear it into history, with the toast `cleared · ↑ brings it back`.
   - Else scrolled up: go to the end.
   - Else, two esc within 600 ms on nothing open the rewind sheet. The first one toasts
     `esc again to rewind`. The rewind sheet is M198; until it lands, today's rewind opens.
   - Away from home, esc never stops anything (M200).
4. **ctrl+c** (`ctrlC`).
   - It stops a running turn, closes a dock, folds detail, or clears a draft, in that order.
   - On an empty line, two presses within 1.5 s exit, and the first toasts
     `ctrl+c again to quit`.
   - Stopping never arms the exit. ctrl+d on an empty line exits.
   - The receipt printed on exit is M199's.
5. **Shell lines.**
   - `!` at the start makes the line a shell line: the input reads in `code` and the
     placeholder names the shell.
   - ⏎ runs it exactly as today (ADR-0008 §5), and the turn it makes is its command and output
     (`shellLines`).
   - ctrl+b sends a running bash step to the background (exists). The footer's `N shells` is
     M193's.
6. **Pictures** (`pasteImage`, `chipSegs`).
   - ctrl+v adds an `image N` chip to the input. The chip travels with the line and is drawn on
     your band when sent.
   - A model that reads no images refuses, with a toast naming `alt+m`.
   - Today's `[image N]` tokens become chips over the same `Image` parts, and the Kitty strip
     goes.
7. **The `/` palette** (`paletteOf`, `paletteRows`, `fuzzy`).
   - Commands and skills share one list in the dock, over the input. It replaces the
     under-the-box layer.
   - Ranking: prefix, then contains, then subsequence, then the catalogue's order.
   - At most 8 rows are shown, then `N of M`. Matched letters are glow bold, and the picked row
     has a `▎` on the pick ground.
   - The right side holds the shortcut, or `skill · <source>` for a skill.
   - The keys are `↑↓ choose`, `⏎ run`, `tab complete`.
   - `bingo-mod-palette` stays the owner and draws these rows (`draw.rs`), and its own matcher
     becomes this one.
8. **`@` mentions** (`mentionOf`, `mentionRows`).
   - Files are listed with their git state on the right: `modified` amber, `new` good. The
     state is read with `git status --porcelain=v1 -z` off the UI thread, once per opening.
   - Folders read dim and matched letters glow. Teammates (M200) and `@all` in a room are added
     to the same list.
   - At most 6 rows. The keys are `↑↓ choose`, `tab insert`.
   - A failed or slow git read lists the files with no state.
9. **`?`** on an empty input with no turn running opens the keys sheet (M198; until then, the
   keys page that exists).

## Files

- `crates/bingo-surface-tui/src/{input.rs,input/,keys.rs,run/withdraw.rs,composer.rs,
  activity.rs,complete.rs,mentions.rs,palette.rs,palette/,skills.rs,keybar.rs}`.
- New `src/queue_rows.rs`; composer/image-chip and shell transcript rendering; no new dependencies.
- `crates/bingo-mod-palette/src/{draw.rs,rows.rs}` (part 2).
- `bingo-tool-bash` background notices; PTY tests, smoke and `docs/verification/m196/` drives.

## Exit criteria

- [ ] Full frame through the harness: `live-steered`, `live-stopped`, `stop`, `stop-resent`,
      `steer`, `steer-landed`, `shell`, `shell-ls`, `image`, `image-sent`, `palette`,
      `palette-st`, `palette-ran`, `skills`, `skills-review`, `skills-sent`, `mention`,
      `mention-inserted`, `small-palette` and `ascii-palette`, with keys as `index.json` lists
      them.
- [ ] `on_key` unit tests:
  - ↑ with a draft does nothing to it;
  - ↑ on an empty input takes the last queued;
  - esc returns the queue in order;
  - the esc and ctrl+c chains, each step;
  - the palette ranks prefix over contains over subsequence.
- [ ] A black-box pty test: type, tab while a turn runs, esc, and the input holds the queued
      text.
- [ ] Gates and checks as in M192. tmux drive of each key above at 100×32 and 80×24.

## Non-goals

Asks and questions (M197); sheets (M198); teammates and rooms (M200); the exit receipt
(M199); vim keys; changing what a shell line puts in the model's context.

## Risks

- **↑ changes from today.** People who recalled history from a draft lose that. The prototype
  decided it, and the keys sheet says it.
- **Withdrawing a steer that is landing.** If the kernel delivers it first, `took()` reports
  nothing and the input stays empty, with no duplicate. Test that race with the fake provider.
- **git in a huge repository.** Bound the read (a timeout and a count cap) and draw without
  state past it.

## Verified

- [x] Bricks 1–6, 9: `cargo test --workspace --locked --no-fail-fast` (provider credentials unset):
  `acting_full_frames_match_the_prototype ... ok` — all ten part-1 frames, their indexed keys/clocks; named spinner cells only.
- [x] Same run: history, ordered queue restoration, every key-policy rung, timed double presses, both alt+Enter cases, image refusal, idle `?` pass;
  `a_fake_provider_delivery_wins_withdrawal_without_duplication ... ok`; TUI `1417 passed; 1 failed` (excluded test below).
- [x] Black-box PTY: same run, `stopping_returns_a_queued_line_to_the_input ... ok`; `20 passed; 0 failed`.
- [x] `cargo fmt --all -- --check`: exit 0; `cargo check --workspace --all-targets --locked`: `Finished dev profile ... in 7.52s`.
- [x] `cargo clippy --workspace --all-targets --locked -- -D warnings`: `Finished dev profile ... in 4.52s`.
- [x] `scripts/check_discipline.sh`: `discipline ok`; `scripts/budget.sh`: `dependencies ... 423 (max 423)`, `budget ok` (7 GB soft warning).
- [x] `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.
- [x] `cargo build -p bingo --locked`: `Finished dev profile ... in 8.50s`; `TUI_SMOKE_SOCKET=m196-drive scripts/tui-smoke.sh`: `tui-smoke ok`.
- [x] `docs/verification/m196/drive.sh`: `PASS 100x32` / `PASS 80x24`; both `CLEAN` lines confirm server and temporary HOME/data/cwd cleanup.
- [x] `cargo check -p bingo-tool-bash --all-targets --target x86_64-pc-windows-msvc`: `Finished dev profile ... in 7.19s`.
- [ ] Workspace gate: `6525 passed; 2 failed; 0 ignored`; the remaining deterministic failure is `palette::tests::ctrl_p_is_unbound_and_absent_from_the_palette_keys`.
  Its old BINDINGS Escape text differs from the new policy. Isolated `cargo test -p bingo-surface-tui --lib --locked <test> -- --exact`: `0 passed; 1 failed`.
  Both the test and BINDINGS are excluded by ownership instructions; left untouched.
  Unrelated ACP journaling failed in this suite; isolated `cargo test -p bingo --test cli --locked acp::bridge::a_bridged_call_posts_to_the_parent_and_is_journaled_under_the_turn -- --exact`: `1 passed; 0 failed`.
- [ ] Windows TUI/binary: the same target check for `bingo-surface-tui` / `bingo` exits 101 in dependencies: missing `stdlib.h` / `windows.h`; binary also lacks QuickJS MSVC `/std:c11` support.
- [ ] Part 2: bricks 7–8 and palette ranking; captures `palette`, `palette-st`, `palette-ran`, `skills`, `skills-review`,
  `skills-sent`, `mention`, `mention-inserted`, `small-palette`, `ascii-palette` await M198's shared list window.
- Prototype wins: brick 5's shell-specific placeholder does not exist in `placeholder()`; retained its standard placeholder.
  Shell folding is gone; `OUTPUT_ROWS`/`EXPAND` still serve action/tool output and generic quiet notices. Legacy background notices keep their generic view.
- No new dependency/package/kernel fact; prototype/scenes untouched. Shared ownership and exact changed functions: [handoff](../verification/m196/handoff.md).
  M198 owes BINDINGS words for `esc`, `ctrl+c`, `up/down`; M202's protected functions are untouched. No branch was merged.
- Command evidence: [checks](../verification/m196/checks.txt); text/ANSI drives in that directory. Verification remains incomplete at the two blockers above.
