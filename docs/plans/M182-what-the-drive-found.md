# M182 — What the drive found

ADR-0095, ADR-0084, M180. Bugs found driving the prototype's 32 scenarios
against the real TUI (2026-10-16), and the `/` lag the person reported. Each is
a bug fix; they share a plan because they share the drive that checks them.

## Goal

Typing after `/` costs what typing after `@` costs and never flickers; the ten
drive bugs are gone, each with a test that failed before the fix.

## Bricks

1. **`/` types locally.** A key that only edits the query (characters,
   backspace, cursor moves) edits the composer at once; the palette's pull for
   the new query is sent, not awaited. Only keys that act on the list (Enter,
   Tab, ↑↓, PgUp/PgDn) wait in `run/typeahead.rs` for the current query's list.
   `palette::waiting` stops rebuilding the full props on every check: it compares
   the query the last answer was drawn for.
2. **No fallback flash.** While a pull is pending the palette keeps the last
   answer's drawing (old rows retained until the new answer), never the
   names-only `palette/fallback.rs`; fallback is only for "no answer has ever
   arrived". The palette's height grows only when the row count exceeds the
   last height (and shrinks to fit a smaller terminal), so the header row does not jump on every key.
3. **Pager Esc** closes the pager and does nothing else; `q` and `ctrl+o` close
   it too (B1).
4. **Esc on a question or form** answers "skipped, it decides" and the turn
   goes on (ADR-0095 §4; `input/interaction.rs::cancel`) (B7).
5. **One receipt per answer**: the `⏺ Form` echo after `The user answered` goes
   (B8); `• 1 steps` reads `• 1 step` (`ask/receipts.rs:118`).
6. **Toasts never replay late**: toast clocks run on the wall clock while a pane
   holds; an expired toast is dropped (`toast.rs`).
7. **Catalogue cwd**: `bingo-core/src/host/catalog.rs` lists skills from the
   session's cwd, not `std::env::current_dir()`.
8. **Model card** shows "compacts first" only when the window is known and the
   chat exceeds it (`bingo-mod-models/card.rs`).
9. **Picker cursor**: `*` unpin keeps the cursor on the row and toasts
   `unpinned <model>`; the hint reads `* unpin` on a pinned row; a picker opened
   with a filter shows its cursor row (B12, B19).
10. **`/login`** opens one pane deep; one Esc closes it (B13).

## Files

- `crates/bingo-surface-tui/src/{palette.rs,palette/fallback.rs,run/typeahead.rs,component.rs,toast.rs,input/interaction.rs,ask/receipts.rs}` and the pager key routing.
- `crates/bingo-mod-palette`, `crates/bingo-mod-models/{card.rs,keys.rs,rows.rs,commands.rs}`, `crates/bingo-mod-ask`.
- `crates/bingo-core/src/host/catalog.rs`, interaction receipts, `bingo-tool-fs/src/ask.rs`,
  the TUI crate guide, and isolated question smoke coverage.
- `bingo-sdk/src/tool.rs`, core executor call binding and `host/tool_host.rs`.

## Exit criteria

- [ ] A `TestBackend`/typeahead test: ten query keys after `/` reach the composer without a reply from the mod; Enter waits for the list of the final query.
- [ ] A tmux drive (fake provider, temp HOME, release build) under `stress`-style CPU load: per-key echo after `/` within 2× the `@` echo; painted bytes per key recorded before/after.
- [ ] Captured frames between keys never show the fallback drawing once a list has arrived.
- [ ] Each of bricks 3–10 has a test that fails on dev and passes here.
- [ ] Gates: fmt, check, clippy, test, discipline, budget, deny, tui-smoke; Windows all-target check of touched crates.

## Non-goals

The palette's new groups, wording and skills group (M187); the `KeyBar` (M184).

## Risks

- Local filtering must use the same matcher the mod uses, or rows shift when the
  real answer lands: reuse the mod's matcher from the sdk-level pure function,
  or keep the old rows unfiltered until the answer arrives.
- M180's guarantee (keys never lost, never into an ask) must hold: its tests
  stay green unchanged.

## Implementation decisions

- Keep the last palette rows unfiltered while awaiting the mod. Its matcher and
  selection stay with its owner; local filtering would duplicate both. List
  actions still wait for the current query and cannot activate those old rows.
- Keep the largest accepted palette height for each opening, bounded by the
  viewport. Padding shorter answers keeps the header steady while narrowing.
- Root Backspace and Escape retain the palette owner's close/back semantics;
  ordinary edits and horizontal cursor movement stay with the composer.
- Question/form Escape sends the existing `Cancel` answer without an interrupt.
  The receipt says `skipped · it decides`; the question tool returns a successful
  skip result so the model continues. Permission and sign-in refusal stay intact.
  Escape from a draft set aside returns to its ask, as ADR-0095 §4 requires.
- An interaction that does not offer `Cancel` cannot accept a skip. Preserve its
  interrupt-only escape route; do not invent an unsupported answer or trap focus.
- The extra Form block comes from a missing source item on tool questions.
  Bind the executor's call prompter to its existing item and carry it through
  `ToolHost::ask_for` (defaulting to `ask` for existing hosts). This repairs the
  receipt link for every tool without guessing from text or tool names. The
  TUI's existing linked-receipt rendering then owns the one receipt.
