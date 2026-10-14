# M165 — Providers at runtime

ADR-0085 §§5–7.

## Goal

A person adds, re-addresses, checks and removes a provider instance in
a running session, sees whether each provider is reachable and where
its key came from, and never restarts.

## Bricks

1. **`ProviderShape` contribution.** `bingo-sdk`: trait
   `ProviderShape { id(), settings_key(), build(name, &InstanceSpec) ->
   Result<Arc<dyn Provider>, PluginError> }`, `InstanceSpec { base_url,
   images }`, `Contribution::ProviderShape`. Registry holds shapes by
   id; a duplicate shape id is a build error. Contract test with a fake
   shape.
2. **Plugins build through their shape.** `bingo-provider-openai`
   (`openai`, `codex`) and `bingo-provider-anthropic` (`anthropic`)
   register shapes. Boot and runtime builds share constructor primitives;
   `Provider::readdress` clones the resolved credential with new URL/images.
   Existing instance behaviour is retained (see Decisions).
3. **Live provider set.** `Registry` gains a runtime instance list
   beside the boot list; `Host::providers` overlays it on `ProviderSet::gather`. Adding a name
   that any provider already uses is refused (ADR-0017 §2). Sessions
   keep the provider `Arc` they resolved; removal affects new choices.
4. **Credential source.** `AuthStatus::Ready { from: Option<KeySource> }`
   (`store | setting | env`), serde additive (`{"kind":"ready"}` still
   parses). `ApiKey::status()` keeps `place`; OAuth answers `store`.
   Wire fixture for each.
5. **Reach.** `host/refresh.rs::ask` records per provider
   `Reach::{Ok { ms, at }, Down { error, at }}` in memory (unchecked is
   absence) and publishes `CatalogChanged { Providers }` on change.
   `/models refresh [<id>]` narrows to one provider. Errors are one
   line, no secrets (test with a key in the URL).
6. **Providers meta.** `catalog.rs` providers entries add `shape`,
   `address` (scheme + host + path, no query), `instance`, `reach`;
   key-set test, RPC fixture, schema.
7. **`/provider` command** (`commands/provider.rs`):
   - `add <name> <shape> <url>`: validate, build, list models (reach),
     write `<key>.instances.<name>` via the settings writer (shared with
     `crates/bingo/src/provider.rs::insert`), join the live set; a shape
     with keys then runs `login(Paste)` so the key goes to `auth.json`
     through the existing interaction.
   - `set <name> url <url>`: rewrite the leaf, rebuild, re-check.
   - `remove <name>`: only `instance` providers; drop leaf, store entry
     and live entry.
   - Outcome lines: `+ <name> · <shape> · <n> models · <ms>` and the
     failure reason when the list call fails (the instance is still
     written; `reach` says down).
8. **`bingo provider add`** calls brick 7's writer; no behaviour change.

## Decisions

- Brick 2 uses the same plugin constructor primitives at boot and from a
  shape. The public spec cannot carry configured keys or Codex issuers.
  `Provider::readdress` clones the resolved provider with new endpoint options;
  retaining boot settings in a shape would revive credentials after remove/add.
  Shapes therefore retain only clients/environment, never instance settings.
- Black-box journal checks exposed plaintext login answers in
  `session/interactions.rs`. Deliver the original only to the provider and
  publish `[redacted]` for login text; this necessary fix extends the file scope.
- Reach errors expose error categories, never arbitrary endpoint text, which
  can echo URL or header credentials. Addresses omit userinfo, query and fragment.
- Runtime mutations are serialized; replacements/removals shadow boot entries.
  Refresh results are accepted only for the same provider Arc, so an old probe
  cannot overwrite the replacement's list. Sessions retain their resolved Arc.
- Edits/removals require a user-layer leaf; a project-only instance cannot be
  durably removed by writing a different layer. Refuse and name that limitation.
- Codex list failures now propagate instead of masquerading as a successful
  static fallback. The kernel keeps the last successful list; reach stays honest.
- CredentialStore uses one process-wide lock: independent runtime/plugin
  handles write the same file; a per-handle lock can lose concurrent logins.
- Removal withdraws live choices before logout awaits. OAuth removes its local
  entry/cache before best-effort revocation, so cancelling a slow issuer cannot
  leave a removed instance signed in (`bingo-auth-oauth/src/source.rs`).
- Test subprocesses run with provider credential/base-URL environment removed.
  Existing instance tests also route the default provider to wiremock.
- Documentation uses the owning crate's guide page (ADR-0054), not docs/guide/.

## Files

- `crates/bingo-sdk/src/{provider.rs,plugin.rs}`.
- `crates/bingo-core/src/host/{registry.rs,choose.rs,refresh.rs,
  catalog.rs}`, `commands/{mod.rs,models.rs}`, new `commands/provider.rs`
  and a small `settings/instances.rs` writer.
- `crates/bingo-provider-{openai,anthropic}/src/{instances.rs,lib.rs}`,
  `crates/bingo-auth-oauth/src/key.rs`.
- `crates/bingo/src/provider.rs`; RPC/CLI contract tests; `schema/`,
  `sdk/typescript/`; the existing `guide-tui` page.
- Necessary boundary fixes: `session/interactions.rs` and OAuth `source.rs`/`store.rs`;
  `AuthStatus` consumers in `bingo-agents` and core authentication.

## Exit criteria

- [ ] Shape contract test; boot instances unchanged (existing tests).
- [ ] Black-box (temp HOME, wiremock as the endpoint): `/provider add`
      → the provider is in `catalog(Providers)` with `reach: ok`,
      `/model <name>/<id>` answers in the same session, the settings
      leaf is written with comments preserved, the key is only in
      `auth.json`.
- [ ] Unreachable address: `reach: down` with a redacted error; the
      instance stays and `/provider set … url` fixes it.
- [ ] `remove` refuses a boot provider; removes an instance everywhere.
- [ ] Journal grep in the black-box test finds no key text.
- [ ] Gates as M164, plus Windows check of `bingo-core`.

## Non-goals

Picker UI (M166); removing default providers; editing `images` from the
chat; detecting a shape from a pasted key (the picker's job); OAuth
for new issuers.

## Risks

- Two processes adding the same name: the settings writer is not a
  lock; second writer sees the leaf and refuses on its next read.
- A runtime provider from a plugin that is then switched off
  (ADR-0057): its shape goes with it; the instance stays in settings
  and is skipped with a notice, as at boot.
- Probing a local server (`:11434`) from CI: tests use wiremock only.
