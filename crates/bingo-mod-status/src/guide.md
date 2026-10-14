# Status

`/status` opens a live card above the composer. Text-only runs keep the plain
status table. Use ↑↓ to select a row and
Enter to run its command: `/rename`, `/model`, `/compact`, `/permission`,
`/permission forget <rule>`, `/plugins`, or `/mcp`. The spent row is informational
and is skipped. Escape closes an idle card; during a turn, Escape stops the turn.

Context uses the provider's count when available. The `about` line estimates
system text, tool definitions, chat and pictures; its total can differ from that
count. Spent includes cached input. Dollar values are estimates at the current
model's catalogue rates, not a bill; changing models can change this estimate.
Missing prices and unpriced cache writes display `price unknown`.

Plugin failures, repository approvals and MCP sign-in problems appear on their
own rows. Long lists show the first name and `+N`. Selection follows the row's
label when live facts change. The card scrolls to keep the selection visible.

The feature is `bingo.mod.status`. Set
`enabledPlugins: {"bingo.mod.status": false}` to retain the kernel's plain
`/status` table. Outside mods can wrap or replace its `Status` drawing through
`ui.render`, or replace its command through `command.run` with rewrite trust.
