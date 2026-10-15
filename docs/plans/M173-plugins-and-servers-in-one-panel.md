# M173 — Plugins and servers in one panel

ADR-0084, ADR-0086, ADR-0087, ADR-0088. Needs M167, M171, M172.
Approved prototype (2026-10-14, "状态 · 插件 · MCP" tab, `/plugins`
and `/mcp` scenarios): https://claude.ai/artifact/XWoqqyP8Jch6p5889dL1ZB

## Goal

`/plugins` (alias `/modules`) and `/mcp` open one two-pane panel; the
opener's group comes first. Left: yours, this project's, built in
(folded to one row: `10 built in · all on · 1 drawn over`), MCP
servers (`their tools always ask`). Right: the card of the row under
the cursor, with what it adds, what it draws and the chain of who
draws over whom, its trust, its error as written. `space` switches,
`⏎` does the row's primary action, `r` reloads or reconnects, `t`
picks a trust tier, `o` signs out of a server. The feature is the
built-in mod `bingo-mod-plugins`; the commands' table views stay as
the plain fallback.

## Bricks

1. **Typed server rows.** `Servers::rows() -> Vec<ServerRow>` replaces the untyped map
   (wire face, fixture and the ACP crossing read the same rows).
2. **Draw chain, pure.** `chain_of(site, links) -> Vec<String>` from
   the catalogue's `ui.render` links in dispatch order, ending in
   `plain` (`my-picker ▸ bingo-mod-models ▸ plain`). Table test.
3. **Groups and rows, pure.** `groups(opened, plugins, servers) ->
   Vec<Group>`; built-ins fold to one row that unfolds with `⏎`;
   glyph and tag from `PluginState` and `Status`. Table test per
   prototype scenario.
4. **Cards, pure.** `plugin_card` and `server_card` → fact rows; a
   chain on its own line under `draws`; a load error cut to the card,
   in the error tone; trust with one sentence of what the tier may do.
5. **Verbs as command lines.** Every key maps to an existing command
   line so the panel holds no second path: `space` → `/plugins enable|
   disable` or `/mcp enable|disable`; `⏎` on a waiting plugin → the
   trust list; on a server by state → `/mcp tools`, `/mcp login`,
   `/mcp reconnect`, `/mcp enable`; `r` → `/plugins reload` or `/mcp
   reconnect` (only when connected or failed; otherwise a toast says
   why); `o` → `/mcp logout`. A needed plugin's `space` is refused with
   ADR-0057 §3's reason. Table test from (row, key) to line.
6. **Trust list.** `t` (and `⏎` on a waiting plugin) opens tighten / rewrite / full (and
   `not now` while waiting), one sentence each; `full` needs a second `⏎`; the pick runs
   `/plugins approve` while waiting, otherwise `/plugins trust`, and writes `pluginTrust`
   for every tier. The footer says `kept in ~/.bingo/settings.toml, never the repo`.
7. **Drawing and keys.** `Plugins` site: header line with counts and the legend, two panes
   split at the prototype's ratio, one `❯` row; keys through `onKey`. Snapshots per
   scenario at 108×30 and 80×24 (at 80 the card goes under the list).

## Files

- `crates/bingo-services/src/mcp.rs`; `crates/bingo-mcp/src/{source,rows}.rs`; ACP `crossing.rs`.
- New `crates/bingo-mod-plugins/src/{lib,groups,cards,chain,verbs,trust,draw,keys}.rs`.
- Core catalogue/commands; SDK `Plugins` site and `PluginSource::contents`; source
  projections; TUI placement/keys, palette integration, fixtures, CLI and TestBackend tests.
- `crates/bingo-mod-plugins/src/guide.md` registered by `guide.rs`; generated schema/TypeScript.

## Exit criteria

- [x] Row fixture; chain, groups, cards and verbs tables.
- [x] Snapshots: `/plugins`, a failed plugin, a waiting one with the
      trust list, a built-in drawn over, `/mcp`, sign-in, both sizes.
- [x] `TestBackend`: opener order; `space` on a draw-only mod (now), a
      tool plugin (`off at next start`), a needed one (refused); trust
      `full` needs two `⏎`; MCP `⏎` per state; `r` refused while
      signing in.
- [x] `bingo-mod-plugins` switched off: both commands draw their tables.
- [x] tmux drive in a temp HOME with a temp repo plugin and a fake MCP
      server, captured and compared with the prototype; tui-smoke;
      local gates as M167.
- [ ] Complete Windows checks for the five C-dependent crates listed below;
      the accepted Windows C-toolchain deferral applies.

## Non-goals

Installing from a registry; editing a server's entry; per-tool switches; the desktop.

## Risks

Verbs drift: every key is a verbs-table line. A handshake moves rows: the cursor follows.

## Decisions

- The prototype's needed examples are illustrative; ADR-0057's store/surface rule stays
  authoritative, so permission and filesystem plugins remain switchable.
- `/plugins trust <name> <tier>` and approval tiers write the user layer for every tier.
- Server rows keep typed transport for ACP forwarding; tools, runs and address derive from
  status/transport; origins come from registrar sources; unknown latency stays absent.
- Guides use the crate's registered Page (ADR-0054). Only typed `Pages` are catalogue facts;
  other services still pin a mod to next start. Workspace edges only: one member more.
- Trust changes reread user grants and retire the module before reenabling; process/tool
  grants apply at next start. Unreadable settings suspend admission. Handlers carry opening
  identity only and reread catalogue facts when pressed.
- External ownership stays with each source: a default-empty `PluginSource::contents`
  projection names external commands and tools, and supplies origin/runtime before admission.
- Enter runs an exactly typed command with optional arguments; Tab still completes. Palette
  handlers carry ids/labels only. Fallback tables stay Markdown for chat, plain for print.

### Review round (2026-10-15)

- A waiting repo server runs nothing on one key: `space`/`r` explain, `⏎` reviews, a second
  `⏎` approves. Privileged lines (`/plugins approve|trust`, `/mcp approve`) run only when the
  kernel vouches: `Drawings` records the compiled-in mod that drew the site, the ack carries
  `drawnBy` only for its own answer to unrewritten args; a mod drawing over a built-in site
  disables them (fail closed). Panel keys answer only the drawing pressed on.
- An approval writes trust only once it stands; a failed reenable says `/plugins enable
  <name>`; an unreadable trust file is named once. Bare-name trust keys: residual risk.
- Errors are cut inside the card; `runs`/`from` cut in the middle at 38 columns. A failed
  server keeps its last stderr line (4 KiB tail, stripped, redacted, 240 chars). Lines a
  panel runs are `Effect::Run`, never in history; palette-composed lines stay in history.
  A placement is said once; a switch asked mid-turn lands after it.

### Third round (2026-10-15)

- `command.run` binds person input only to typed arguments. `ArgSpec::Words.person` names
  person-only verbs (`/plugins approve|trust`, `/mcp approve`, ACP `approve`); their lines
  pin `/args`. The TUI refuses them, `!` lines and unknown commands from unvouched views.
- A waiting server's review shows the line a yes runs, args, url and cwd verbatim; env and
  header names kept. Approval holds only for the entry reviewed (digest of the merged entry).
- Person-only palette rows are labelled by the id they run. Vouching names the row and the
  site (`drawnBy` must be the site's owner); `render::vouched()` binds only for unchanged
  arguments, and the panel ignores any other press.

### Fourth round (2026-10-15)

- `mcp::LOADS_CODE` (row.rs, owned by `Transport::reviewed`) lists the env names that change
  what code a process loads or where it finds programs; the review shows their values marked
  `(loads code)`, other values and every header stay hidden. A stdio server's env is `env`,
  `sign-in` is a remote server's auth. Wrapped card values hang under the value column; a
  relative cwd reads `in ./sub, from where bingo started` (the child's dir is bingo's cwd).
- `CommandSpec::person_verb` is the one reader of person-only verbs (sdk, plugins command,
  ACP approval, palette); a palette row must show every untyped argument, contiguously.
- `Input::Action` carries an optional `origin`; it is asked only when signed by a surface in
  the sdk's `PERSON_SURFACES` (tui, print, rpc, acp, channels), else fail closed. RPC signs a
  client's unsigned action `rpc`; a mod's `$.command.run` is `plugin:<name>`, so a pane it
  opens waits for room. `/plugins reload|enable` stay not person-only (neither raises trust);
  follow-up: `enable` can undo a person's `off`.
- On M178: the panel opens and closes through `site_lifecycle`/`site_keys` like the other
  sites; browse runs a person-only line only when vouched by `browse`, as the palette does.

## Verified — 2026-10-15

Rounds 1–3: all gates, test 5,811 passed, tui-smoke ×3; Windows PASS sdk, services,
mod-plugins, mod-palette, DEFER bingo-mcp/core/TUI (aws-lc-sys). Own tmux at 108×30 and
80×24: a project server ran only on the second `⏎`; no secret value shown.
Fourth round: all gates, test 5,821 passed, `node --test` 31, tui-smoke ×3; Windows PASS
sdk, services, mod-plugins, mod-palette, surface-rpc. tmux NODE_OPTIONS repro: the review
shows `env NODE_OPTIONS=--require ./evil.js  (loads code)`, `env TOKEN (values hidden)`.
On M178 (rebased): all gates, test 5,950 passed, `node --test` 32, tui-smoke ×3; tmux 108×30:
/plugins shows db's review card; /sessions, /memory and ctrl+f browse open and close.
