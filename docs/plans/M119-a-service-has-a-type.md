# M119 — A service has a type

## Goal

ADR-0069 §1–4. A service lookup names a typed key, so a key or type
mismatch is a compile error. The two services plugins share today
(`agents.team`, `mcp.servers`) get one spelling each, in a new
library crate, `bingo-services`. A manifest says which services it uses
without being switched off when they are missing. `PluginManifest.sdk`,
read nowhere, goes. No kernel door, no behaviour change a person can see,
no new external dependency.

## Bricks, in build order

1. **Contract tests first** (sdk, `service.rs`). A value registered under
   `ServiceKey<dyn Trait>` is found by the same key and not by a key of
   another type with the same name. A family key
   (`ServiceKey::<Pages>::named`) is found under its computed name. Both
   fail to compile or fail against today's `&str` lookup.
2. **`ServiceKey<T: ?Sized>`**: `const fn new`, `named(String)`,
   `name()`. `HostHandle::service(&ServiceKey<T>)` and
   `ToolContext::service` do the same downcast of an `Arc<T>` stored in
   the `Any`. `Contribution::service(&KEY, Arc<T>, wire)` builds the
   variant. `Pages::key(id)` returns `ServiceKey<Pages>`. The `&str` form
   of `HostHandle::service` is deleted. `service_any`, `service_wire` and
   `open_service` are unchanged.
3. **The registry refuses an undeclared service.** A test registers a
   `Contribution::Service` whose key is missing from `provides` and
   expects a registration error naming both. The per-plugin tests that
   checked the same thing by hand (`agents`, `mcp`, `context`,
   `experience`, `hooks-shell`, `surface-tui`, `channels`) are deleted.
4. **`bingo-services`** (`tier = "library"`, depends on `bingo-sdk`,
   `async-trait`, `serde`, `serde_json`, all already in the tree). Two
   modules, each holding a key, a trait, typed→wire and wire→typed
   adapters, and `find(host)`, which tries the typed face and then the
   wire face. Each module gets golden JSON for its one method, taken from
   today's wire answers so plugin-rpc callers see no change.
   - `team`: `TeamFile::section(&Path, &str) -> Result<Value, TeamError>`.
   - `mcp`: `Servers::servers() -> Map<String, Value>`.
5. **Owners move.** `bingo-agents/src/team/service.rs` implements
   `TeamFile` and registers both faces through the adapters.
   `bingo-mcp/src/rows.rs` does the same for `Servers`.
6. **Consumers move.** `bingo-rooms/src/team.rs` calls
   `team::find(host)` and loses `TEAM`, `SECTION`, the JSON round trip
   and `TeamError::Unnameable`. `bingo-provider-acp/src/crossing.rs`
   calls `mcp::find(host)` and loses `ROWS`/`SERVERS`. Their existing
   tests pass unchanged, and a new test checks each against the other
   side's real owner.
7. **`uses`.** Add `uses: &'static [&'static str]` to `PluginManifest`.
   Rooms says `uses: &[team::KEY.name()]`, the ACP provider
   `uses: &[mcp::KEY.name()]`, every other manifest `uses: &[]`. A
   `standing()` test shows a missing used service leaves the plugin up.
   `bingo plugins` prints a used service nobody provides; its black-box
   test is in `crates/bingo/tests/cli`.
8. **`sdk` deleted** from `PluginManifest` and from every manifest, a
   mechanical edit across 39 files. The doc line that says it is checked
   at boot goes with it.
9. **The check.** `scripts/check_discipline.sh` gains 4e. It reads each
   `ServiceKey::new("…")` in `crates/bingo-services/src`, finds the crates
   that import that module, and fails when such a crate's manifest lists
   the key nowhere among `provides`, `requires` or `uses`. Seed a
   violation by hand to watch it fail.
10. **Records.** ADR-0001 §5 gets a dated note that `bingo-services`
    exists and that `bingo-teams` and `bingo-acp` never did.
    ADR-0031 §2 and §5 and ADR-0054 §1 get dated notes. ARCHITECTURE.md
    lists the crate.

## Files

- `crates/bingo-sdk/src/{service.rs,host.rs,tool.rs,plugin.rs,page.rs,lib.rs,testing.rs}`
  and `testing/tests.rs` (M111 service fixtures).
- `crates/bingo-core/src/host/registry.rs` and
  `crates/bingo-core/src/host/tests/services.rs`
- `crates/bingo-services/{Cargo.toml,src/lib.rs,src/team.rs,src/mcp.rs,fixtures/}` (new)
- `crates/bingo-agents/src/{team/service.rs,lib.rs}`; `team.rs` and
  `team/seat.rs` mechanically reuse the shared name/re-export.
- `crates/bingo-rooms/src/{team.rs,lib.rs,tests.rs,hook.rs}` (service fixtures).
- `crates/bingo-mcp/src/{rows.rs,lib.rs}`
- `crates/bingo-provider-acp/src/{crossing.rs,lib.rs}`
- `crates/bingo-skills/src/pages.rs`, every `src/guide.rs` that registers
  pages
- Every `PluginManifest` (`rg -l 'sdk: "' crates`)
- `crates/bingo-plugin-rpc/tests/plugin/{service.rs,doors.rs,hooks.rs}`
- `crates/bingo/src/plugins.rs` and `crates/bingo/tests/cli/`
  (real-owner integration at the composition layer; no plugin dependency).
- The four owners/consumers' `Cargo.toml`, `Cargo.lock`, `scripts/budget.toml`
  (+1 member, 347 → 348); no new external dependencies.
- Mechanical page-key assertions in owners' `lib.rs`, SDK page tests, and
  `bingo-skills/src/tests.rs`, `bingo-plugin-rpc/src/service.rs` fixtures (protocol §6).
- `Cargo.toml` (workspace member), `scripts/check_discipline.sh`,
  `ARCHITECTURE.md`, `docs/adr/{0001,0031,0054,0069}-*.md`

## Exit criteria

- [ ] `cargo test -p bingo-sdk service` passes the typed-key contract tests.
- [ ] `rg 'service::<' crates --glob '!**/tests/**'` finds no `&str` lookup.
- [ ] `rg '"agents\.team"|"mcp\.servers"' crates` finds one match each,
      in `bingo-services`.
- [ ] `rg 'sdk: "' crates` finds nothing.
- [ ] Registry test: an undeclared service key is refused.
- [ ] `standing()` test: a missing `uses` entry leaves the plugin standing.
- [ ] `bingo plugins` black-box test names the missing used service.
- [ ] `scripts/check_discipline.sh` fails on a seeded undeclared lookup
      (output pasted) and passes on the tree.
- [ ] `cargo deny check` and `scripts/budget.sh` pass with the new crate.
- [ ] All gates of the upgrade protocol, plus the Windows cross-check for
      `bingo-services`, `bingo-rooms` and `bingo-provider-acp`.

## Non-goals

- `RoomPost` and the collaboration types (M122).
- Typed surface options and one settings resolution (M126).
- `uses` in external `plugin.json` (protocol 2, M132).
- Changing what a service answers. The wire shapes are pinned unchanged.

## Risks

- `Arc<T>` inside the `Any` costs one pointer hop per lookup; no lookup is per token.
- plugin-rpc's external services are `ServiceHandle`s under string keys.
  `find`'s wire fallback must keep an external `agents.team` owner
  working. The `doors.rs` test pins that.
- The 4e check is grep-based, like 4c. A consumer that renames the
  import (`use bingo_services::team as t`) evades it. That is accepted:
  the registry still answers `None` safely.
- Re-verified at `7265cfec`: M111 supplies `ServiceHost`; migrate its typed
  holding helper and the rooms scripted wire face. M130 adds manifests but
  the total remains 39 files. Existing manual provides assertions are mixed
  with config/registration tests; delete only redundant declaration checks.
- Checked: the `&str` lookup sites (`rg 'service::<|\.service\('`),
  the manifest count, the per-plugin `provides` tests named in brick 3,
  and that M111's test host has not changed `HostHandle`.
