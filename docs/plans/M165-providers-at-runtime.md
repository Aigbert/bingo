# M165 — Providers at runtime

ADR-0085 §§5–7.

## Goal

Add, re-address, check and remove named providers without restarting. Sessions
keep their resolved endpoint; credentials never enter errors or recorded input.

## Bricks

1. **Shape contract.** SDK `ProviderShape` contributes `id`, `settings_key`,
   `build(name, InstanceSpec)`, `takes_key`, and `credential_exists`.
   `InstanceSpec` contains only `base_url` and `images`. Duplicate shapes fail.
2. **Plugin constructors.** OpenAI, Anthropic and Codex register shapes; boot
   and runtime share constructor primitives. `readdress` retains credentials
   only within an origin. `credential_origin` and `retire` describe that boundary.
3. **Live registry.** Runtime replacements/tombstones overlay gathered providers.
   Existing sessions retain their Arc. Registered shapes/providers reserve names.
4. **Credential source.** Additive `AuthStatus::Ready { from }` reports only
   `store`, `setting`, or `env`; fixtures retain the old ready spelling.
5. **Reach.** Live `probe_models` records unchecked/ok/down and publishes
   Providers changes; successful lists update Models. `/models refresh [id]`
   narrows the check. No credentials means no probe.
6. **Provider catalogue.** Meta adds shape, sanitized address, instance and reach;
   fixtures pin the keys and generators own schema/TypeScript.
7. **Runtime verbs.** `/provider add <name> <shape> <url>` validates namespace
   and credential vacancy, persists the instance, then prompts for a key before
   probing. `set <name> url <url>` changes the address and checks after any
   required paste. `remove <name>` clears instance, default and pins together,
   retires resolved endpoints and deletes the credential. Outcomes name changes.
8. **CLI writer.** `bingo provider add` shares validation and persistence; it
   also refuses occupied credential names. Credentials go only to auth.json.

## Decisions

- Brick 2 shares constructor primitives rather than retaining boot settings in
  shapes: the public spec cannot carry configured keys or Codex issuers, and
  retaining settings could revive a credential after remove/add.
- Review changes brick 7's original probe-before-login order: names are strictly
  `[A-Za-z0-9_-]+`, and any stored entry blocks add with "remove it first".
  Shapes check the store through the SDK contract; the kernel imports no auth plugin.
- Reserved names come from registered providers and shapes, not vendor literals
  in the kernel. The CLI receives its registered-name set from composition.
- Review replaces credential-preserving readdress across origins: scheme/host/port
  changes clear stored and user-layer configured keys and ask for Login Paste.
  Same-origin paths retain them; no keep-key option exists. Validate user ownership,
  retire old endpoints and clear credentials before persisting a changed address,
  so failed deletion cannot expose a key after restart. A failed write can require
  signing in again. Old sessions and OAuth retries cannot read a new origin's key.
- Review changes Arc-only refresh identity: source providers match by ID; runtime
  entries additionally require their installed Arc, rejecting stale replacement
  or removal probes while accepting plugin-rpc's freshly allocated adapters.
- Removal clears the user default and matching `name/` pins in the same settings
  write and names what was cleared. The live pin list is cleared under its writer
  lock so subsequent pin edits cannot restore removed pins. Retiring the boot default
  is remembered independently of live entries, so re-adding a name cannot revive it.
- Review restores ADR-0012's Codex fallback for session model selection. Live
  `probe_models` remains fallible so a failed catalogue still records down reach.
- Removal intentionally stops its live sessions at their next request with
  AuthRequired naming the removal. This includes configured keys and same-origin
  clones; choosing another provider continues the session. Guide states this limit.
- Login answers are delivered privately and published as `[redacted]`. Review
  confirmed TUI history and queued command previews also preceded validation;
  every recorded `/provider` argument is now redacted, including rejected userinfo
  URLs and extra key arguments. Dispatch alone keeps the original command.
- Reach errors expose categories, never endpoint text; addresses omit userinfo,
  query and fragment. No new external dependencies or allowlist changes.
- ProviderState groups registry facts; free host functions respect cohesion limits.
  Command/mutation locks serialize edits. Project-only leaves must be edited there.
- CredentialStore uses a process-wide lock so independent handles cannot lose
  concurrent writes. OAuth logout clears local cache/store before best-effort revoke.
- Guides live in the owning crate's registered Page (ADR-0054), not docs/guide/.
  Tests use local fakes/wiremock with provider credential/URL environment removed.

## Files

- SDK provider/plugin/command contracts; core host, commands, settings/instances,
  session input recording and turn/endpoint; provider-openai and provider-anthropic.
- Auth library key/source/store/lifetime; bin provider writer and RPC/CLI tests.
- TUI guide/history; existing AuthStatus consumers; schema/ and sdk/typescript/.

## Exit criteria

- [x] Shape contract, dynamic reserved names and boot instance compatibility.
- [x] Wiremock RPC add → paste → catalogue → same-session model answer; no pre-paste HTTP.
- [x] Occupied credential namespace refusal; strict names; no foreign entry deletion.
- [x] Origin change requires a new paste; same-origin paths retain credentials.
- [x] Fresh plugin Arcs publish lists/reach; stale runtime probes are discarded.
- [x] Codex fallback survives failed catalogue while live probes report failure.
- [x] Remove clears default/pins and returns a named AuthRequired in live sessions.
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
