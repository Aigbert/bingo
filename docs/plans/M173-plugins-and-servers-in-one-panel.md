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

1. **Typed server rows.** `bingo_services::mcp::Servers::rows() ->
   Vec<ServerRow { name, transport, origin, status, auth,
   latency, sign_in_from }>`; tools/runs derive from status/transport.
   This replaces the untyped map (wire face
   and fixture updated; the ACP crossing reads the same rows).
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
6. **Trust list.** `t` (and `⏎` on a waiting plugin) opens tighten /
   rewrite / full (and `not now` while waiting), one sentence each;
   `full` needs a second `⏎`; the pick runs `/plugins approve` while waiting, otherwise `/plugins trust`,
   and writes `pluginTrust` for every tier. The footer says `kept in
   ~/.bingo/settings.toml, never the repo`.
7. **Drawing and keys.** `Plugins` site: header line with counts and
   the legend, two panes split at the prototype's ratio, one `❯` row;
   keys through `onKey`. Snapshots per scenario at 108×30 and 80×24
   (at 80 the card goes under the list).

## Files

- `crates/bingo-services/src/mcp.rs`; `crates/bingo-mcp/src/{source.rs,
  rows.rs}`; `crates/bingo-provider-acp/src/crossing.rs`.
- New `crates/bingo-mod-plugins/src/{lib.rs,groups.rs,cards.rs,chain.rs,
  verbs.rs,trust.rs,draw.rs,keys.rs}`; listed in `crates/bingo/src/main.rs`.
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

- Two owners' verbs can drift: every key is a command line in the verbs table. A
  handshake landing while the panel is open moves rows: the cursor follows the name.

## Decisions

- The prototype's needed examples are illustrative; ADR-0057's store/surface rule stays
  authoritative, so permission and filesystem plugins remain switchable.
- `/plugins trust <name> <tier>` and approval tier arguments write through the user-layer
  writer for every tier, which also narrows a broader grant.
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
  first `⏎` reviews, the second approves. Rows nobody may dial carry the program only, so
  the review names `/bin/sh` and not its arguments (deliberate: args may hold secrets).
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

## Verified — 2026-10-15

First round: fmt, check, clippy, test (5,737), discipline, budget, deny, tui-smoke passed;
schema/TypeScript regenerated, `node --test` 31 passed; tmux drive 13 captures PASS.

Review round, with the four provider variables unset for every cargo run:

```text
cargo fmt --all -- --check                                      exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings  exit 0
RUST_TEST_THREADS=4 cargo test --workspace --locked             5,797 passed; 0 failed; 5 ignored
  after the last fixes: bingo-surface-tui 1,185, bingo-core+tui 1,785, bingo-mod-plugins 25
bash scripts/check_discipline.sh                                discipline ok
bash scripts/budget.sh                                          415/415; budget ok
cargo deny check                                                all four ok
bash scripts/tui-smoke.sh                                       tui-smoke ok (three runs)
cargo check --target x86_64-pc-windows-msvc
  PASS: bingo-mod-plugins, bingo-sdk, bingo-services
  DEFER: bingo-mcp (aws-lc-sys: stdlib.h unavailable)
```
Own tmux (`-L m173fix`), temp HOME, fake provider, at 108×30 and 80×24: a project server
whose command touches a marker file. Two `space`s, then one `⏎`: notice and review card,
no marker. Second `⏎`: marker appears, the row connects, the command is cut beside its key.
