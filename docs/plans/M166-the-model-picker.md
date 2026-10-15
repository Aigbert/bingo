# M166 — The model picker

ADR-0085 §8, ADR-0084 (`ModelPicker`, first interaction site),
ADR-0086 (a built-in mod). Needs M167's seam.
Approved prototype: https://claude.ai/artifact/XWoqqyP8Jch6p5889dL1ZB

## Goal

The built-in `bingo-mod-models` draws the approved model setup: alt+m/alt+t
without a panel; `/model <filter>` with a list, fact card and provider sheets.
Facts come from M164/M165 catalogues. The surface gains no picker code.

## Bricks

1. **Site.** `Component::ModelPicker`, props
   `{ filter, selected, scope, effort, sheet }` (ADR-0084 §1),
   `surfaces()` terminal and desktop; schema and TypeScript; snapshot.
2. **Catalogue with meta.** The mod reads `catalog(Models)` and
   `catalog(Providers)` with `meta` through the host, refreshed on
   `CatalogChanged`.
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
   Opening the picker sends `/models refresh <id>` for each unchecked
   provider. Typing a filter never acts.
7. **Sheets.** Manage (test, change address, replace key, refresh,
   sign out — disabled for `from: env` —, remove with confirm, use
   anyway); sign-in (browser, device, paste) using the existing
   `Login` interaction drawn inside the panel, then chains to the
   pending model; add: a pasted `sk-ant-`, `sk-or-`, `:11434`, `:1234`
   or http(s) address prefills name, shape and address (bare `sk-` asks)
   (`picker::guess`, pure, table test), then `/provider add`, then the
   pasted key answers the paste interaction, then "pick one".
8. **Transcript line.** The mod draws the `⇄` outcome lines of M164
   dim with the glyph in ember; the `SessionStatus` drawing remains owned by M170; reconcile `model ∴ effort` at its merge.
9. **Direct entries.** `/think`, `/login <id>`, `/logout <id>`,
   `/models` open the panel at the matching place when given no
   arguments; with arguments they act as today.

## Decisions

- `command.run` lacked the submitting surface, so it gained an optional `origin`
  for the terminal-only picker opening. M170's typed `CommandRun.surface` wins:
  `origin` and the `"tui"` filter are dropped when this branch rebases after M170.
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
- Chords are checked at the surface; cycling and refresh at host/RPC boundaries.
- Guides are owning-crate Pages. Existing dependencies are reused; the new
  workspace member raises the normal-tree budget by one.

## Files

- SDK site and command-origin contracts; generated schema and TypeScript.
- `bingo-mod-models`; binary registration; owning-crate guides and snapshots.
- Core cycling, catalogue limits, command origin and error propagation.
- Generic TUI sealed paste, pane focus and semantic text tone; smoke scenario.

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
- [ ] Windows checks for mods/TUI/bin await the MSVC C toolchain
      (aws-lc-sys/rquickjs-sys); SDK/core/models mod pass.

## Non-goals

Desktop/chat drawings; the merged palette; mouse; editing `models` overrides from the panel.
Follow-ups (round 2, not fixed): type-ahead dropped while a pane opens;
`--provider fake --model fake/fake-1` reports `fake/fake/fake-1`; the masked
line says "pasted" for a typed key.

## Risks

- alt+m/alt+t collide with a plugin band hotkey (`band::hotkey`): the
  static binding wins; documented in the models mod's guide.
- Alt as ESC-prefix: covered by `late.rs` and the tmux drive.
- A pasted key must never be drawn unmasked or reach the journal:
  test the frame text and the journal.

## Verified — 2026-10-15 (review fixes)

Runs unset OPENAI_*/ANTHROPIC_*; drives use an owned tmux socket, a temp HOME, the fake provider.

```text
cargo fmt --all -- --check: exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings: Finished, exit 0
RUST_TEST_THREADS=4 cargo test --workspace --locked: 5777 passed; 0 failed; 6 ignored
bash scripts/check_discipline.sh: discipline ok
bash scripts/budget.sh: 415/415 dependencies; warm core 1s; isolation 0; budget ok
cargo deny check: advisories ok, bans ok, licenses ok, sources ok
bash scripts/tui-smoke.sh (3 runs): tui-smoke ok ×3
node --test src/*.test.ts (sdk/typescript): pass 31; fail 0
cargo check -p bingo-mod-models -p bingo-sdk -p bingo-core --all-targets
  --target x86_64-pc-windows-msvc: exit 0
tmux 108×30 and 80×24: `/model fake/other` → "⇄ other · fake …", status fake/other;
  palette /model ⏎ opens "own place ▾"; ⏎ during the opening probes uses the drawn
  fake-1; a 200-char dummy paste draws "••••••••  pasted" on one line, bare sk- opens
  the address row; the dummy is in no frame and no file under the temp HOME;
  palette `/model fak` ⏎ opens "1 match for “fak”".
cargo test -p bingo --test model_picker_tmux --locked -- --ignored: 1 passed
  (owned socket, wiremock; 18 captures, none says "rect")
```

## Verified — 2026-10-15

Round 2. Runs unset OPENAI_*/ANTHROPIC_*; drives use an owned socket and temp HOME.

```text
cargo fmt --all -- --check: exit 0 · cargo check/clippy -D warnings --workspace: exit 0
RUST_TEST_THREADS=4 cargo test --workspace --locked --no-fail-fast: 5782 passed; 0 failed;
  6 ignored (a first run stopped on pty look::…within_one_focus; alone 3/3 pass)
check_discipline: discipline ok · budget: 415/415, budget ok · cargo deny: all ok
tui-smoke.sh ×3: tui-smoke ok · schema/TypeScript regenerated: no change
windows msvc: sdk, core, mod-models, mod-palette exit 0; channels, surface-print,
  surface-tui, bingo stop at aws-lc-sys (C toolchain deferral)
tmux, openai list alpha…gamma: /model openai/ Down Enter → openai/beta;
  Down Down Enter → openai/delta; /model zeta → "⏎ use fake/zeta", Enter → fake/zeta
/ledger key-to-frame, debug, fake provider, 35 keys per run, median/mean/max ms:
  dev 5422cac4 92/90/119, 91/89/117 · this branch 91/90/120, 93/96/224 (no regression)
```
