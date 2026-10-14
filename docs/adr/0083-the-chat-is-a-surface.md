# ADR-0083 — The chat is a surface

Status: accepted · 2026-10-14 · Plan: M162 · Amends: ADR-0016, ADR-0079 §2, ADR-0082 §4

## Context

ADR-0079 §2 gave print and the IM channels one table, `text`: folds
only. ADR-0082 made every place a site but left the channels drawing by
hand: `deliver.rs` keeps the assistant's words, shell notes, notices and
command results, and `card.rs` builds the Feishu JSON. A mod that
redraws the answer in the terminal has no say in the chat. A Feishu
card is not a terminal, but it is not plain text either: it has
markdown, buttons, selects, forms, columns and collapsible panels.

Feishu also has a `/` menu (`app_slash_commands`, per app, ≤100,
description only), which the person sees when typing in the bot's chat.
Today the menu is empty and the person must know bingo's commands.

No kernel door opens: `HostApi::render` and `catalog` exist; this
record adds one surface name and one adapter capability.

## Decision

1. **`chat` is a surface table.** `SurfaceName::Chat` draws `Box`
   (column, row as columns), `Text` (bold, italic, strikethrough,
   `color` where the platform has it), `Code`, `Link`, `Image`,
   `Button`, `Select`, `Input` and `Engine`. Anything else draws its
   fold. `text` stays folds only, for print and plain channels.
2. **A channel asks as `chat` with no cell grid.** Its request carries
   `viewport { columns: 0, rows: 0 }`: zero means "not measured in
   cells"; a mod lays out by structure, not width. Terminal requests are
   unchanged.
3. **A channel asks only about what it draws, and only when a hook
   matches** (ADR-0079 §4): `AssistantMessage` once an answer is
   complete, `CommandOutput`, `InfoNotice`, `ShellRun`. While an answer
   streams, the channel streams its own drawing; the finished card is
   the site's answer. Permission and SignIn stay the channel's question
   ladder (ADR-0082 §6 sealing applies when a channel asks for them,
   which it does not yet).
4. **A pressed control is the existing door.** A `Button`, `Select` or
   `Input` submit in a card carries the minted `Action` (ADR-0079 §6) in
   its callback value; the channel submits `Input::Action` as the TUI
   does. A control from a card older than the mod's last reload answers
   with the same notice.
5. **An adapter publishes the command menu if it has one.**
   `ChannelAdapter::menu()` is a capability like `buttons()`: given the
   catalogue (`HostApi::catalog(Commands)`, hidden commands already
   gone) plus the adapter's own mapped words (`/new`, `/stop`), it
   brings the platform's menu to match. Feishu writes
   `app_slash_commands`: it creates and updates only names it wrote
   before, recorded in the channel's data directory, and deletes only
   those; a command the person made in the console is never touched.
   It syncs on start and on `CatalogChanged`, at most once a minute.
   A missing scope is one notice and no retry until the next start.
6. **The menu is per app, not per chat.** A mod's session command shows
   in every chat of that bot; picking a command the session lacks
   answers as any unknown command does.

## Consequences

- With no hook matching a chat site, Feishu output is byte-identical
  to today; `card.rs` remains the channel's own drawing (`Engine`).
- A mod that draws `AssistantMessage` with a `Button` gets a working
  button in Feishu and the terminal from one tree.
- The Feishu app needs `application:app_slash_command:write` and
  `:read`; `scripts/feishu-smoke.md` lists them. Menu changes take
  minutes to reach clients.
- Interaction sites (completion, pickers, forms as sites) remain the
  later record ADR-0082 names.

Refs: ADR-0008, ADR-0013, ADR-0016, ADR-0051, ADR-0073, ADR-0079, ADR-0082.
