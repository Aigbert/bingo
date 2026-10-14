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

### Implementation refinements

- A pure card renderer accepts uploaded image keys; I/O uploads relative to the
  session cwd before drawing. Upload failure uses the image's alt text.
- Each Input owns a submit form, because each has its own minted onSubmit;
  onInput alone folds. No callback action is trusted until it matches the
  retained host answer and session. Old controls use the runtime's notice.
- The completed turn is the chat's AssistantMessage (its streaming delivery
  already joins the turn's prose). Command input stays with its pending intent:
  acknowledgements carry neither the command name nor its arguments.
- ConfigChanged also wakes the shared menu publisher: session registrations
  refresh config today without emitting CatalogChanged. Both streams coalesce.
  Loopback menus opt in so existing fixtures stay byte-identical.
- Ownership lives in an app-keyed subdirectory beside attachments, so two app
  credentials cannot accidentally claim each other's names.

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

- [x] `chat` table test per element; `surfaces()` test updated.
- [x] View → card snapshots for every element, a form, caps exceeded.
- [x] No existing Feishu or channels snapshot changes; a count test shows
      no render request when no hook matches.
- [x] Black-box over loopback (`crates/bingo/tests/channels.rs` style):
      the fixture mod's answer arrives as its tree; pressing its button
      submits the minted action and the mod's state changes.
- [x] Menu: wiremock tests for create / patch / delete-owned-only /
      skip-foreign / scope-missing / 100 cap; loopback records entries;
      `CatalogChanged` re-publishes once within the debounce.
- [x] Gates: fmt, check, clippy -D warnings, test, check_discipline, budget,
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

## Verified — 2026-10-14

Final command output summaries (offline only; no credentials or live Feishu):
```text
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk: passed
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-surface-rpc: passed (1 ignored)
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc: passed
cargo fmt --all -- --check: exit 0
cargo check --workspace --all-targets --locked: Finished dev, 6.34s
cargo clippy --workspace --all-targets --locked -- -D warnings: Finished dev, 7.05s
cargo test --workspace --locked: 5483 passed; 0 failed; 5 ignored
bash scripts/check_discipline.sh: discipline ok (advisory length warnings)
bash scripts/budget.sh: budget ok; 411/411 dependencies; warm core 1s; relink 0
cargo deny check: advisories ok, bans ok, licenses ok, sources ok
bash scripts/tui-smoke.sh: tui-smoke ok (isolated socket and temporary HOME)
cargo check --workspace --all-targets --target x86_64-pc-windows-msvc: exit 101
  Deferred: rquickjs-sys / aws-lc-sys C toolchain; Windows headers unavailable.
cargo check -p bingo-sdk --all-targets --target x86_64-pc-windows-msvc --locked: Finished dev, 6.41s
```
Targeted: adapter 34, channels 109, Feishu 132 tests passed; loopback black-box
17 passed, including minted actions, forged callbacks, reload and reset. No
existing snapshots changed. Initial runs exposed a transient gateway claim
failure (isolated retry and final suite passed) and the guide's line cap (fixed).
Budget reported 7 GB versus its 5 GB soft limit; final worktree cleanup follows.
The person's live menu/event/button check remains **pending**, not verified.
