# M165 — Providers at runtime

ADR-0085 §§5–7.

## Goal

Add, re-address, check and remove named providers without restarting. Sessions
keep their resolved endpoint; credentials never enter errors or recorded input.

## Bricks

1. SDK `ProviderShape`: id, settings key, public spec construction and credential operations.
2. Shared plugin constructors; `credential_origin` and `retire` enforce address changes.
3. Runtime replacements/tombstones overlay gathered providers; sessions keep their Arc.
4. Additive `AuthStatus::Ready { from }`: store, setting or env, never the credential.
5. Fallible `probe_models` updates reach and catalogues; no credential means no probe.
6. Provider catalogue: shape, sanitized address, instance and reach; generated wire fixtures.
7. Runtime add → persist → paste → probe; set and remove serialize settings/live edits.
8. CLI add shares validation/persistence; keys go only to auth.json.

## Decisions

- Constructors share primitives rather than retain boot settings in shapes: otherwise
  removed configured keys could revive on add. The public spec contains no credentials.
- Review changes add's probe-before-login order: no refresh before a new key is pasted.
  Existing credential entries block add and name `/provider remove <name>` as recovery.
- Round 2 moves the nonempty `[A-Za-z0-9_-]+` rule to SDK, shared by core and boot claims.
  Invalid names are skipped before construction, with notices that cannot echo secrets.
- Round 3 binds stored keys to canonical scheme/host/port, and OAuth to issuer plus
  resource origins. Paste, wizard, login and refresh stamp metadata; every request checks
  it, including cached tokens. Mismatches keep providers built but signed out and name
  the resolved origin and override file. No repository baseUrl approval is needed.
- Legacy named entries gain missing metadata only from exactly one user-layer family
  declaration, using that layer's addresses before merging. Project/local never adopt.
  Default legacy entries use user-layer addresses or the shipped defaults; never overrides.
  Existing metadata is retained. Explicit legacy API-key and Codex regressions remain.
- Reserved names include registered providers/shapes and all shipped families, switched
  on or off, supplied by binary config. Boot collisions skip instances with notices;
  late providers take precedence. Notices name the instance, winning plugin/family and
  declaration file to rename/delete. The kernel contains no vendor name list.
- Scheme/host/port changes clear stored and user-layer keys and request Login Paste;
  same-origin paths retain them. No keep-key option. Any non-user layer holding apiKey
  blocks an origin change and names its file, including keys shadowed by higher layers.
- Validate user ownership, delete the credential, persist the address, then retire the
  old endpoint. Failed writes leave it unretired but can require signing in again.
  Old sessions and OAuth retries cannot read the new origin's key.
- Source refresh identity uses provider ID; runtime overlays additionally require their
  installed Arc, rejecting stale results while accepting plugin-rpc's fresh adapters.
- Removal clears the saved provider/model default and matching pins in one write, and
  names the cleared fields. Live pins clear under their writer lock. Re-adding a name
  cannot revive the boot default. Orphan removal also checks the owner family's declarations
  in every layer, including disabled families, and compares ownership again at deletion.
- Removal stops live sessions at their next request with AuthRequired naming removal,
  including configured keys and same-origin clones. The guide states this limitation.
- Codex session selection retains ADR-0012's fallback; live probes still report failure.
- Registration receives provenance only for its claimed settings, plus user-instance
  counts for unambiguous adoption. Unreadable auth.json produces one notice and supplies
  no keys; invalid names and foreign owners name the skipped declaration file.
- Login answers are private. Journal, queue previews and TUI history redact every
  `/provider` argument before validation. Reach exposes categories; addresses strip secrets.
- ProviderState groups registry facts; mutation/command locks serialize edits. Project-only
  instances must be edited there. CredentialStore's shared lock prevents lost writes.
- Follow-ups: replacing `default_cleared` with mutable settings affects selection and
  approval consumers (>40 lines). SDK argument secrecy also needs declaration access
  in history/queue readers plus constructors/wire fixtures (>40 lines). Defer both
  optional cleanups; retain tested redaction and cleared-default behavior.
- Guides use the owning crate's Page (ADR-0054), never docs/guide/. No dependencies or
  allowlist changes. HTTP tests use local fakes/wiremock with provider environment unset.

## Files

- SDK provider/plugin/command contracts; core host, commands, settings/instances,
  session input recording and turn/endpoint; provider-openai and provider-anthropic.
- Auth library key/source/store/lifetime; bin provider writer and RPC/CLI tests.
- TUI guide/history; existing AuthStatus consumers; schema/ and sdk/typescript/.

## Exit criteria

- [x] Shape contract, shipped reserved names and safe boot instances.
- [x] Wiremock RPC add → paste → catalogue → same-session model answer; no pre-paste HTTP.
- [x] Strict names; no implicit foreign-credential adoption/deletion; orphan recovery.
- [x] Origin change requires a new paste; same-origin paths retain credentials.
- [x] Fresh plugin Arcs publish lists/reach; stale runtime probes are discarded.
- [x] Codex fallback survives failed catalogue while live probes report failure.
- [x] Remove clears provider/model default and pins; live sessions get named AuthRequired.
- [x] Failed reach, invalid URL and key arguments never disclose secrets in recordings.
- [x] Native gates and isolated TUI smoke pass after review.
- [x] Windows SDK, core, agents and RPC checks pass.
- [ ] Windows auth, both providers, TUI and bin: missing C headers/toolchain (below).

## Non-goals

Picker UI (M166); removing built-ins; editing images in chat; shape inference;
OAuth for new issuers; retaining credentials across origins.

## Risks

- Settings edits are process-local, not an inter-process transaction.
- Disabled runtime plugin shapes leave persisted instances for the next enable.
- Removed or re-addressed origins cannot continue existing requests indefinitely;
  the next request requires choosing a current endpoint.

## Earlier verification

Before review/rebase, native gates and smoke passed (5542 tests, 5 ignored).
An intermittent `mentions` continuation failure was recorded; no unrelated fix
was retained. Windows SDK/core/agents/RPC passed; C-dependent crates were deferred.
M164's Verified record reports the known smoke counter failure (`Name: Ada`).

## Verified — 2026-10-15

Review regressions failed before fixes: names, reserved names, fresh source Arcs,
removed default/pins, cross-origin credentials, Codex fallback, removal message,
and both queue/history recordings. All HTTP fixtures are local; no live credentials.
Sequential gates, with provider credential/base-URL environment removed:

```text
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk -p bingo-surface-rpc -p bingo-plugin-rpc -p bingo-mods -p bingo-store-jsonl --locked => 595 passed, 3 ignored
cargo fmt --all -- --check => exit 0
cargo check --workspace --all-targets --locked => exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings => exit 0
cargo test --workspace --locked => 5555 passed; 5 ignored; 167 groups
bash scripts/check_discipline.sh => discipline ok (advisory warnings)
bash scripts/budget.sh => dependencies 412; warm core 0s; isolation 0; target 11 GB; budget ok
cargo deny check => advisories ok, bans ok, licenses ok, sources ok
bash scripts/tui-smoke.sh => tui-smoke ok; both counter sizes passed
cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc:
  bingo-sdk, bingo-core, bingo-agents, bingo-surface-rpc => exit 0
  bingo-auth-oauth, bingo-provider-openai, bingo-provider-anthropic, bingo-surface-tui => exit 101: aws-lc-sys
  bingo => exit 101: aws-lc-sys and rquickjs-sys; stdlib.h/windows.h and Windows C toolchain absent
```
Generators left schema/ and sdk/typescript/ unchanged; final workspace tests confirm no drift.
Initial gates exposed an old CLI wording assertion and an unbounded test client;
both were corrected before the final sequential rerun. No wall-clock flake occurred.
The known smoke counter failure did not reproduce; its implementation is unchanged.
The 11 GB soft warning is addressed by final `cargo clean` after this verification commit.

## Verified — 2026-10-15 (review round 2)

Regressions failed before fixes: boot names/owners, model removal, collisions,
failed origin writes, layered keys, orphan removal and secret set-name errors. Sequential gates:

```text
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk -p bingo-surface-rpc -p bingo-plugin-rpc -p bingo-mods -p bingo-store-jsonl --locked => 603 passed, 3 ignored; schema/TS unchanged
cargo fmt --all -- --check => exit 0
cargo check --workspace --all-targets --locked => exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings => exit 0
RUST_TEST_THREADS=4 cargo test --workspace --locked => 5596 passed, 5 ignored, 167 groups
bash scripts/check_discipline.sh => discipline ok (advisory warnings)
bash scripts/budget.sh => dependencies 412; warm core 1s; isolation 0; target 16 GB; budget ok
cargo deny check => advisories ok, bans ok, licenses ok, sources ok
cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc:
  bingo-sdk, bingo-core => exit 0
  auth, both providers, TUI, bin => exit 101: known aws-lc-sys C headers; bin also rquickjs-sys
```
Provider credential/URL environment was unset; all HTTP fixtures were local.
CLI: 266 passed; runtime-provider RPC: 4 passed, including a turn after removal.
Initial clippy needless-borrow and two test-fixture mistakes were corrected; no wall-clock flake.
TUI smoke was not rerun (guide-only surface change); prior counter bug record stands.
Final `cargo clean` follows this commit to remove the 16 GB build-directory soft warning.
