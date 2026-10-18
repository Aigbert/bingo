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
- New `src/queue_rows.rs`.
- `crates/bingo-mod-palette/src/{draw.rs,rows.rs}`.

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
