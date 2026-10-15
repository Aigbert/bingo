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
   chain on its own line under `draws`; a load error in full, wrapped,
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
- Core catalogue/commands: link order, owned contributions and person-owned trust.
- SDK `Plugins` site/props and `PluginSource::contents`; modules/process source projections.
- TUI site placement/keys/lifecycle, palette exact-line/choice payload integration, binary
  registration, fixtures, CLI and TestBackend tests.
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

- Two owners' verbs in one panel can drift: every key is a command
  line, and the verbs table test lists each (row kind, key) pair.
- A server list that changes while the panel is open (a handshake
  lands) moves rows: the cursor follows the row by name.

## Decisions

- The prototype's needed examples are illustrative. ADR-0057's existing store/surface rule
  remains authoritative; permission and filesystem plugins remain switchable.
- Add `/plugins trust <name> <tier>` and optional approval tier arguments: no command
  currently writes trust. Both use the user-layer writer; writing every tier also reduces an existing broader grant.
- Server rows retain typed transport configuration for ACP forwarding. Tool counts and
  run/address text derive from status/transport instead of duplicating those facts.
  Origins come from registrar sources, retained after approval; unknown latency stays absent.
- Guides use the new crate's registered Page (ADR-0054). No docs/guide directory is added.
- Add only workspace edges for the new mod and its test consumers. The normal dependency
  count grows 414 → 415 because the budget counts workspace members.
- The template's registered guide is a typed static `Pages` service. Classifying it as a
  runtime service pinned every documented mod to next-start. Treat only typed `Pages`
  as catalogue facts; ordinary services still pin. The live-disable integration test exposed this.
- Trust changes read fresh user grants at registration and use existing module retirement
  before reenabling a reloadable generation; process/tool grants apply at next start.
  Unreadable live settings suspend new admission instead of substituting default grants.
  Handlers carry opening identity only and reread catalogue facts when pressed.
- External command ownership is held by each source, not the native registry. Add a
  default-empty typed `PluginSource::contents` projection, implemented by module and
  process sources from their live registrations, so cards also name external commands
  and tools without another cache or a guessed owner.
- The real terminal exposed two palette integration gaps: an exactly typed command with
  optional arguments entered completion instead of invoking its default; Enter now runs
  the exact command/alias and Tab still completes. Palette handlers carry only catalogue
  ids/labels used by argument choices; full metadata stays in render props. This avoids
  traversing irrelevant schemas through an outside wrapper's handler tree.
- The same source projection supplies origin/runtime before admission; these are discovery
  facts, while a grant remains absent until admission. The tmux capture exposed the gap.
- Surface-action fallbacks retain Markdown tables so chat keeps its existing table dialect;
  print receives a plain-text table. The queued `/mcp` channel test caught a lossy fold.

## Verified — 2026-10-15

Verification used isolated HOME/data/config directories and fake/local fixtures; gates ran sequentially.

```text
cargo fmt --all -- --check                                  exit 0
cargo check --workspace --all-targets --locked               Finished dev profile; exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings  exit 0
cargo test --workspace --locked                             5,737 passed; 5 ignored; 173 suites (aggregate)
bash scripts/check_discipline.sh                            discipline ok
bash scripts/budget.sh                                      415/415 dependencies; 0 core recompiles; budget ok
cargo deny check                                           advisories ok, bans ok, licenses ok, sources ok
bash scripts/tui-smoke.sh                                   tui-smoke ok
cargo check -p <crate> --all-targets --locked --target x86_64-pc-windows-msvc
  PASS: bingo-sdk, bingo-services, bingo-core, bingo-provider-acp, bingo-mod-plugins, bingo-mod-palette
  DEFER: bingo-mcp, bingo-plugin-rpc, bingo-mods, bingo-surface-tui, bingo
  all five: aws-lc-sys v0.44.0 custom build failed; Windows stdlib.h/windows.h unavailable
node --test src/*.test.ts (sdk/typescript)                   31 passed; 0 failed
```
`BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk --locked`: passed; then `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc -p bingo-surface-rpc -p bingo-mods -p bingo-store-jsonl --lib --locked`: passed; generated schema and TypeScript committed.
`python3 /tmp/m173-tmux-drive.py`: PASS; 13 captures at 108×30/80×24 compared with the local prototype; approval/full confirmation, user-only trust, disable/enable/reconnect and table fallback checked. Own isolated tmux socket and local echo-server fixture only; temporary HOME/repo removed.
Twelve TestBackend snapshots cover six scenes at both sizes; key tests cover live/next-start/needed switches, trust and MCP states. Both command fallbacks have integration coverage.
No timing flake observed; the counter smoke passed on the first final run. Discipline advisories and the budget's 28 GB soft target warning remain; final `cargo clean` removes build artifacts.
