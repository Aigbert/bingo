# M165 — Providers at runtime

ADR-0085 §§5–7.

## Goal

Add, re-address, check and remove named providers without restarting. Sessions keep their resolved endpoint; credentials never enter errors or recorded input.

## Bricks

1. SDK `ProviderShape`: id, settings key, public spec construction and credential operations.
2. Shared plugin constructors; `credential_origin` and `retire` enforce address changes. Readdress receives fresh settings provenance, not a retained configured key.
3. Runtime replacements/tombstones overlay gathered providers; sessions keep their Arc.
4. Additive `AuthStatus::Ready { from }`: store, setting or env, never the credential.
5. Fallible `probe_models` updates reach and catalogues; no credential means no probe.
6. Provider catalogue: shape, sanitized address, instance and reach; generated wire fixtures.
7. Runtime add → persist → paste → probe; set and remove serialize settings/live edits.
8. CLI add shares validation/persistence; keys go only to auth.json.
9. HTTP redirect policy defaults to same-origin, with bounded credential-free opt-ins.
10. Refresh replaces or expires only the stored credential snapshot it started with.

## Decisions

- Constructors share primitives rather than retain boot settings in shapes: otherwise removed configured keys could revive on add. The public spec contains no credentials.
- Review changes add's probe-before-login order: no refresh before a new key is pasted. Existing credential entries block add and name `/provider remove <name>` as recovery.
- Round 2 moves the nonempty `[A-Za-z0-9_-]+` rule to SDK, shared by core and boot claims. Invalid names are skipped before construction, with notices that cannot echo secrets.
- Round 3 binds stored keys to canonical scheme/host/port, and OAuth to issuer plus resource origins. Paste, wizard, login and refresh stamp metadata; every request checks it, including cached tokens. Mismatches keep providers built but signed out and name the resolved origin and override file. No repository baseUrl approval is needed.
- Legacy named entries gain missing metadata only from exactly one user-layer family declaration, using that layer's addresses before merging. Project/local never adopt. Default legacy entries use user-layer addresses or the shipped defaults; never overrides. Existing metadata is retained; reserved names cannot adopt another family's legacy key. Explicit legacy API-key and Codex regressions remain.
- Reserved names include registered providers/shapes and all shipped families, switched on or off, supplied by binary config. Boot collisions skip instances with notices; late providers take precedence. Notices name the instance, winning plugin/family and declaration file to rename/delete. The kernel contains no vendor name list.
- Scheme/host/port changes clear stored and user-layer keys and request Login Paste; same-origin paths retain them. No keep-key option. Any non-user layer holding apiKey blocks an origin change and names its file, including keys shadowed by higher layers.
- Validate user ownership, delete the credential, persist the address, then retire the old endpoint. Failed writes leave it unretired but can require signing in again. Old sessions and OAuth retries cannot read the new origin's key.
- Source refresh identity uses provider ID; runtime overlays additionally require their installed Arc, rejecting stale results while accepting plugin-rpc's fresh adapters.
- Removal clears the saved provider/model default and matching pins in one write, and names the cleared fields. Live pins clear under their writer lock. Re-adding a name cannot revive the boot default. Orphan removal also checks the owner family's declarations in every layer, including disabled families, and compares ownership again at deletion.
- Removal stops live sessions at their next request with AuthRequired naming removal, including configured keys and same-origin clones. The guide states this limitation.
- Codex session selection retains ADR-0012's fallback; live probes still report failure.
- Registration receives provenance only for its claimed settings, plus user-instance counts for unambiguous adoption. Unreadable auth.json produces one notice and supplies no keys; invalid names and foreign owners name the skipped declaration file.
- Login answers are private. Journal, queue previews and TUI history redact every `/provider` argument before validation. Reach exposes categories; addresses strip secrets.
- ProviderState groups registry facts; mutation/command locks serialize edits. Project-only instances must be edited there. CredentialStore's shared lock prevents lost writes.
- Round 4 binds configured keys too: environment keys use the default or their BASE_URL variable; settings keys use addresses from their own or more trusted layers. One request-time guard also applies after readdressing. Round 5 drops directly embedded keys on every readdress because they have no settings source to re-read. SDK provenance is an origin enum, so user/repository/explicit cannot contradict each other.
- Boot, migration and provider construction share address resolution, including environment URLs and blank-as-unset. Invalid origins retain a refusal, never a binding or credential write. Canonical tests cover case, userinfo, IPv6 and trailing dots (distinct, failing closed).
- Repository-controlled sign-in first asks for destination confirmation showing canonical origin and source file, before paste, browser or device flows. Cancellation writes nothing and sends no HTTP. Same-origin rebinding retains provenance; a changed origin gets its new source. Reserved-name notices use the declaring manifest even when disabled; composition-only reservations name the binary.
- Round 5 fails closed on HTTP redirects: scheme, host and effective port must match the initial request, with the existing ten-hop bound. Only credential-free page fetching, image loading and public release downloads opt into cross-origin redirects; Brave search keeps a separate same-origin client. Same-origin OAuth 307 bodies still replay.
- Every readdress discards the configured key and resolves current settings through the existing trust/origin policy. The settings writer returns the document it just persisted, replacing the boot user-layer snapshot without a second read race; only the owning family's values cross the SDK boundary. A→B→A cannot revive a removed key, and same-origin edits pick up newly configured keys.
- OAuth refresh compares the complete stored entry (credential, owner, issuer origin and resource origin) under the shared lock before replacement or expiry deletion. A late response cannot overwrite or remove a newer sign-in. This is process-local compare-and-set, consistent with the store's existing concurrency boundary. A CAS mismatch clears the cache and re-reads the replacement: only a matching binding with a nonempty, unexpired credential may serve the waiting request. An expiry persistence error instead logs a warning and retains Cached::Expired, so repairing the store cannot retry the retired refresh token.
- Follow-ups: replacing `default_cleared` with mutable settings affects selection and approval consumers (>40 lines). SDK argument secrecy also needs declaration access in history/queue readers plus constructors/wire fixtures (>40 lines). Defer both optional cleanups; retain tested redaction and cleared-default behavior.
- Guides use the owning crate's Page (ADR-0054), never docs/guide/. No dependencies or allowlist changes. HTTP tests use local fakes/wiremock with provider environment unset.

## Files

- SDK provider/plugin/command contracts; core host, commands, settings/instances, session input recording and turn/endpoint; provider-openai and provider-anthropic.
- HTTP client/redirect policy and credential-free consumer opt-ins; auth library key/source/store/lifetime; bin provider writer and RPC/CLI tests; plugin-rpc provider attribution and its fixture.
- TUI guide/history; existing AuthStatus consumers; schema/ and sdk/typescript/.

## Exit criteria

- [x] Shape contract, shipped reserved names and safe boot instances.
- [x] Wiremock RPC add → paste → catalogue → same-session model answer; no pre-paste HTTP.
- [x] Strict names; user-only legacy adoption; no foreign-key adoption/deletion; safe orphan recovery.
- [x] Stored and configured origins checked at boot/request/refresh; origin changes require a new paste and repository sign-ins show the destination.
- [x] Cross-origin redirects are refused; removed configured keys cannot revive; late refresh writes cannot replace new credentials or bindings.
- [x] Fresh plugin Arcs publish lists/reach; stale probes are discarded; collisions name plugin/file.
- [x] Codex fallback survives failed catalogue while live probes report failure.
- [x] Remove clears provider/model default and pins; live sessions get named AuthRequired.
- [x] Failed reach, invalid URL and key arguments never disclose secrets in recordings.
- [x] Native gates pass; earlier isolated TUI smoke is recorded below.
- [x] Earlier Windows SDK, core, agents and RPC checks passed (below).
- [ ] Windows auth, both providers, TUI and bin: missing C headers/toolchain (below).

## Non-goals

Picker UI (M166); removing built-ins; editing images in chat; shape inference; OAuth for new issuers; retaining credentials across origins.

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

## Verified — 2026-10-15 (review round 3)

Regressions failed before fixes: project overrides (named/default keys and OAuth issuer), legacy adoption, foreign-owner notices, disabled-family orphan removal, unreadable auth.json, bad-name provenance and plugin attribution.
Additional regressions cover OAuth resource overrides, cached-token checks, reserved-name migration and URL-safe notices.
Sequential native gates with provider key/URL environment unset and `RUST_TEST_THREADS=4`:

```text
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk -p bingo-surface-rpc -p bingo-plugin-rpc -p bingo-mods -p bingo-store-jsonl --locked => 603 passed, 3 ignored; schema/TS unchanged
cargo fmt --all -- --check => exit 0
cargo check --workspace --all-targets --locked => exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings => exit 0
cargo test --workspace --locked => 5619 passed, 5 ignored, 167 groups
bash scripts/check_discipline.sh => discipline ok (advisory warnings)
bash scripts/budget.sh => dependencies 412; warm core 0s; isolation 0; target 18 GB; budget ok
cargo deny --offline check => advisories ok, bans ok, licenses ok, sources ok
```

CLI: 279 passed; runtime-provider RPC: 5 passed. No wall-clock flake; HTTP fixtures were local.
Direct-construction fixtures now carry bindings; explicit legacy API-key and Codex boot tests remain.
Native-only review: TUI smoke and Windows checks were not rerun; earlier records and C-toolchain deferrals stand.
Final `cargo clean` follows this verification commit to remove the build-directory soft warning.

## Verified — 2026-10-15 (review round 4)

Regressions reproduced configured-key redirects, environment-origin migration,
blank addresses, invalid bindings, reserved-plugin attribution, missing sign-in
context and embedded-key readdressing. Additional tests pin Anthropic stored-key
overrides, canonical origins and same-layer configured credentials. RPC proves
OpenAI, Anthropic and Codex show the destination before accepting a credential;
cancellation sends no HTTP and writes no key.
Sequential native gates with provider key/URL environment unset,
`RUST_TEST_THREADS=4` and Cargo offline:

```text
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk -p bingo-surface-rpc -p bingo-plugin-rpc -p bingo-mods -p bingo-store-jsonl --locked => 608 passed, 3 ignored; schema/TS unchanged
cargo fmt --all -- --check => exit 0
cargo check --workspace --all-targets --locked => exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings => exit 0
cargo test --workspace --locked => 5684 passed, 5 ignored, 169 groups
bash scripts/check_discipline.sh => discipline ok (advisory warnings)
bash scripts/budget.sh => dependencies 413; warm core 0s; isolation 0; target 22 GB; 322 test binaries; budget ok
cargo deny --offline check => advisories ok, bans ok, licenses ok, sources ok
```

CLI: 285 passed; runtime-provider RPC: 6 passed. All HTTP fixtures were local.
Initial runs exposed an old stderr-order assertion, a direct-registration fixture
without provenance, and a JSON fixture whose filename migrated to TOML; corrected
before the final run. No wall-clock flake occurred. No dependencies were added.
Native-only review: prior Windows/toolchain and TUI-smoke records remain unchanged.
Final `cargo clean` follows this verification commit to remove the build-directory
soft warning. Historical verification is retained despite the plan-length advisory.

## Verified — 2026-10-15 (review round 5)

Cross-origin refresh-token and provider redirect regressions failed before the
policy fix; A→B→A configured-key revival and the late-refresh overwrite also
failed before their fixes. Wiremock covers 307/308 token bodies, Anthropic and
OpenAI redirects, same-origin 307 replay, credential-free page redirects and
the ten-hop limit. RPC covers both families after the user apiKey is removed,
and a same-origin edit reading a freshly changed key. Late success and expired
refresh replies both preserve the replacement resource credential. Anthropic
BASE_URL legacy adoption and HTTP default port 80 have explicit coverage.

All commands ran sequentially, with ambient OPENAI_* / ANTHROPIC_* removed,
`RUST_TEST_THREADS=4` and Cargo offline. Tests set only synthetic credentials
and local endpoint overrides; no real credentials or provider network calls.

```text
BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk -p bingo-surface-rpc -p bingo-plugin-rpc -p bingo-mods -p bingo-store-jsonl --locked => 608 passed, 3 ignored; schema/TS unchanged
cargo fmt --all -- --check => exit 0
cargo check --workspace --all-targets --locked => exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings => exit 0
cargo test --workspace --locked => 5697 passed, 5 ignored, 169 groups
bash scripts/check_discipline.sh => discipline ok (advisory warnings)
bash scripts/budget.sh => dependencies 413; warm core 0s; isolation 0; target 23 GB; 328 test binaries; budget ok
cargo deny --offline check => advisories ok, bans ok, licenses ok, sources ok
```

CLI: 286 passed; runtime-provider RPC: 8 passed. No wall-clock flake occurred.
The first discipline run refused a fourth CredentialStore implementation file;
its refresh persistence methods now live in store.rs. Check, Clippy, the full
workspace tests and discipline passed again after that structural correction.
No dependencies or allowlists changed. Native-only review: earlier Windows and
TUI-smoke records stand. Final `cargo clean` follows this verification commit.

## Verified — 2026-10-15 (image redirect follow-up)

Image loading also opts into bounded cross-origin redirects: it attaches no
credentials. Grep for request-header setters, authorization, cookies and API-key
headers in load.rs found no matches. A two-server wiremock 302 regression failed
with `cross-origin redirect refused` before the fix, then followed and decoded
the expected PNG after it. Provider environment was unset; fixtures were local.

```text
RUST_TEST_THREADS=4 cargo test -p bingo-pictures --locked => 69 passed, 0 failed
cargo fmt --all -- --check => exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings => exit 0
bash scripts/check_discipline.sh => discipline ok (advisory warnings)
```

Final `cargo clean` follows this commit.

## Verified — 2026-10-15 (refresh recovery follow-up)

Both regressions failed before their fixes. A late success or expired reply now
serves a usable same-binding replacement, including a newly pasted key or an
unexpired token inside the proactive refresh window. Removed, empty, expired,
unknown-expiry and different-resource replacements remain refused. Portable
read/write I/O failures during expiry retain Cached::Expired; after the store
is repaired, subsequent normal and forced refresh calls send no HTTP retry.

Sequential checks with OPENAI_* / ANTHROPIC_* unset, `RUST_TEST_THREADS=4`,
Cargo offline, synthetic credentials and wiremock only:

```text
cargo test -p bingo-auth-oauth -p bingo-provider-openai -p bingo-provider-anthropic --locked => 313 passed, 0 failed, 0 ignored
cargo fmt --all -- --check => exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings => exit 0
bash scripts/check_discipline.sh => discipline ok (advisory warnings)
```

Clippy's initial boolean-simplification finding was corrected; tests and checks
passed again. No timing flake or dependency change. Final `cargo clean` follows
this commit.
