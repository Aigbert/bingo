# ADR-0086 — Built-in features are mods

Status: accepted · 2026-10-15 · Plans: M167 (the seam), then each redesigned screen's milestone (first M166) · Amends: ADR-0082 §1, ADR-0084 §3 §5

## Context

ADR-0082 made the surface's own drawing the innermost answer of a
site, so a mod can replace a screen. The feature behind the screen is
still the surface's: the terminal owns `/help` and `/resume`, the
`ctrl+g` switcher, the `?` list and the completion menu; a mod gets
keys only through pane controls and band hotkeys; there is no way to
bind a key to anything a mod offers. The person's word (2026-10-14):
every part is a built-in mod that an outside mod can replace, from what
it does to how it is drawn. In-process interceptors sit outermost today
(tier Full, `host/registry.rs`), the wrong end for a default.

The ratchet: no `HostApi` method is added. Additive sdk contracts
(a link's placement, a command's default keys, a root key handler and its
handled acknowledgement) and one settings key. M167 keeps handlers on the
existing supervised action mailbox; ready render answers need no channel hop.

## Decision

1. **A built-in mod is a Rust plugin that owns one feature whole.** It
   registers the feature's commands, its catalogue entries and the
   drawing of its sites through the contract an outside mod uses;
   nothing of the feature lives in a surface. It is compiled in, listed
   in `crates/bingo/src/main.rs`, named `bingo-mod-<noun>`, and is
   switched off by `enabledPlugins` like any plugin (ADR-0057).
2. **It draws with the shared vocabulary.** A built-in answers
   `ui.render` with a `View` tree (Box, Text, List, Table, Progress,
   Input, …), synchronously, in process; it never imports a surface or
   a terminal crate. One drawing serves the terminal and the desktop;
   it may read the asking surface (ADR-0082 §5) and differ. Where the
   vocabulary cannot say what a screen needs, the vocabulary grows in
   `bingo-sdk`; no private path to a surface.
3. **Built-ins are innermost.** A link declares `Placement::Builtin`;
   the chain orders outside mods, then other in-process links, then
   built-ins (plugin list order), then the surface's `Engine` answer.
   An outside mod wraps a built-in with `next` or answers alone; its
   tier (ADR-0074, `pluginTrust`) decides as for any link: replacing a
   command still needs `rewrite`.
4. **The surface's drawing is the fallback.** `Engine` stays the last
   answer: plain, complete, used when the built-in is switched off or
   refused, and for sealed sites.
5. **Keys bind to commands.** A command may declare default keys
   (`/model next` ← `alt+m`); the `keys` settings key overrides them;
   the surface reads the bindings and runs the command line. The `?`
   list is the bindings, so it lists an outside mod's keys too. A
   static surface key (esc, ctrl+c, the composer's editing keys) cannot
   be bound.
6. **The open site owns its keys.** While a built-in's interaction site
   holds focus (a picker, the palette), the surface routes keys to the
   site's minted handlers (ADR-0079 §6) as it does for pane controls;
   an unhandled key returns to the surface. Selection and filter are
   the owner's state, not the surface's.
7. **What stays with the surface:** the composer's editing core
   (buffer, cursor, IME, history, paste), focus and layout, `/exit`,
   `/clear`, and the sealed sites: the permission ask, sign-in, and the
   KeyBar while one of them is open (ADR-0082 §6, ADR-0084 §4). A mod may interpret
   a question's keys and return a chosen answer in its command acknowledgement; the surface
   resolves it through the person's handle. Plugin handles carry kernel-stamped provenance,
   independent of their claimed client name or activation. The kernel refuses every permission
   answer from a plugin and records the source of accepted answers.

## Consequences

- A redesigned screen ships as a built-in mod; M166's picker lives in
  `bingo-mod-models`, not `bingo-surface-tui`. Old screens move when
  redrawn, never as a port.
- `/help` and the `?` list move to `bingo-mod-palette`; `/resume` moves
  with the session switcher once a surface handler opens a session.
- An outside mod test wraps each built-in with `next` and asserts it
  receives the built-in's tree (M167).
- Switching every built-in off still leaves a usable terminal.
- M167 adds the test-only `bingo-mod-hello` member: the normal-tree budget
  counts workspace members, so 411 → 412; no external dependency is added.
- `check_discipline.sh` treats `bingo-mod-*` as plugins: no surface,
  no ratatui, no other plugin.

- M169 adds the compiled-in `bingo-mod-ask` member without external
  dependencies; the normal-tree budget counts it, so 412 → 413.

- M168 adds `bingo-mod-palette`; the member budget grows 413 → 414 without
  external dependencies. `CommandOutcome::Surface` carries a client action in
  the existing intent acknowledgement. The host never dispatches that action.
  An optional fallback message explains terminal-only actions on other surfaces;
  terminal-family commands are excluded from chat menus.
- The global command catalogue has no session. `CommandCatalog`, a typed service,
  overlays `Command::now` using a borrowed `SessionView` supplied by its caller;
  current values do not trigger I/O or select a hidden active session.

Refs: ADR-0057, ADR-0074, ADR-0079, ADR-0082, ADR-0084, ADR-0085.

M177 adds `bingo-mod-browse`: the member budget grows 414 → 415 with no
external dependency. Its guide is a per-crate Page. The checkpoints preview
contract lives in `bingo-services`; neither plugin imports the other.

M170 adds `bingo-mod-status` without external dependencies; the normal-tree
budget counts the workspace member, so 418 → 419. It supplies Status facts
from the asking surface's fold; drawing never attaches a session.
`CommandRun.surface` keeps shared-host text callers on their fallback. The
Composer surface action prefills editing; `Acknowledged` forwards an existing
command result without repeating its record or action.
