# M166 — The model picker

ADR-0085 §8, ADR-0084 (`ModelPicker`, first interaction site).
Approved prototype: https://claude.ai/artifact/XWoqqyP8Jch6p5889dL1ZB

## Goal

The terminal draws the approved model setup: alt+m and alt+t with no
panel, `/model <filter>` as a list and a fact card grown up from the
prompt, and provider sheets (manage, sign in, add by pasting) in the
same panel. Every fact comes from the catalogues M164 and M165 fill.

## Bricks

1. **Site.** `Component::ModelPicker`, props
   `{ filter, selected, scope, effort, sheet }` (ADR-0084 §1),
   `surfaces()` terminal and desktop; schema and TypeScript; snapshot.
2. **Catalogue with meta.** `run.rs::fetch_catalogue` keeps `meta`
   (today only ids), one store per catalogue kind, refreshed on
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
6. **Keys.** `↑↓` pick, `←→` effort, `tab` scope, `*` pin, `+` add,
   `⏎` act, `esc` back one layer; alt+m (`/model <next pinned>`) and
   alt+t (`/think <next of efforts>`, wrapping) in `keys.rs` BINDINGS.
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
8. **Transcript line.** The `⇄` outcome lines of M164 drawn dim with
   the glyph in ember; footer shows `model ∴ effort`.
9. **Direct entries.** `/think`, `/login <id>`, `/logout <id>`,
   `/models` open the panel at the matching place when given no
   arguments; with arguments they act as today.

## Files

- `crates/bingo-sdk/src/{render.rs,component.rs}`; `schema/`,
  `sdk/typescript/`.
- `crates/bingo-surface-tui/src/{run.rs,keys.rs,input.rs,commands.rs}`,
  new `picker/{mod.rs,rows.rs,card.rs,label.rs,guess.rs,sheet.rs,
  draw.rs}`, `status.rs` footer.
- `scripts/tui-smoke.sh` scenario; snapshots.

## Exit criteria

- [ ] Table tests for rows, card, label, guess; snapshot per prototype
      scenario (9) at 108×30 and 80×24.
- [ ] `TestBackend`: alt+m cycles pins, alt+t wraps, a picker open →
      unchecked providers asked once.
- [ ] tmux drive in a temp HOME against wiremock providers, captured:
      switch, keep, pin, sign-in chain, add by paste, down provider,
      compact-before-switch. Screenshots compared with the prototype.
- [ ] tui-smoke scenario passes; gates as M164.

## Non-goals

Desktop and chat drawings of the picker; the merged palette (its own
milestone); mouse; editing `models` overrides from the panel.

## Risks

- alt+m/alt+t collide with a plugin band hotkey (`band::hotkey`): the
  static binding wins; document it in the mods guide.
- Terminals that send alt as ESC-prefix: covered by `late.rs`; drive in
  tmux to confirm.
- A pasted key must never be drawn unmasked or reach the journal:
  test the frame text and the journal.
