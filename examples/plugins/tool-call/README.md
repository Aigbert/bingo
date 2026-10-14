# Tool call example

Run `bingo --plugin-dir examples/plugins/tool-call`. This original module denies
edits to `.env`, `.env.*`, and `protected.txt`, rewrites the exact Bash command `echo before` to
`echo after`, and counts calls to Bash, Read and Edit in the status line.
Bash whitespace is trimmed, and a failure status clears on success.
Denied calls do not reach `tool.call` and are not counted. A matched ToolUse
hook wraps Bash rows around their Engine drawing.

Explicit loading grants the rewrite tier, with only state and UI operations.
The module does not grant permission to a tool; the normal policy still applies.
State is session scoped and survives a module reload. No credentials are used.
