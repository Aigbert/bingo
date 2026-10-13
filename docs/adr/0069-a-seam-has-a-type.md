# ADR-0069 — A seam has a type

Status: accepted · 2026-10-13 · Plan: M119 (§1–4).
§5–6 remain proposed · Plan: M126.
Amends: ADR-0001 §5, ADR-0031 §2, §5, ADR-0054 §1, ADR-0057
(Consequences)

## Context

Plugin isolation holds at the crate line, so the coupling between plugins
moved into strings the compiler cannot see:

- A service is a `HashMap<String, Service>` whose value is an
  `Arc<dyn Any>` (`bingo-sdk/src/host.rs`, `HostHandle::service(&str)`).
  A key or type mismatch is a runtime `None`. Both services two plugins
  share today are spelled twice: `agents.team` and its method `section`
  in `bingo-agents/src/team/service.rs` and again in
  `bingo-rooms/src/team.rs`; `mcp.servers` and `servers` in
  `bingo-mcp/src/rows.rs` and again in `bingo-provider-acp/src/crossing.rs`.
  The JSON between them costs rooms an error path (`TeamError::Unnameable`)
  for a path that is not UTF-8.
- Neither consumer declares what it uses: both lookups are fail-soft and
  both manifests say `requires: &[]`. `requires` cannot say it, because
  `standing()` (`bingo-core/src/host/registry.rs`) switches off a plugin
  whose requirement is unmet.
- `PluginManifest.sdk` is documented as checked at boot and read nowhere.
- `SurfaceOptions.args` is a JSON object whose eight keys are written in
  `crates/bingo/src/main.rs` (`surface_options`) and read back by name in
  print, rpc and tui. The bin re-spells plugin schemas: `bingo/src/mcp.rs`
  writes `"bingo.mcp"`/`"mcpServers"`, `provider.rs` writes the providers'
  instance keys and `baseUrl`.
- The bin resolves settings by its own rules (the highest layer naming a
  key wins in `update_wanted`, `tui_measure`, `demo_ui`; `.any()` in
  `channels_wanted`) while the kernel deep-merges (ADR-0003 §4). A plugin
  is switched four ways: `enabledPlugins`, `demoUi`, `experience.enabled`,
  `BINGO_FAKE_SCRIPT`.

No kernel door is opened. `ServiceKey<T>` types the lookup that exists,
`uses` is a manifest field the registry does not act on, and the rest
deletes spellings. Each item removes a second representation.

## Decision

1. **A key carries its type.** `bingo_sdk::ServiceKey<T: ?Sized>` is a
   name plus `PhantomData<fn() -> Arc<T>>`: `const fn new(&'static str)`
   for one service, `ServiceKey::named(String)` for a family whose name is
   computed (`Pages::key(id) -> ServiceKey<Pages>`). Lookup is
   `HostHandle::service(&KEY) -> Option<Arc<T>>`, and `ToolContext::service`
   matches it. Registration is `Contribution::service(&KEY, Arc<T>, wire)`,
   which stores the `Arc<T>` itself in the `Any`, so a trait object can be
   met. The registry refuses a service whose key the manifest does not
   list under `provides`. The `&str` lookup goes. `service_any`,
   `service_wire` and `open_service` stay string-keyed for plugin-rpc's
   router alone (ADR-0031 §4).
2. **Shared contracts live in `bingo-services`**, a library-tier crate
   (ADR-0012 §1: sdk and other libraries only) with one module per
   service: key, typed trait, and the two mechanical adapters ADR-0031 §6
   names. `team::TeamFile` (`agents.team`, `section(&Path, &str)`) and
   `mcp::Servers` (`mcp.servers`, `servers()`) move there first. A
   consumer's `find(host)` tries the typed face, then wraps the wire face,
   so an owner that moved out of process is still found. This is the crate
   ADR-0001 §5 kept for a third consumer. It is made at the second because
   the second already re-spelled the key. It is one crate, not one per
   service as ADR-0031 §2 had it, because two traits do not pay for two
   crates. `Pages` stays in the sdk, because every plugin writes pages and
   `bingo-skills` meets each one by its family key (ADR-0054).
3. **A manifest says what it uses.** `PluginManifest` gains
   `uses: &'static [&'static str]`, service keys a plugin looks up and can
   do without, spelled from the constant (`team::KEY.name()`). `requires`
   keeps ADR-0031 §5's meaning: missing switches the plugin off. The
   registry ignores `uses` when deciding who stands. `bingo plugins`
   names a used service that nobody provides. `check_discipline.sh`
   fails when a crate looks up a `bingo_services` key that its manifest
   does not list under `provides`, `requires` or `uses`. It reads the keys
   from `bingo-services`, not from a list of its own. External manifests
   gain `uses` in protocol 2 (ADR-0075).
4. **`PluginManifest.sdk` is deleted.** Any version agreement belongs to
   protocol 2's handshake (ADR-0075), not to a field nobody reads.
5. **A surface's options are its own type.** `SurfaceOptions.args` is
   removed. `SurfaceOptions` keeps what every surface is asked: `cwd`,
   `selector`, `prompt`, `env`. Each surface plugin takes its own
   `Options` at construction (`TuiPlugin::new`, `PrintPlugin::new`,
   `RpcPlugin::new`), and each `Options` is `Default` for compositions
   that run no surface (`provider_ids`, `bingo plugins`). The bin imports
   a plugin's key and shape from the plugin crate instead of re-spelling
   them.
6. **A setting resolves one way.** A plugin reads the keys it claims from
   its own slice at `register` (ADR-0003 §4). Before a host exists, the
   bin reads only what decides the composition: which plugins are
   composed, and whether a chat surface starts. It reads these through
   `bingo_core::settings::merge(layers, claims)` with
   `Claim::from_manifest` over the static manifests. Claims exist before
   any host, so the hand-written layer walks go. A kernel key a surface
   needs (`pictures.cacheDays`) reaches it in `Options` from
   `KernelSettings`. A plugin is switched by one key, `enabledPlugins`
   (ADR-0057). `demoUi` and `experience.enabled` are read for one release
   by the plugin that claimed them, with a notice naming the new spelling,
   then become unknown keys. No file is rewritten. The demo is composed
   only on an explicit `true` or `--demo-ui`, because "absent is on" is
   for plugins a build composes. `BINGO_FAKE_SCRIPT` stays an environment
   variable: it selects a harness, not a preference.

## Consequences

- A key or type mismatch is a compile error. A contract between two
  plugins has one spelling, in a crate both depend on, and rooms loses
  the error path that a JSON string bought.
- About 25 manifests lose `sdk` and gain `uses: &[]`, a mechanical edit.
  Rooms and the ACP provider declare the service they read.
- A higher settings layer can turn channels off, which `.any()` could not
  do. A setting the bin reads and one the kernel reads cannot disagree.
- Surface option keys stop being JSON. A typo in one is a compile error,
  not a silently ignored member.
- Plugin-rpc keeps string keys: across a process, key, method and schema
  remain the contract (ADR-0031 §1).

Refs: ADR-0001, ADR-0003, ADR-0012, ADR-0031, ADR-0054, ADR-0057,
ADR-0075; Plans: M119, M122, M126; audit-2026-10 §2, §Also
