# Status

`/status` opens a live card above the composer. Text-only runs keep the plain
status table, including channels sharing the terminal host. Use ↑↓ to select a row.
Enter on session or permission prefills `/rename ` or `/permission ` for editing.
An existing draft stays intact; clear it to accept the prefill.
Other rows run `/model`, `/compact`, `/permission forget <rule>`, `/plugins`, or
`/mcp`. Queued commands report their position immediately. The spent row is
informational and is skipped. Escape closes an idle card; during a turn, Escape
stops the turn while ordinary panes retain their own close behavior.

Context uses the provider's count when available. The `about` line estimates
system text, tool definitions, chat and pictures; its total can differ from that
count. Spent includes cached input. Dollar values are estimates at the current
model's catalogue rates, not a bill; changing models can change this estimate.
Missing prices and unpriced cache writes display `price unknown`.

Plugin failures, repository approvals and MCP sign-in problems appear on their
own rows. Long lists show the first name and `+N`. Selection follows the row's
label when live facts change. The card scrolls to keep the selection visible.
Unviewed cards stop ticking after five seconds; rendering again restores the
displayed selection and live updates. Simultaneous viewers of one
session share selection because the render contract has no viewer identifier.

The feature is `bingo.mod.status`. Set
`enabledPlugins: {"bingo.mod.status": false}` to retain the kernel's plain
`/status` table. Outside mods can wrap or replace its `Status` drawing through
`ui.render`, or replace its command through `command.run` with rewrite trust.
