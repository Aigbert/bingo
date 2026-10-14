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
3. **The running set.** `Registry.running: RwLock<BTreeSet<String>>` of
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

- [ ] Kind and derivation tables; link-owner test.
- [ ] `TestBackend` with `bingo-mod-hello`: `/plugins disable hello` →
      the `Toast` is the surface's own drawing in the next frame,
      `/hello` is gone, `ctrl+x` does nothing; enable brings all back.
- [ ] A fixture plugin with a tool answers `off at next start (tools)`
      and keeps its tool in the running session.
- [ ] Gates as M167.

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
  Enable waits for retirement before reusing resources.
  Dynamic `tool` authority is conservatively next-start even before registration;
  otherwise an unloaded module could add tools while being enabled.
- The sole new dependency edge is TUI → core in dev-dependencies, to drive
  the real `/plugins` command in TestBackend coverage; no external dependency.
- Switching documentation belongs in the existing mods Page; add a second Page
  if full, and correct the product guide's existing `/plugins` summary.

## Non-goals

Live switching of tools or providers; unloading a process plugin
without a restart; per-session switches.

## Risks

- A built-in mod holding state in its own fields keeps it while off:
  that is allowed (the next on resumes it); a test pins that no frame is
  drawn from it while off.
- Racing a switch with a command already dispatched: the command runs
  to its end; the switch is read once per turn and once per command.
