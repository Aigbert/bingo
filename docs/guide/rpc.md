# RPC wire compatibility

`bingo serve --stdio` speaks JSON-RPC over NDJSON. Events carry the SDK's frames;
`schema/rpc.json` is generated from the Rust types. See [ADR-0007](../adr/0007-rpc-wire.md)
for transport and answering semantics.

## M197: permission caution and command stakes

A permission's optional `caution` now emits an object instead of a string:

```json
{"reason":"rewrites remote history","act":"force-push"}
```

Clients that read `caution` as a string must read `caution.reason` instead. The optional
`act` names the action an approval allows; it may be absent. Old journals with a bare
string still load as `{ "reason": <old string> }`, with no act. New writes use the object.
A caution continues to remove the session-wide approval rung. Permission answers and
`session/answer` are unchanged.

A command preview may additionally carry optional `stakes`:

```json
{"line":"drops 1 commit that only origin/dev has","items":["4e1a9c2 wip: queue experiments"]}
```

Absent stakes are omitted. Bash computes them only for an explicit force push of one
remote and branch. The count includes every commit in the local remote-tracking ref
that the local branch lacks; items retain the first five log lines. This lookup has a
500 ms deadline, never fetches, and supplies no stakes on failure (including an older Git
that lacks `--no-lazy-fetch`). Remote-tracking refs
can be stale. All other wire changes are additive; generated schemas and TypeScript
SDK types describe the new fields.
