# M162 — The chat is a surface

ADR-0083.

## Goal

A mod's drawing of a chat site reaches Feishu as a real card (markdown,
columns, buttons, selects, inputs) whose controls work, and the bot's `/`
menu lists bingo's commands. With no mod matching, Feishu output is
byte-identical to today.

## Bricks

1. **`SurfaceName::Chat` and its table.** `view/table.rs` gains the `chat`
   arm (ADR-0083 §1); `Component::surfaces()` adds `chat` to
   `AssistantMessage`, `CommandOutput`, `InfoNotice`, `ShellRun`. Schema and
   TypeScript regenerated; `guide.md` names the surface. Table test per
   element, as `terminal` has.
2. **View → Feishu card, pure.** A new `bingo-channel-feishu` module turns a
   `View` (already checked against the `chat` table) into Card 2.0 elements:
   `Text` runs → `markdown` (escaped; bold/italic/strike/color where
   supported), column `Box` → stacked elements, row `Box` → `column_set`,
   `Code` → fenced markdown, `Link` → markdown link, `Image` → `img` via the
   existing upload path, `Button`/`Select` → `button`/`select_static` with
   the `Action` in `value`, `Input`s plus their submit → a `form`; `Engine`
   → the channel's own elements for that component; anything else → its
   fold as markdown. Respect Feishu's 200-element and ~30 KB caps: past
   them, the fold. insta snapshots of the JSON per element and per cap.
3. **Asking.** The channels surface plugin reads the render config (which
   components some loaded hook matches) and, only for a matched component,
   calls `HostApi::render` with `surface: chat`, `viewport {0,0}`, and the
   sdk props M159 defined. `deliver.rs` ops carry either the built-in text
   or a `View`; the adapter draws a `View` through brick 2 when it has
   `buttons()`, else posts `fold_markdown()`. `AssistantMessage` is asked
   once the answer completes; the streaming card is finished with the
   site's answer. Caching by (component, request id, props) like the TUI.
   `Event::Invalidated` for a delivered card re-asks and edits it in place
   when the adapter has `edit()`, else does nothing.
4. **Controls.** Feishu's `card.action.trigger` handler (`event.rs`) reads
   a minted `Action` from the callback value and submits `Input::Action`
   on the conversation's session, with the `value` for selects/inputs.
   The existing question buttons keep their own path. A control whose
   session or mod generation is gone answers with the notice.
5. **Menu capability.** `ChannelAdapter::menu() -> Option<&dyn Menu>`,
   `Menu::publish(&[MenuEntry])` (name, description). `Loopback` records
   it (the mock implementation). Channels host builds entries from
   `HostApi::catalog(Commands)` plus the adapter's mapped words, sorted
   by the catalogue's order, truncated at 100 with one notice line.
6. **Feishu menu.** `app_slash_commands` client in `bingo-channel-feishu`
   (list, create, patch, delete; `description.default_value` = the
   command hint). Owned names persist in the channel data dir as
   `slash-commands.json` `{name: command_id}`; create missing, patch
   changed descriptions, delete owned names no longer listed, never touch
   others; a name taken by a non-owned command is skipped with a notice.
   Runs at start and on `CatalogChanged`, debounced to once a minute; a
   permission error (scope missing) is one notice and stops until restart.
   Contract first: wiremock fixtures for the four calls.
7. **Docs.** `scripts/feishu-smoke.md` adds the two scopes and a live
   checklist (menu appears, a picked command runs, a mod's button card
   works); `docs/design/delivery.md` one dated line.

## Files

- `crates/bingo-sdk/src/{render.rs,view/table.rs}`; `schema/`,
  `sdk/typescript/` regenerated; `crates/bingo-mods/src/guide.md`.
- `crates/bingo-channel-adapter/src/{adapter.rs,loopback*}` (menu).
- `crates/bingo-channels/src/{deliver.rs,runner.rs,host.rs}` and a small
  new module for asking sites.
- `crates/bingo-channel-feishu/src/{card.rs,event.rs,mod.rs}`, new
  `view.rs` (brick 2) and `menu.rs` (brick 6).
- Fixture mod under `crates/bingo/tests/fixtures/mods/` drawing
  `AssistantMessage` with a `Button` on `chat`.
- `scripts/feishu-smoke.md`, `docs/design/delivery.md`.

## Exit criteria

- [ ] `chat` table test per element; `surfaces()` test updated.
- [ ] View → card snapshots for every element, a form, caps exceeded.
- [ ] No existing Feishu or channels snapshot changes; a count test shows
      no render request when no hook matches.
- [ ] Black-box over loopback (`crates/bingo/tests/channels.rs` style):
      the fixture mod's answer arrives as its tree; pressing its button
      submits the minted action and the mod's state changes.
- [ ] Menu: wiremock tests for create / patch / delete-owned-only /
      skip-foreign / scope-missing / 100 cap; loopback records entries;
      `CatalogChanged` re-publishes once within the debounce.
- [ ] Gates: fmt, check, clippy -D warnings, test, check_discipline, budget,
      cargo deny, Windows check (or recorded deferral).
- [ ] Live Feishu check by the person (Codex never runs it): menu shows
      bingo's commands, picking `/model` runs it, the fixture mod's button
      works. Recorded as pending until the person reports.

## Non-goals

Interaction sites (completion, pickers, forms as sites); permission and
sign-in as chat sites; per-chat menus; i18n descriptions; other channels'
menus; drawing reasoning or tool calls in Feishu; new dependencies.

## Risks

- What Feishu sends when a menu command is picked is undocumented; assume
  `/name` text through `im.message.receive_v1` and confirm live.
- Card JSON caps and markdown escaping: a mod's text must never inject
  card markup; escape and test with hostile strings.
- A finished card replaced by a site's answer must not lose the footer
  (`footer.rs`) or break the 8-minute stream renewal.
- Menu writes count against the app's rate limits; never more than one
  sync in flight per app (the app lock already holds one process).
