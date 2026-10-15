# M166 — The model picker

ADR-0085 §8, ADR-0084 (`ModelPicker`), ADR-0086; uses M167’s seam.
Approved prototype: https://claude.ai/artifact/XWoqqyP8Jch6p5889dL1ZB

## Goal

The built-in `bingo-mod-models` draws the approved model setup: alt+m/alt+t
without a panel; `/model <filter>` with a list, fact card and provider sheets.
Facts come from M164/M165 catalogues. The surface gains no picker code.

## Bricks

1. **Site.** `Component::ModelPicker`, props `{ filter, selected, scope, effort, sheet }`
   (ADR-0084 §1); terminal/desktop, schema, TypeScript and snapshot.
2. **Catalogue with meta.** Host `catalog(Models)` and `catalog(Providers)`
   supply `meta`; refresh on `CatalogChanged`.
3. **List, pure.** `picker::rows(models, providers, filter, show_all)
   -> Vec<Row>`: pinned group first, then one group per provider with
   its state (`●` ok, `○` signed out, `◌` unchecked/checking, `✗`
   down), chat models with `★ ● ∴ ▣`, a `· N hidden` row (non-chat and
   aliases), `+ add a provider` last. Filter accepts `provider/`
   prefixes. Table tests.
4. **Fact card, pure.** `picker::card(model, session) -> Card`: context
   bar against this chat's usage, output, price, images (with "this chat
   has pictures" when it does), source, effort dial with only
   `efforts`, scope. Snapshot per state.
5. **The ⏎ label, pure.** `use here | use and keep | sign in, then use
   | fix <provider> | check, then use | compact, then use`, from the
   row, reach, auth and window vs usage. Table test.
6. **Keys.** While open, the site's handlers take `↑↓` pick, `←→`
   effort, `tab` scope, `*` pin, `+` add, `⏎` act, `esc` back one layer
   (ADR-0086 §6). `/model next` (next pin) and `/think next` (next of
   `efforts`, wrapping) declare `alt+m` and `alt+t` (ADR-0086 §5).
   Open probes unchecked providers with `/models refresh <id>`; filters never act.
7. **Sheets.** Manage (test, change address, replace key, refresh,
   sign out — disabled for `from: env` —, remove with confirm, use
   anyway); sign-in (browser, device, paste) using the existing
   `Login` interaction drawn inside the panel, then chains to the
   pending model; add: a pasted `sk-ant-`, `sk-or-`, `:11434`, `:1234`
   or http(s) address prefills name, shape and address (bare `sk-` asks)
   (`picker::guess`, pure, table test), then `/provider add`, then the
   pasted key answers the paste interaction, then "pick one".
8. **Transcript line.** The mod draws the `⇄` outcome lines of M164
   dim with the glyph in ember; M170 owns `SessionStatus` and its status hub.
9. **Direct entries.** Bare `/think`, `/login`, `/logout`, `/models` open the
   matching place; arguments act as today.

## Decisions

- Rebased onto dev `46843d78`: removed `CommandRun.origin` in its introducing
  commit. `CommandRun.surface` selects terminal opening; signed `Input::Action`
  attributes internal commands to the models plugin. No contract extension.
- The existing `Pane` action focuses the picker; shared `site_keys` handles its
  key queue, like memory/status panes. No parallel lifecycle or pager is added.
  No picker line is run by the TUI and no composer prefill is needed; future
  such actions must use owner vouching and `composer::fill` respectively.
- The site mounts in a focused, bottom-anchored pulled pane (18 rows at 80×24);
  a late pane keeps its focus request. The mod owns filter and sheets; the pane
  title names a placement in words ("own place ▾"), never the wire's `rect`.
- Selection is the kept row's key, never an index and never the drawn row: ↑↓
  set it at once; a kept row that is gone takes no action and the cursor falls
  to the first model row, never a heading. Probes run concurrently after the
  picker is published, so rows change under a stable cursor. Amended in round 2:
  a keyboard key names only itself in every pane (`pane::key_handler`), since a
  queued key rides on a drawing the previous key already replaced; M174's
  current-drawing resolution is untouched.
- `/model` opens the picker bare, with a filter that has no `/`, or with
  `provider/`; a `provider/model` argument goes to the kernel, listed or not,
  and the kernel refuses an empty segment (InvalidInput). ⏎ on an empty match
  offers "use <provider>/<arg>", a bare word taking the current model's provider,
  so the switch never re-enters the picker. Bare `/model` elsewhere names the model, the pins
  and a count, never the whole catalogue. `next` walks `pinned` in its order.
- Sign-in and the add paste use sealed Login (ADR-0086 §7); keys never reach
  composer, handlers, props or journal, and pastes before a pane's editor opens
  are ignored. A pasted key is trimmed and unwrapped before it is kept, drawn as
  `••••••••  pasted` on one line, and `Guess`'s Debug redacts it.
- A bare `sk-` key names no vendor: the add sheet opens on its address and
  refuses to save without one. `sk-ant-` and `sk-or-` still name theirs.
- A pasted OpenAI address loses a trailing `/v1`: M165's adapter appends it.
- `⇄` receipts are recorded for every client: an interceptor turns a model or
  effort outcome into a recorded Notice tied to its intent, and the ack carries
  the same words. Print skips the item its own ack already printed. Round 2:
  only an Info notice (`ItemBody::receipt`) rides on the ack; Warn and Error
  stay items, so print keeps them on stderr. Channels say a receipt once, as
  the ack's card; the notice status is for acks that carry no words.
- Text's `attention` tone draws the ember glyph without hard-coded RGB.
- `/model next` and `/think next` are key bindings on `model` and `think`: a
  `KeyBinding` is a chord or `{chord, args}`. No catalogue entry has a space.
- A command spec's `opens` says running it bare opens where its argument is
  chosen; the models mod sets it for its five commands. The palette runs such a
  command bare or as typed (a filter reaches the picker), its hints one arrow away.
- The catalogue publishes resolved limits, `switchAt` and, through the
  `bingo.modelSwitch` service, the kernel's own compact-before-switch decision;
  the mod never recomputes either. A context nobody reported draws "—".
- Surface tests cover chords; host/RPC tests cover cycling and refresh. Guides
  are crate Pages; the full TUI guide splits provider details into `tui-providers`,
  keeping “Choosing a model” in `guide.md`. One member budget entry is added.

## Files

SDK contracts/generators; `bingo-mod-models`, bin, crate guides/snapshots; core
cycling/catalogue/errors; generic TUI paste/focus/text and smoke coverage.

## Exit criteria

- [x] Table tests for rows, card, label, guess; snapshot per prototype
      scenario (9) at 108×30 and 80×24.
- [x] `TestBackend` checks alt+m/alt+t dispatch; host/RPC checks pin cycling,
      effort wrap and unchecked providers asked once on open.
- [x] An outside JS mod wraps `ModelPicker` with `next` and gets the
      built-in's tree; with `bingo-mod-models` switched off, `/model`
      still lists and switches through the surface's fallback.
- [x] tmux drive in a temp HOME against wiremock providers, captured:
      switch, keep, pin, sign-in chain, add by paste, down provider,
      compact-before-switch. Screenshots compared with the prototype.
- [x] tui-smoke scenario and native gates pass.
- [ ] Windows: 19 touched crates pass; 9 await MSVC C headers/compiler
      for aws-lc-sys, rquickjs-sys or tree-sitter (details below).

## Non-goals

Desktop/chat drawings, mouse, editing `models` overrides. Existing follow-ups:
type-ahead while opening; `--model fake/fake-1` double prefix; typed keys say "pasted".

## Risks

- Static alt+m/alt+t win over band hotkeys; documented in the mod’s guide.
- Alt as ESC-prefix: covered by `late.rs` and tmux.
- A pasted key must never be drawn unmasked or journaled: test both.

## Verified — 2026-10-15

Earlier reviews: 5782 Rust / 31 TS passed, 6 ignored; native gates and smoke ×3 passed.
Prior local drives covered secret masking/journal absence, stored selection and provider sheets.
Rebase onto `46843d78`: resolved contracts in their introducing commits; one final glue commit; all runs unset `OPENAI_*`/`ANTHROPIC_*`.
- `cargo fmt --all -- --check`: exit 0; `cargo check --workspace --all-targets --locked`: Finished.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: Finished, exit 0.
- `RUST_TEST_THREADS=4 cargo test --workspace --locked --no-fail-fast`: 6039 passed; 0 failed; 6 ignored.
- After moving catalogue limits under `catalog/` to meet host.rs's line budget: 6038 passed; 1 PTY focus timing failure; 6 ignored. The focus test passed alone (1 passed). An earlier rewind timeout also passed alone and in the green full run; both are recorded wall-clock flakes.
- `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk --lib catalogue::`: 6 passed; `cargo test -p bingo-mods --lib dollar` with the same flag: 17 passed.
- `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc -p bingo-surface-rpc --lib`: 134 + 19 passed; `git status --short`: empty after regeneration.
- `node --test sdk/typescript/src/*.test.ts`: tests 32; pass 32; fail 0.
- `bash scripts/check_discipline.sh`: exit 0 (existing warnings); run.rs 956 non-test lines, `impl Run` in 3 files.
- `bash scripts/budget.sh`: dependencies 420 (max 420), warm core 0s, relink isolation 0 crates; budget ok (target-size warning).
- `cargo deny check`: advisories ok, bans ok, licenses ok, sources ok.
- `bash scripts/tui-smoke.sh` three consecutive runs: tui-smoke ok ×3; Ada persisted at every tested width.
- Isolated `tmux -L m166rb -f /dev/null`, temp HOME, fake provider: 108×30 and 80×24 picker open/filter/Enter/Escape, zeta and fake/ choices, status/plugins/sessions/memory and ctrl+f browse passed. Full filter text is checked with paced input; burst input can still be dropped during handoffs/redraws (existing type-ahead follow-up); an extra full-drive repeat stalled on sessions Escape after the completed drive passed it.
- `cargo check -p <each touched crate> --all-targets --target x86_64-pc-windows-msvc --locked`: 19 passed, including SDK/core/models/palette; 9 deferred on C-toolchain errors (missing stdlib.h/stdio.h/windows.h or unsupported MSVC flags), with no Rust errors reported.
- Deferred crates: bingo, auth-oauth, channels, mcp, mods, permissions, plugin-rpc, surface-print, surface-tui; aws-lc-sys/rquickjs-sys/tree-sitter builds require the MSVC C toolchain.
