# M189 — Status and plugins in short

ADR-0095. `/status` and `/plugins` in the prototype's rows.

## Goal

status-1, status-3 and the MCP scenarios match: short id, `~` paths, branch,
effort, `$` spent, rule rows with revoke, plugin counts, an MCP empty state.

## Bricks

1. **Facts**: picture count in the context breakdown; plugin manifest
   description, load time, MCP tool names, log path; fixture tests.
2. Status rows at 12-column labels: session (title · short id · `~`cwd · branch),
   model (`∴ effort` · provider reach), context bar and breakdown, `$` spent,
   permission with rule rows and `⏎ revoke`, plugin counts; `up` from session
   start.
3. Plugins: groups yours / this project / built in / MCP; card fields `loaded
   in`, `adds`, `draws`; MCP `none configured` empty state.

## Files

- `crates/bingo-mod-status`, `crates/bingo-mod-plugins`, `crates/bingo-sdk`, `crates/bingo-mcp` (tool names).

## Exit criteria

- [ ] Snapshot per scenario; revoke removes the rule (test).
- [ ] tmux drive with captures; gates as in M182.

## Non-goals

Editing rules from the status panel beyond revoke.

## Risks

- Revoke is a permission change: it goes through the permissions plugin's own
  verb, never by editing its record from the panel.
