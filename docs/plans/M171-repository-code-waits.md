# M171 — Repository code waits for a yes

ADR-0087. Independent of M167; M173 draws on it.

## Goal

A plugin under `.bingo/plugins` and a project-layer `mcpServers` entry
are listed but never started until the person approves them for this
checkout and these bytes. The approval lives in the user settings
layer; a changed plugin or entry waits again. Every surface says what
waits and how to approve it; nothing asks over the chat.

## Bricks

1. **Digest, pure.** `bingo_core::approval::digest_dir(root) ->
   Result<Digest, TooBig>` over sorted relative paths and bytes
   (symlinks as their target text, never followed), capped at 8 MiB or
   2 000 files; `digest_entry(&Value)` over canonical JSON. Uses
   `bingo-digest`. Table tests, including a symlink loop and a Windows
   path separator normalised to `/`.
2. **The record.** Kernel key `approved` (ADR-0003 §2 list, `ByName`
   merge, user layer only; `APPROVED_IGNORED` notice elsewhere). Pure
   `verdict(approved, root, what, digest) -> Yes | Waiting { changed }`.
   Table test; merge test that a project layer cannot approve itself.
3. **The verb.** `HostApi::approve(Approval { root, what })` computes
   the digest, writes the user layer (ADR-0058 keeps comments), answers
   the digest. Wire and fixture as ADR-0007; refused on the plugin
   bridge for `tighten` plugins.
4. **Plugins wait.** Discovery reads the verdict before admission; a
   waiting project plugin is `PluginState::Waiting { changed }` and its
   user-layer namesake stands (ADR-0087 §5). `PluginStatus` replaces
   `enabled` + `reason` with `state: PluginState { On, Off { why },
   Failed { why }, Waiting { changed }, Pending }` (schema bump,
   fixture, TypeScript). Tests with a temp project dir: waiting, yes,
   edit a file → waiting changed, user namesake stands.
5. **Entries wait.** `ConfigClaim.approve: &[&str]` names runnable
   keys; the slice handed to the owner carries `waiting: Vec<Waiting {
   key, name, changed }>` for project-layer entries without a matching
   yes. `bingo-mcp` claims `mcpServers`, files those as
   `Status::Waiting`, never dials them, and `/mcp approve <name>` calls
   the verb then dials. Tests in `bingo-mcp` with a fake host.
6. **Saying it.** `/plugins approve <name>` and `bingo plugins approve
   <name>` (headless, before any host); a `PROJECT_WAITING` notice at
   start naming each item and the command; the status line count
   (`1 from this repo waits for you`), read from the catalogue.
   Black-box: `bingo --print` in a temp repo prints the notice on
   stderr and keeps stdout pure.

## Files

- New `crates/bingo-core/src/approval.rs`; `settings.rs`,
  `settings/merge.rs`, `host/` (verb), `commands/plugins.rs`.
- `crates/bingo-sdk/src/{plugin.rs,host.rs}`; `schema/`,
  `sdk/typescript/`; `crates/bingo-surface-rpc` method.
- `crates/bingo-plugin-host/src/{discovery.rs,admission.rs}`;
  `crates/bingo-plugin-rpc/src/source.rs`; `crates/bingo-mods/src/load.rs`.
- `crates/bingo-mcp/src/{config.rs,manager.rs,command.rs,rows.rs}`.
- `crates/bingo/src/` (subcommand); `docs/guide/` plugins and MCP pages.

## Exit criteria

- [ ] Digest and verdict tables; merge refuses a project approval.
- [ ] `PluginState` fixture; every existing `enabled`/`reason` reader
      moved (TUI, print, RPC, ACP, channel adapter).
- [ ] Temp-repo tests: a project plugin never spawns before the yes,
      runs after it, waits again after an edit; a project MCP entry is
      never dialled before the yes.
- [ ] Black-box: headless notice, `bingo plugins approve`, stdout
      purity, exit status 0.
- [ ] Gates as M167, plus Windows check of `bingo-core`,
      `bingo-plugin-host` and `bingo-mcp` (paths, symlinks).

## Non-goals

Signing or publishing plugins; approving a whole repository at once;
approving anything that is not code a repository brings (hooks scripts
in project settings are a later case, named in the guide).

## Risks

- Repositories that rely on their plugin in CI break once: the notice
  names `bingo plugins approve`, and the guide's upgrade note says so.
- Digesting on every start costs I/O: only project plugins are read,
  under the cap, and the digest is skipped when the directory's newest
  mtime and size match the last approval's (kept beside it).
