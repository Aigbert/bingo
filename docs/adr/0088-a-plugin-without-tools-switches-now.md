# ADR-0088 — A plugin without tools switches now

Status: accepted · 2026-10-15 · Plan: M172 · Amends: ADR-0057 §6

## Context

ADR-0057 made every switch take effect at the next start, and left a
live switch to a later ADR "if a case for it appears: it would need
every contribution to remember which plugin made it". The case is here:
with every feature a built-in mod (ADR-0086), turning off a picker or a
band and seeing nothing change until a restart reads as broken. The
approved prototype (2026-10-14, "状态 · 插件 · MCP") switches a
draw-only mod at once and writes `off at next start` on a plugin whose
tools the model sees, because changing the tool list mid-session
throws away the prompt cache (ADR-0048).

In-process chain links do not know their plugin today
(`chain/mod.rs::Link` carries only `external`). No `HostApi` method is
added; the ratchet question does not arise.

## Decision

1. **Two kinds of switch, decided from what a plugin contributes.** A
   pure `switch_kind(contributions) -> Now | NextStart { why }`: a
   plugin whose contributions are only commands, catalogue entries and
   interceptors at points that do not shape a request switches now;
   one that contributes a tool, provider, store, surface, service,
   context contributor, or an interceptor at a request-shaping point
   (`prompt.compose`) switches at the next start. A process plugin is
   always next start.
2. **Every contribution remembers its plugin.** `Link` and the command
   table carry the plugin id; the registry keeps the running switch set
   beside them. Dispatch skips a link or command whose plugin is off
   now; the `Engine` tail and the surface's drawing answer as for a
   plugin that was never there.
3. **JS mods switch through the reload lifecycle** (ADR-0057 M157
   amendment): off unloads the module, on loads it with admission and
   switches applied; `$.state` and `$.store` survive.
4. **The wish and the running state are two facts, never three.**
   `enabledPlugins` is the wish; the registry's switch set is what
   runs. `PluginStatus` reports both through its state (`on`, `off`,
   `pending { at: next start }`); pending is derived where listed, never
   stored.
5. **`/plugins enable|disable` answers truthfully per plugin:** `off
   now` or `off at next start` with the reason from §1.

## Consequences

- `bingo-mod-*` crates switch now unless they bring tools; the TUI
  redraws the site with its fallback in the same frame.
- A turn already running keeps its chain; the switch lands between
  turns, like `reconfigure`.
- `/mcp enable|disable` is unchanged: a server's tools already arrive
  after I/O (ADR-0009 §1).
- The headless `bingo plugins` subcommand still writes the wish only.

Refs: ADR-0009, ADR-0048, ADR-0057, ADR-0074, ADR-0086, ADR-0087.
