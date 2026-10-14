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

- New `crates/bingo-core/src/approval.rs`; `settings.rs`,
  `settings/merge.rs`, `host/` (verb), `commands/plugins.rs`.
- `crates/bingo-sdk/src/{plugin.rs,host.rs}`; `schema/`,
  `sdk/typescript/`; `crates/bingo-surface-rpc` method.
- `crates/bingo-plugin-host/src/{discovery.rs,admission.rs}`;
  `crates/bingo-plugin-rpc/src/source.rs`; `crates/bingo-mods/src/load.rs`.
- `crates/bingo-mcp/src/{config.rs,manager.rs,command.rs,rows.rs}`.
- `crates/bingo/src/` (subcommands); plugin-owned modules and MCP guides.

## Decisions during implementation

- The SDK exposes a read-only `ApprovalReader` service alongside the write
  verb. Discovery and reload need current kernel verdicts before selecting
  a name; refusing that seam would duplicate settings parsing in runtimes.
- `approved` is a nested object, merged by object key in the user layer;
  `Merge::ByName` only handles arrays, so it is not used for this record.
- Count directories and path bytes toward digest caps, reject non-UTF-8
  paths and Unix literal backslashes, and refuse approval of escaping or
  looping symlinks. Digesting alone still records link text without following.
- Repository `pluginDirs` could reclassify repository code as explicit and
  bypass discovery approval. Its claim now uses `PersonalAccumulate`: user
  and explicit layers still work; repository contributions are ignored with
  a notice. This generic merge rule keeps provenance in the kernel.
- Waiting catalogue IDs use `plugins:<name>` or the owning command prefix,
  so an unapproved namesake cannot borrow an active plugin's grant.
- Repository modules do not generate author files at startup: that would
  change the bytes just approved. Author generation stays on personal dirs.
- A typed layer origin replaces the user boolean; repository roots are
  canonicalized while loading, keeping the settings merge pure. A user
  settings path is not reread as project settings when cwd is home.
- Waiting MCP entries join the catalogue through their live owner, which
  also refuses config export and all dial/login paths until approval.
- Guide updates live in the plugin-owned guide pages; `docs/guide` does not
  exist in this checkout.
- Startup runtimes now receive the resolved `--cwd`: using process cwd
  silently discovered a different repository in the black-box test.
- Directory digest errors use `io::Result<String>`: refused paths and I/O
  failures must fail closed alongside size limits, not become a partial hash.
- Schema definitions and RPC methods are additive; the frozen plugin protocol
  fixture remains compatible, so no protocol version changes. Approval is
  person-input-only and absent from every reverse plugin bridge tier.
- The advisory gate requires rustls 0.23.45 → 0.23.46 (RUSTSEC-2026-0346
  through 0349); update that existing package only, without adding dependencies.
- The only new dependency edge is core → bingo-digest, as planned; the
  dependency discipline check names that pure-library exception. Ember
  allowlists remain unchanged.

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
      see the verification results and deferrals below.

## Non-goals

Signing or publishing plugins; approving a whole repository at once;
approving anything that is not code a repository brings (hooks scripts
in project settings are a later case, named in the guide).

## Risks

- Repositories that rely on their plugin in CI break once: the notice
  names `bingo plugins approve`, and the guide's upgrade note says so.
- Digesting on every start costs bounded I/O; always hash. Mtime and size
  can be preserved across hostile edits and are not approval evidence.

## Verified — 2026-10-15

Sequential gates; command/output summaries (no live or credential tests):

```text
cargo fmt --all -- --check → exit 0
cargo check --workspace --all-targets --locked → Finished dev profile; exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings → exit 0
cargo test --workspace --locked -- --test-threads=4 → 5507 passed; 0 failed; 5 ignored (163 summaries)
bash scripts/check_discipline.sh → discipline ok (advisory warnings remain)
bash scripts/budget.sh → 411/411 dependencies; warm core 1s; isolation 0; budget ok
cargo deny check → advisories ok, bans ok, licenses ok, sources ok
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-plugin-rpc -p bingo-surface-rpc --lib → 131 + 16 passed
```

Default-thread runs intermittently failed the existing mentions resume assertion
and gateway lock probe. Full serial and four-thread runs passed. Use a harness
`--test-threads` argument: exporting `RUST_TEST_THREADS=1` breaks child readiness framing.
The budget's 13 GB target-size warning is soft; final cleanup removes build outputs.

`bash scripts/tui-smoke.sh` → `tui-smoke ok` before the TLS patch; final reruns exit 1:
`timed out waiting for: ? for shortcuts` (last Escape case), then `Name: Ada` (counter input). This gate remains flaky and unticked.
`cargo check -p EACH --all-targets --target x86_64-pc-windows-msvc --locked` → 10 pass, 10 C-toolchain deferrals across all 20 touched crates.
Pass (`bingo-` prefix): core, demo-ui, experience, hooks-shell, persona, plugin-host, provider-acp, schedule, sdk, surface-rpc.
Deferred: bingo; bingo-{channels,mcp,mods,permissions,plugin-rpc,provider-anthropic,provider-openai,surface-tui,tool-web}.
Blockers: aws-lc-sys cannot find `stdlib.h`/`windows.h`; permissions' tree-sitter/tree-sitter-bash cannot find `stdio.h`/`stdlib.h`. The configured cross-toolchain lacks Windows C headers; no Rust source diagnostic was reported.
