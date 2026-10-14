# M172 — A plugin without tools switches now

ADR-0088. Needs M167 (built-in links); M171's `PluginState`.

## Goal

Turning off a plugin that only draws, adds commands or hooks points
that do not shape a request takes effect between turns, not at the next
start. A plugin with tools, a provider, a store, a service, a context
contributor or a `prompt.compose` hook still switches at the next start
and says why. The listing tells the wish from what runs.

## Bricks

1. **Kind, pure.** `switch_kind(&[ContributionKind]) -> Now |
   NextStart { why }` in `bingo-core::plugins`; `why` names the first
   contribution that pins it (`tools`, `prompt.compose`, …). The
   catalogue marks request-shaping points (`shapes_request` on the
   `PROMPT_COMPOSE` entry). Table test over every contribution kind.
2. **Links know their plugin.** `chain::Link.plugin: String`, set by
   the registry for in-process and outside links; the command table
   records the owning plugin. Test that every registered link names a
   plugin in the listing.
3. **The running set.** `Registry.switches.running: RwLock<BTreeSet<String>>` of
   plugins switched off now; dispatch skips their links and commands
   between turns (a turn keeps the chain it started with). Test: off
   mid-turn lands on the next turn.
4. **JS mods.** Off unloads through the M157 lifecycle, on loads with
   admission and M171's verdict; `$.state`/`$.store` survive. Test in
   `bingo-mods` with a temp plugin dir.
5. **Listing.** `PluginState` gains `Pending { now: bool, at: NextStart
   }`, derived where listed from `enabledPlugins` against the running
   set, never stored. `/plugins enable|disable` answers `off now` or
   `off at next start (tools)`. Table test for the derivation.

## Files

- `crates/bingo-core/src/{plugins.rs,chain/mod.rs,host/registry.rs,
  commands/plugins.rs}`; `crates/bingo-sdk/src/{catalogue.rs,plugin.rs}`;
  `schema/`, `sdk/typescript/`.
- `crates/bingo-mods/src/{manager.rs,load.rs}`.

## Exit criteria

- [x] Kind and derivation tables; link-owner test.
- [x] `TestBackend` with `bingo-mod-hello`: `/plugins disable hello` →
      the `Toast` is the surface's own drawing in the next frame,
      `/hello` is gone, `ctrl+x` does nothing; enable brings all back.
- [x] A fixture plugin with a tool answers `off at next start (tools)`
      and keeps its tool in the running session.
- [x] Native gates as M167.
- [ ] Windows checks for every touched crate: four C-toolchain deferrals below.

## Decisions

- Admission's old unit `Pending` becomes `Registering`; `Pending { now, at }`
  exclusively describes a wish differing from the running state. This avoids
  conflating M171 admission with a restart and keeps pending derived.
- Retain registered native contributions for eligible plugins switched off at
  boot. Initialize these resident plugins once; dispatch controls visibility.
  Plugins with requirements remain next-start: their dependency fixpoint cannot
  safely change during a session. Unknown late sources fail closed.
- The runtime consumes the merged switches and owns the resolved wish beside
  its running set; accepted switches
  update it only after the user-layer write succeeds. No pending flag is saved.
- Extend the file list to the existing host/turn dispatch seams and tests, the
  plugin-source switching contract and module reload helpers, admission readers,
  and crate-owned guides. These are necessary consumers of the changed contracts.
- Freeze a turn's hooks once, sharing that snapshot with bridged calls and the
  completion chain. Live native dispatch filters a single owner set; core
  command fallbacks remain registered beneath switchable commands.
- Module retirement keeps old generations alive while a captured chain or
  callback holds them, including their admitted grant scoped to each operation.
  Catalogue rereads would otherwise revoke a running turn at its next `$` call.
  Box the scoped host operation: the composed three-mod smoke exposed stack
  overflow when the new grant wrapper carried that large future inline.
  Enable waits for retirement before reusing resources.
  Dynamic `tool` authority is conservatively next-start even before registration;
  otherwise an unloaded module could add tools while being enabled.
- The sole new dependency edge is TUI → core in dev-dependencies, to drive
  the real `/plugins` command in TestBackend coverage; no external dependency.
- Group the native switch fields and the module off/retirement fields into
  their own state records; use module-local lifecycle helpers. Discipline
  exposed oversized registries and a fourth Manager impl file in the first cut.
- Switching documentation belongs in the existing mods Page; add a second Page
  if full, and correct the product guide's existing `/plugins` summary. The
  199-line mods guide links to a second registered Page, `mods-switches`;
  the bundled map stays within its stricter 80-line body limit (ADR-0054).

## Non-goals

Live switching of tools or providers; unloading a process plugin
without a restart; per-session switches.

## Risks

- A built-in mod holding state in its own fields keeps it while off:
  that is allowed (the next on resumes it); a test pins that no frame is
  drawn from it while off.
- Racing a switch with a command already dispatched: the command runs
  to its end; the switch is read once per turn and once per command.

## Verified — 2026-10-15

Final gates ran sequentially; native commands exited 0. No live/credential tests.
```text
cargo fmt --all -- --check: no output
cargo check --workspace --all-targets --locked: Finished dev profile
cargo clippy --workspace --all-targets --locked -- -D warnings: Finished dev profile
RUST_TEST_THREADS=4 cargo test --workspace --locked: 5578 passed; 0 failed; 5 ignored (166 summaries)
bash scripts/check_discipline.sh: discipline ok (advisory warnings remain)
bash scripts/budget.sh: 412/412 dependencies; warm core 0s; isolation 0; budget ok
cargo deny check: advisories ok, bans ok, licenses ok, sources ok
bash scripts/tui-smoke.sh: tui-smoke ok (isolated socket and temporary HOME)
cd sdk/typescript && npm test: tests 31; pass 31; fail 0
```
Generators, each with `BINGO_UPDATE_SCHEMA=1 cargo test --locked`:
```text
-p bingo-sdk: 189 passed; 0 failed
-p bingo-plugin-rpc -p bingo-surface-rpc --lib: 150 passed; 0 failed
-p bingo-mods --lib schema_and_types_are_generated_from_the_same_rows: 1 passed
-p bingo-store-jsonl --lib journal: 35 passed; 0 failed
```
Regenerated schema/events.json, schema/plugin.json, schema/rpc.json and
sdk/typescript/src/wire.ts through those generators; no handwritten wire edits.

Ran `cargo check -p EACH --all-targets --target x86_64-pc-windows-msvc --locked`
for all eight touched crates. Exit 0: bingo-{sdk,core,plugin-host,skills}.
Exit 101: bingo, bingo-{mods,plugin-rpc,surface-tui}; aws-lc-sys lacks
`stdlib.h`/`windows.h`. bingo/mods also fail in rquickjs-sys because host clang
rejects MSVC `/std:c11` and `/experimental:c11atomics`. No Rust source diagnostic;
the Windows criterion remains unticked pending these C prerequisites.

Two intermittent failures passed alone: gateway's
`doctor_never_removes_the_gateway_claim_file`, and CLI's
`a_permission_no_rule_could_silence_offers_no_session_rung` (read a partial PTY
prompt). Final full suite passed with four test threads to reduce contention.
Verification also corrected the tool fixture, the bundled guide's line limit,
and two TUI action fixtures needing a Tokio runtime for catalogue refreshes.
The first smoke exposed a scoped-operation stack overflow; the new composed-mod
black-box test reproduced it, passed after boxing, and the full smoke then passed.
Budget warned at 9 GB of debug output; final worktree cleanup is `cargo clean`.
