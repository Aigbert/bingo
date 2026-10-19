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

- TUI: input/key policy, queue_rows, dock/composer, shell/chip/headline drawings, parts/tone toasts.
- ADR-0098; sdk/headline.rs, Input/item/props/slot contracts and generated schemas.
- Core/user.rs shares user-item construction for logging, opening and steering.
- TUI run/time.rs and clipboard.rs inject clock/clipboard; run/acting exercises real Run boundaries.
- tool-bash completion headlines/toasts; m194b/fixtures.cjs exports turns only.
- PTY tests, smoke and docs/verification/m196/drive.py. Palette/mentions implementation stays part 2.

## Exit criteria

- [x] Full-frame keyed scenes: live-steered, live-stopped, stop, stop-resent, steer,
  steer-landed, shell, shell-ls, image, image-sent; exact sends and real-kernel helper proofs.
- [x] on_key: draft/history, last queued withdrawal, ordered stop restoration, every esc/ctrl+c
  rung and timed double press, both legacy alt+Enter cases, image refusal, idle-only `?`.
- [ ] Part 2: bricks 7–8, palette ranking; palette, palette-st, palette-ran, skills,
  skills-review, skills-sent, mention, mention-inserted, small-palette, ascii-palette.
- [x] Round-2 black-box PTY stopping_returns_a_queued_line_to_the_input.
- [x] Seven native gates, smoke, and drive.py at 100×32 and 80×24. Windows results below.

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

- [x] Bricks 1–6, 9 and all ten keyed full frames: `cargo test --workspace --locked` (provider credentials unset):
  `6552 passed; 0 failed; 0 ignored`; TUI `1435 passed`. Exact sends and six real-Host answer proofs pass.
- [x] Same run: all on_key rungs, clock windows, legacy alt+Enter, stop races/order/NotFound, images and idle `?` pass.
  `acting_full_frames_match_the_prototype ... ok`; only named spinner cells are exempt.
- [x] Black-box PTY: `stopping_returns_a_queued_line_to_the_input ... ok`; `20 passed; 0 failed`.
- [x] Headline contracts/provider isolation, parts/tone/hold and shell rows: same run; core `692 passed`, bash `157 passed`.
  `BINGO_UPDATE_SCHEMA=1 cargo test -p <crate> --locked` for sdk/mods/plugin-rpc/surface-rpc: all pass, schemas committed.
- [x] `cargo fmt --all -- --check`: exit 0; `cargo check --workspace --all-targets --locked`: `Finished dev profile ... in 6.78s`.
- [x] `cargo clippy --workspace --all-targets --locked -- -D warnings`: `Finished dev profile ... in 20.51s`.
- [x] `scripts/check_discipline.sh`: `discipline ok`; `scripts/budget.sh`: `423 (max 423)`, `budget ok` (14 GB soft warning).
- [x] `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`. No wall-clock retry in the final gate run.
- [x] `cargo build -p bingo --locked`: `Finished dev profile ... in 55.71s`.
  `TUI_SMOKE_SOCKET=m196-drive scripts/tui-smoke.sh`: `tui-smoke ok`.
- [x] `python3 docs/verification/m196/drive.py`: `PASS 100x32` / `PASS 80x24`; both `CLEAN` lines confirm cleanup.
- [x] Windows `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc --locked --keep-going`:
  sdk/core/tool-bash pass all the way through; additional passes: surface-rpc/mod-models/mod-palette.
- [ ] Windows TUI/binary: aws-lc-sys lacks stdlib.h/windows.h; binary also fails tree-sitter/tree-sitter-bash
  (stdio.h) and rquickjs-sys (/std:c11). Channels/print share aws-lc-sys; mods also fails rquickjs-sys.
- [ ] Part 2: bricks 7–8/ranking and the ten named captures in Exit criteria await M198's list window.
- Prototype/kernel win: shell lines are standalone items, not turns; no shell-specific placeholder exists.
  Image transport retains its token; fixture prompt comparison removes it. Setup facts are named in round-2.md.
- Only authorized Esc rows changed in seven palette snapshots. Three error-toast snapshots gain ×;
  shell_ran/quiet_notices retain round-one bytes. Prototype/captures untouched. No M202 protected function changed.
- No new dependency or package; one kernel fact, ADR-0098: a delivery may carry its headline.
- Evidence, per-scene timeline, departures and constructor exceptions: [round 2](../verification/m196/round-2.md),
  [checks](../verification/m196/checks.txt), [shared-site handoff](../verification/m196/handoff.md). Final Cargo checks ran serially.
