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
   pending model; add: a pasted `sk-ant-`, `sk-or-`, `sk-`, `:11434`,
   `:1234` or http(s) address prefills name, shape and address
   (`picker::guess`, pure, table test), then `/provider add`, then the
   pasted key answers the paste interaction, then "pick one".
8. **Transcript line.** The mod draws the `⇄` outcome lines of M164
   dim with the glyph in ember; the `SessionStatus` drawing remains owned by M170; reconcile `model ∴ effort` at its merge.
9. **Direct entries.** `/think`, `/login <id>`, `/logout <id>`,
   `/models` open the panel at the matching place when given no
   arguments; with arguments they act as today.

## Decisions

- `command.run` lacked the submitting surface: add optional, pinned `origin` so
  terminal commands can open the site without intercepting headless print commands.
  Forwarding a terminal error must not produce an interceptor-failure notice.
- Mount the interaction site in an existing focused, bottom-anchored pulled pane.
  The mod owns selection, filter and sheets; the surface keeps placement and the FIFO.
  A normal Above seat reserves eight transcript rows and clips the card at 80×24;
  the existing bottom-anchored Rect seat preserves the approved 18-row interaction.
- A pane arriving while the palette closes must retain its opening focus request;
  generic pane sync now claims it once the composer is free, without FIFO changes.
- Sign-in and the initial add paste use sealed Login (ADR-0086 §7) above the
  prompt. Its private reply can infer a provider without exposing a key to
  the composer, handlers, props or journal; the mod owns no credential editor.
- Login Paste opens its sealed masked editor immediately; bracketed paste routes to
  that editor; pastes before a focused pane opens its editor are ignored. The terminal
  drive exposed the old composer fall-through. No FIFO changes.
- Choice receipts were acknowledgement-only, so the mod converts successful `⇄`
  command outcomes (including thinking receipts) to transcript notices before drawing them. The fallback keeps
  M164's plain acknowledgements; only terminal InfoNotice links draw receipts.
- Text's semantic `attention` color reuses the badge tone to draw the ember glyph
  without hard-coded RGB values or changing the theme's spending allowlists.
- Register catalogue commands `model next` and `think next` for default key lines;
  the existing parser dispatches their base command and argument. No new key wire format.
- The command-describe link marks site-opening commands as argument-free for the
  palette; the real commands still accept explicit arguments. This avoids a second
  generic argument list swallowing `/model` before the picker can open.
- Pasted OpenAI API addresses lose a trailing `/v1` at the command boundary:
  M165’s adapter appends that path itself. The reviewed address stays visible.
- The old catalogue omitted configured/learned limits, and window × 90% ignored
  reserved output. Publish resolved limits and `switchAt` from the existing kernel
  resolver/threshold brick; the mod consumes them instead of reimplementing the ruler.
- Chords are checked at the surface boundary; cycling and refresh at host/RPC
  boundaries, because the surface is a client and owns no choice state.
- Guides are owning-crate Pages, including the TUI Choosing a model section.
- Existing dependencies are reused; the new workspace member raises the
  normal-tree budget by one. No external package is added.

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

## Risks

- alt+m/alt+t collide with a plugin band hotkey (`band::hotkey`): the
  static binding wins; documented in the models mod's guide.
- Alt as ESC-prefix: covered by `late.rs` and the tmux drive.
- A pasted key must never be drawn unmasked or reach the journal:
  test the frame text and the journal.

## Verified — 2026-10-15

All runs unset OPENAI_*/ANTHROPIC_*; terminal drives use owned sockets, temp homes and local mocks.
Command output summaries (Windows crates checked separately):

```text
cargo fmt --all -- --check: exit 0
cargo check --workspace --all-targets --locked: Finished dev profile
cargo clippy --workspace --all-targets --locked -- -D warnings: Finished dev profile
cargo test --workspace --locked: 5689 passed; 0 failed; 6 ignored
bash scripts/check_discipline.sh: discipline ok (no new picker warnings)
bash scripts/budget.sh: 415/415 dependencies; warm core 0s; isolation 0; budget ok
  target/debug: 10 GB (soft max 5); final cleanup follows commits
cargo deny check: advisories ok, bans ok, licenses ok, sources ok
bash scripts/tui-smoke.sh: tui-smoke ok
BINGO_UPDATE_SCHEMA=1 generator lib tests (SDK/mods/plugin-rpc/store/rpc): 443 passed; 1 ignored
npm test (sdk/typescript): pass 31; fail 0
cargo test -p bingo --test model_picker_tmux --locked -- --ignored --nocapture: 1 passed
cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc:
  bingo-sdk, bingo-core, bingo-mod-models: Finished dev profile
  bingo-mods, bingo-surface-tui, bingo: exit 101; known MSVC C-toolchain deferral
```

18 TestBackend snapshots and 18 captures in `/tmp/m166-picker-captures` were compared with the local prototype.
