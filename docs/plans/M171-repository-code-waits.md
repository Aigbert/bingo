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
   io::Result<String>` over sorted relative paths and bytes
   (symlinks as their target text, never followed), capped at 8 MiB or
   2 000 entries; `digest_entry(&Value)` over canonical JSON. Uses
   `bingo-digest`. Table tests, including a symlink loop and a Windows
   path separator normalised to `/`.
2. **The record.** Kernel key `approved` (ADR-0003 §2 list, object-key
   merge, user layer only; `APPROVED_IGNORED` notice elsewhere). Pure
   `verdict(approved, root, what, digest) -> Yes | Waiting { changed }`.
   Table test; merge test that a project layer cannot approve itself.
3. **The verb.** `HostApi::approve(Approval { root, what })` computes
   the digest, writes the user layer (ADR-0058 keeps comments), answers
   the digest. Wire and fixture as ADR-0007; refused on the plugin
   bridge at every trust tier.
4. **Plugins wait.** Discovery reads the verdict before admission; a
   waiting project plugin is `PluginState::Waiting { changed }` and its
   user-layer namesake stands (ADR-0087 §5). `PluginStatus` replaces
   `enabled` + `reason` with `state: PluginState { On, Off { why },
   Failed { why }, Waiting { changed }, Pending }` (schema regeneration,
   fixture, TypeScript). Tests with a temp project dir: waiting, yes,
   edit a file → waiting changed, user namesake stands.
5. **Entries wait.** `ConfigClaim.approve: &[&str]` names runnable
   keys; the slice handed to the owner carries `waiting: Vec<Waiting {
   root, key, name, changed }>` for project-layer entries without a matching
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

- Core approval authority, settings/provenance, host verb and plugin commands.
- SDK approval/status/source contracts; RPC methods; `schema/`, `sdk/typescript/`.
- Plugin-host discovery/admission; process bridges; module discovery/reload.
- MCP configuration, manager, commands and rows; ACP adapter registration.
- Binary subcommands/notices and plugin-owned guides.

## Decisions during implementation

- The SDK read-only `ApprovalReader` service gives discovery and reload current
  kernel verdicts; runtimes must not duplicate settings parsing.
- `approved` uses nested object-key merge, since `ByName` handles arrays.
- Digest caps include directories and path bytes. Non-UTF-8 paths, Unix literal
  backslashes, escaping/looping links are refused; hashes record link text.
- `pluginDirs` uses `PersonalAccumulate`: repository values cannot reclassify
  their code as explicit. Waiting IDs have command prefixes, distinct from
  running personal namesakes. Repository modules generate no author files.
- Origins are typed and repository roots canonicalized at load; home settings
  are never reread as project settings. Startup runtimes receive resolved `--cwd`.
- MCP's live owner supplies waiting catalogue entries and blocks every dial,
  login and export path. Guides live in plugin-owned pages (`docs/guide` is absent).
- Digest I/O failures fail closed. Schema/RPC additions preserve protocol 2;
  approval is person-input-only and absent from every reverse bridge tier.
- The rebase takes rustls 0.23.46 from dev; no separate TLS patch remains.
- The sole new edge is core → bingo-digest, the planned pure-library exception;
  Ember allowlists and external dependencies are unchanged.
- Review: approval and spawn validate explicit command/argument paths inside
  the canonical plugin root. Bare PATH programs stay allowed; unresolvable
  paths fail closed. Runtime module lookup outside the root is not digested.
- Runnable claims accept dotted paths: `acp.adapters` gates each adapter before
  registration/probing. `/acp approve` records a yes for the next start.
- Reload selects canonical roots, registration completes only pending entries,
  and refused reloads release their attempt. `/plugins approve` uses discovery's
  root, never the session directory; a source method avoids copying provenance.
- MCP waiting metadata is independent of dial state; approval preserves Disabled.
  Layer constructors and file readers require origin, including explicit callers.
- RPC approval is person input; the reverse plugin bridge still has no verb.
  The binary owns headless command hints; kernel hints stay generic.
- Rebase exceeded structural limits: move HostConfig and resume-configuration
  tests into their own modules; group presentation scripts in the SDK test host.

## Exit criteria

- [x] Digest and verdict tables; merge refuses a project approval.
- [x] `PluginState` fixture; every existing `enabled`/`reason` reader
      moved (TUI, print, RPC, ACP, channel adapter).
- [x] Temp-repo tests: a project plugin never spawns before the yes,
      runs after it, waits again after an edit; a project MCP entry is
      never dialled before the yes.
- [x] Black-box: headless notice, `bingo plugins approve`, stdout
      purity, exit status 0.
- [ ] All gates, including Windows checks of every touched crate;
      native gates passed; Windows C-toolchain deferrals remain below.

## Non-goals

Signing or publishing plugins; approving a whole repository at once;
approving anything that is not code a repository brings (hooks scripts
in project settings are a later case, named in the guide).

## Risks

- Repositories that rely on their plugin in CI break once: the notice
  names `bingo plugins approve`, and the guide's upgrade note says so.
- Digesting on every start costs bounded I/O; always hash. Mtime and size
  can be preserved across hostile edits and are not approval evidence.
- Follow-up: namespace `pluginTrust` by repository/origin and name. Existing
  grants remain name-keyed, so an approved project namesake can inherit personal
  trust. This needs a persisted-key migration plus grant-writing surface and
  registration-identity changes; changing only runtime lookup would orphan
  existing grants. Approval does not imply a separate trust decision yet.

## Verified — 2026-10-15

Review regressions were red before fixes and green afterwards: launch escape,
ACP registration/provenance, refused reload, root selection, pending admission,
discovery-root approval, disabled MCP, required origin, refusal notices and hints.
No live or credential tests. Final sequential command/output summaries:

```text
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc -p bingo-surface-rpc --lib → 134 + 16 passed; schema/TypeScript regenerated, no diff
cargo fmt --all -- --check → exit 0
cargo check --workspace --all-targets --locked → Finished dev profile; exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings → Finished dev profile; exit 0
cargo test --workspace --locked → 5559 passed; 0 failed; 5 ignored (166 summaries)
bash scripts/check_discipline.sh → discipline ok (advisory warnings remain)
bash scripts/budget.sh → 412/412 dependencies; warm core 0s; isolation 0; budget ok (9 GB soft warning)
cargo deny check → advisories ok, bans ok, licenses ok, sources ok
bash scripts/tui-smoke.sh → tui-smoke ok (isolated socket and temporary HOME)
cargo check -p EACH --all-targets --target x86_64-pc-windows-msvc --locked → 10 pass; 10 C-toolchain deferrals
```

Windows pass (`bingo-` prefix): core, demo-ui, experience, hooks-shell, persona,
plugin-host, provider-acp, schedule, sdk, surface-rpc.
Deferred: bingo; bingo-{channels,mcp,mods,permissions,plugin-rpc,provider-anthropic,
provider-openai,surface-tui,tool-web}. aws-lc-sys lacks `stdlib.h`/`windows.h`;
permissions' tree-sitter crates lack `stdio.h`/`stdlib.h`; bingo/mods also hit
rquickjs-sys MSVC flags rejected by host clang (`/std:c11`, `/experimental:c11atomics`).
No Rust source diagnostic appeared. The cross-platform criterion stays unticked.
