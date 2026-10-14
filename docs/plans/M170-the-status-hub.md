# M170 — The status hub

ADR-0084, ADR-0086. Needs M167 (placement, `onKey`); prices from M164.
Approved prototype (2026-10-14, "状态 · 插件 · MCP" tab, `/status`
scenarios): https://claude.ai/artifact/XWoqqyP8Jch6p5889dL1ZB

## Goal

`/status` is one card grown up from the prompt, and every row is a
door: `⏎` on a row runs the command that changes it. Session (rename),
model (`/model`), context with what fills it (compact now), spent (not
selectable), permission mode (`/permission`), each rule this session
accepted (forget it), plugins (`/plugins`), MCP (`/mcp`). Problems are
written on their row in the colour of the problem: a plugin that failed
to load, one from this repository waiting, a server that needs sign-in
or is not running. The feature is the built-in mod `bingo-mod-status`;
the kernel's `/status` key-value view stays as the plain fallback.

## Bricks

1. **What fills the context.** `ContextUsage.parts: Vec<ContextPart>`
   (additive; `system`, `tools`, `chat`, `pictures`, each tokens):
   `turn.rs::measure` already estimates the system blocks and the
   messages; it keeps the four sums. Fixture; a test that the parts
   sum to `used` within one count's rounding.
2. **What it cost.** Pure `spent(usage, price) -> Spent { input, output,
   cache_share, usd: Option<f64> }`; `usd` is `None` where M164's
   catalogue has no price. Table test.
3. **Forget a rule.** `/permission forget <rule>` in `bingo-permissions`
   (`ArgSpec` values from this session's rules); answers the rule it
   dropped. Test over a session with two rules.
4. **Rows, pure.** `rows(&SessionState, plugins, servers) -> Vec<Row>`
   in the mod: label, value spans, tone (`ok`, `warn`, `bad`, `dim`),
   and the command line `⏎` runs (`None` for spent). Plugins and
   servers are summarised to one row each: counts, then each problem
   by name. Table test per prototype scenario.
5. **Drawing.** `Status` site as a `View` tree: a title line with the
   hint (`live; ⏎ on a row goes there`) and version · uptime, one `❯`
   row, the context bar with its parts on a dim second line, the rules
   indented under permission. At most half the screen; snapshot per
   scenario at 108×30 and 80×24.
6. **Live.** The card redraws from frames while open (a turn finishing,
   a mode change, a server connecting); the cursor stays on its row by
   label, not index. `TestBackend` test: a `ConfigChanged` while open.
7. **Keys.** `↑↓` move over selectable rows, `⏎` runs the row's command
   and closes the card (the next screen opens in its place), `esc`
   closes. Through `onKey` (M167).

## Files

- `crates/bingo-sdk/src/event.rs` (`ContextPart`); `schema/`,
  `sdk/typescript/`; `crates/bingo-core/src/{turn.rs,context/budget.rs}`.
- `crates/bingo-permissions/src/command.rs` (`forget`).
- New `crates/bingo-mod-status/src/{lib.rs,rows.rs,spent.rs,draw.rs,
  keys.rs}`; listed in `crates/bingo/src/main.rs`.
- Reads `catalog(Plugins)` and `bingo_services::mcp::find` (no plugin
  import).

## Exit criteria

- [ ] Parts fixture and sum test; spent and rows tables; forget test.
- [ ] Snapshots: overview, long chat with two rules, a failed plugin,
      a waiting one, an MCP server needing sign-in, at both sizes.
- [ ] `TestBackend`: each row's `⏎` reaches its command; spent is
      skipped by `↑↓`; esc closes; live redraw keeps the row.
- [ ] `enabledPlugins: {"bingo.mod.status": false}`: `/status` draws the
      kernel's key-value view.
- [ ] tmux drive in a temp HOME against the fake provider, captured and
      compared with the prototype; tui-smoke; gates as M167.

## Non-goals

Editing a value in the card itself (each row goes to its own screen);
a cost history across sessions; the desktop's drawing.

## Risks

- The context split is an estimate and can disagree with the server's
  count: the bar shows `used` from the server, the parts line says
  `about`, and the sum test allows the rounding.
- A summary row that names every failing thing can overflow at 80
  columns: the row truncates names after the first with `+N`.
