# ADR-0087 — What a repository brings waits for a yes

Status: accepted · 2026-10-15 · Plan: M171 · Amends: ADR-0015 §1, ADR-0057 §6

## Context

Opening a repository runs its code. `.bingo/plugins/<name>` is spawned
or loaded at start (`bingo-plugin-host/src/discovery.rs`), at `tighten`
with the unsafe nouns stripped (`grant.rs`), and shadows a plugin of
the same name the person installed. A project-layer `mcpServers` entry
is dialled: a stdio server is a command the repository chose, run with
the person's rights. `pluginTrust` is already user-only
(`settings/merge.rs`); being *loaded at all* is not. The approved
prototype (2026-10-14, "状态 · 插件 · MCP") shows a repo's plugin
waiting with `waits for your yes` until the person picks a tier.

The ratchet: only the kernel knows which layer an entry came from.
Refusing the door would make `bingo-mcp` read settings files itself, a
second representation of the layering. One `HostApi` write verb, a read-only
SDK approval service, one field on a settings claim, one kernel settings key. Discovery and reload read the same
authority; refusing the read seam would make runtimes parse settings again.

## Decision

1. **Nothing a repository brings runs before a yes.** A project-layer
   plugin directory is discovered and listed, never spawned or loaded,
   until approved. A project-layer entry under a key a plugin claims as
   runnable (`ConfigClaim.approve`, e.g. `mcpServers`) is handed to
   its owner marked waiting; the owner never dials it.
2. **An approval names the repository, the thing and its bytes.** The
   user layer keeps `approved` (a kernel key; ignored with a notice in
   any other layer, like `pluginTrust`): project root → `"plugin:<name>"`
   or `"<key>:<name>"` → `sha256:<hex>`. A plugin's digest covers every
   file in its directory (relative path and bytes, symlinks as their
   target text); an entry's covers its canonical JSON. A changed digest
   waits again and says `changed since your yes`.
3. **The verb.** `HostApi::approve(Approval { what, root })` writes
   the user layer and answers the digest it recorded; `/plugins approve
   <name>`, `bingo plugins approve <name>` (headless twin) and the
   owners' verbs (`/mcp approve <name>`) are its callers. Picking
   `rewrite` or `full` in the panel also writes `pluginTrust`, as today.
4. **What a yes starts.** An approved JS mod loads now through the
   reload lifecycle (M157); a process plugin or one with tools at the
   next start (ADR-0088); an approved MCP entry is dialled now, as
   `/mcp enable` does.
5. **A waiting plugin shadows nothing.** While a project plugin waits,
   the person's own plugin of that name stands (amends ADR-0015 §1);
   after the yes the project's wins the name as before.
6. **It is said, not asked over the chat.** No dialog at start. The
   status line counts waiting items (`1 from this repo waits for you`),
   `/status` and `/plugins` list them with their reason; a headless
   start prints one `PROJECT_WAITING` notice naming them and the
   command that approves them. `PluginStatus` gains a typed state,
   `waiting { changed }` among them (M171).

## Consequences

- A repository whose CI relies on its own plugin adds `bingo plugins
  approve <name>` to its setup; the notice names it.
- `bingo-mcp` gains `Status::Waiting`; `/mcp` lists it with `approve`.
- Approvals never travel with a repository and never apply to another
  checkout path; a moved checkout asks again.
- A plugin directory over the digest cap (8 MiB or 2 000 entries) cannot
  be approved by digest and says so; vendoring belongs outside it.
- The digest is `bingo-digest` (the library tier's own SHA-256), a new
  workspace edge from `bingo-core`; no external dependency.

Refs: ADR-0003, ADR-0015, ADR-0057, ADR-0074, ADR-0088.

Implementation decisions (2026-10-15): directory fingerprints are always read;
mtime and size cannot prove unchanged bytes. Escaping/looping links are refused
at approval even though the digest itself hashes link text. `pluginDirs` uses
the generic personal-only accumulation rule so a repository cannot classify its
own code as explicitly supplied. Waiting catalogue rows have command-prefixed
IDs (`plugins:name`, `mcp:name`), distinct from a running user namesake.

Review decisions (2026-10-15): approval and each spawn verdict refuse a process
entry whose command or argument names a path outside the canonical plugin root
(after `${PLUGIN_ROOT}` substitution). Unresolvable paths fail closed and name
the entry. Bare PATH programs remain allowed. Digesting a runtime's own module
lookup outside the root (PATH programs, global `node_modules`) is a non-goal.
RPC approval is person input: an RPC client is a person's surface by definition.
The reverse plugin bridge has no approval verb, at any trust tier.
