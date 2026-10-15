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

1. **Typed server rows.** `Servers::rows() -> Vec<ServerRow { name, transport, origin,
   status, auth, latency, sign_in_from }>` replaces the untyped map (wire face and
   fixture updated; the ACP crossing reads the same rows).
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

Installing plugins from a registry; editing a server's entry in the
panel; per-tool switches inside a server; the desktop's drawing.

## Risks

Verbs drift between owners: every key is a command line in the verbs table. A handshake
landing while the panel is open moves rows: the cursor follows the name.

## Decisions

- The prototype's needed examples are illustrative; ADR-0057's store/surface rule stays
  authoritative, so permission and filesystem plugins remain switchable.
- `/plugins trust <name> <tier>` and approval tiers write the user layer for every tier.
- Server rows keep typed transport for ACP forwarding; tools, runs and address derive from
  status/transport; origins come from registrar sources; unknown latency stays absent.
- Guides use the crate's registered Page (ADR-0054). Only typed `Pages` are catalogue facts;
  other services still pin a mod to next start. Workspace edges only: 414 → 415 members.
- Trust changes reread user grants and retire the module before reenabling; process/tool
  grants apply at next start. Unreadable settings suspend admission. Handlers carry opening
  identity only and reread catalogue facts when pressed.
- External ownership stays with each source: a default-empty `PluginSource::contents`
  projection names external commands and tools, and supplies origin/runtime before admission.
- Enter runs an exactly typed command with optional arguments; Tab still completes. Palette
  handlers carry ids/labels only. Fallback tables stay Markdown for chat, plain for print.

### Review round (2026-10-15)

- A repo server waiting for approval runs nothing on one key: `space`/`r` explain, the
  first `⏎` reviews, the second approves (the review's content: third round below).
- Privileged panel lines (`/plugins approve|trust`, `/mcp approve`) run only when the kernel
  vouches: `Drawings` records the compiled-in mod that drew the site; the ack carries
  `drawnBy` only for its own answer to unrewritten args. A mod drawing over a built-in site
  disables them (fail closed). Panel keys answer only the drawing pressed on, in this session.
- An approval writes its trust only once it stands; a reenable that fails says
  `/plugins enable <name>`; an unreadable trust file is named once. Bare-name trust keys
  stay as ADR-0087 §5 left them: a residual risk.
- Errors are cut inside the card, not wrapped; `runs`/`from` are cut in the middle at 38
  columns. A failed server keeps its last stderr line (4 KiB tail, ANSI stripped, redacted,
  240 chars). OAuth shows only when in play. The plain-drawing note derives from sites.
- Lines a panel or key runs are `Effect::Run`: sent, never written to either history.
  Lines composed in the palette are the person's own and stay in history.
- A placement is said once, in its status. A switch asked mid-turn lands after it.

### Third round (2026-10-15)

- What the person typed is what the person asked: `command.run` binds person input only to
  the arguments typed. `ArgSpec::Words.person` names the verbs only the person may run
  (`/plugins approve|trust`, `/mcp approve`, ACP `approve`); their lines pin `/args`, and no
  description or completion may drop those words. The TUI refuses them, `!` lines and
  unknown commands from a view nothing vouches for.
- A waiting server's review shows the line a yes runs, args, url and cwd verbatim; env and
  header values are hidden, names kept. Other rows nobody may dial stay program-only. An
  approval still holds only for the entry reviewed (digest of the whole merged entry).
- On a palette level of person-only lines a row's label is the id it runs, the catalogue's
  label only its hint; enter refuses a person-only line whose last argument the row hides.
- Vouching names the row and the site: the press must carry the row the drawing showed, and
  `drawnBy` counts only when it names the site's owner (`plugins`, `palette`).
- `render::vouched()` is bound for a vouched press whose arguments reach the command
  unchanged; the plugin panel ignores any other press (no cursor, review or line).

## Verified — 2026-10-15

First round: fmt, check, clippy, test (5,737), discipline, budget, deny, tui-smoke passed;
schema/TypeScript regenerated, `node --test` 31 passed; tmux drive 13 captures PASS.

Review round: the same gates (5,797 passed; 5 ignored), tui-smoke three runs, Windows
checks PASS for bingo-mod-plugins, bingo-sdk, bingo-services, DEFER bingo-mcp (aws-lc-sys).
Own tmux at 108×30 and 80×24: a project server ran only on the second `⏎`.

Third round: fmt, clippy, test (5,811 passed; 5 ignored), discipline ok, budget ok, deny
ok, `node --test` 31 passed, tui-smoke three runs ok; Windows PASS for bingo-sdk,
bingo-services, bingo-mod-plugins, bingo-mod-palette, DEFER bingo-core/TUI (aws-lc-sys). Own
tmux at 108×30 and 80×24: the review showed the full line with `--flag=visible`, `env
SECRET_TOKEN (values hidden)`, no value; nothing ran until the second `⏎`.
