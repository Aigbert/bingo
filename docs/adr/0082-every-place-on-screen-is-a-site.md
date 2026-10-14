# ADR-0082 — Every place on screen is a site

Status: accepted · 2026-10-14 · Plans: M159 (§§1–5), M160 (§6) · Amends: ADR-0079 §2 §4 and its non-goal · Amended by ADR-0086 (2026-10-14): §1's innermost answer is the built-in mod; the surface's `Engine` is the fallback

## Context

ADR-0079 gave a mod fifteen components to answer: the drawings Claude
Code lets a mod wrap. Everything else a surface puts on screen — a
reasoning block, a compaction line, a receipt, the turn's elapsed time,
the status facts, a toast, the welcome — is drawn by the surface alone,
so a person who wants a different screen has to fork the surface. The
person's word (2026-10-14): every place on screen becomes a site, and
the surface's own drawing is the innermost answer, replaceable like any
other. The mod system already sits below the surfaces (`bingo-mods`
imports no surface; `ui/render` is a wire method), so the same sites
must serve the desktop client in `../Rei` and the chat channels, not
only the terminal.

The ratchet: no kernel door opens. `HostApi::render` and `invalidate`
exist; this record adds component names and props to the sdk.

## Decision

1. **A site is a component.** Every place a surface draws is a
   `Component` with sdk props. The kernel link's `Engine` answer is the
   surface's own drawing, so the built-in screen is the innermost layer
   of the `ui.render` chain and a mod replaces or wraps it with `next`.
2. **Claude Code's fifteen keep Claude Code's shape and meaning**
   (ADR-0079 §1). Where bingo filled a prop with something else, bingo
   is wrong: `Spinner.message` is the text that replaces the word while
   a state overrides it (else `null`), `Spinner.suffix` the ellipsis.
   What bingo carried there — the interrupt hint, elapsed time and
   tokens — moves to its own site (§3).
3. **bingo's own sites carry facts, never drawing.** Names are bingo's;
   props are what the event stream says, with no key names, no
   preformatted strings, no cell counts. A surface spells the facts:
   - transcript: `Reasoning`, `Compaction`, `Rewind`, `Interruption`,
     `QuestionAnswer`, `PermissionReceipt`, `Asset`, `ShellRun`, and
     any `ItemBody` left without one;
   - turn: `TurnProgress { elapsedMs, tokensOut, interruptible,
     stopping }`;
   - session: `SessionStatus` (model, effort, context use, tokens: the
     kernel-owned facts the status line shows), `Toast`, `Welcome`.
   Plugin-placed views (task lists, the crew) are not sites here: they
   are the plugin's view, replaced through that plugin.
4. **A site says where it exists.** `Component::surfaces()` lists the
   surface tables (ADR-0079 §2) that draw it. Every site in §§2–3 is
   `terminal`, `desktop` and `text`; a terminal-only site (the opening,
   embers) lists `terminal` alone. `validate` names a hook matching a
   site on a surface that never asks for it.
5. **The asker names its surface.** A request carries the surface that
   asks, never a constant chosen where the slot was written: a pulled
   pane's request is completed by the surface that pulls it.
6. **The permission prompt and sign-in are sealed sites.** A mod may
   draw around them, never them. The answer must hold exactly one
   `Engine` node for the asked component, with props equal to the
   request's; anything else is answered `Engine` alone with the notice
   line ADR-0079 §5 writes. A surface that cannot draw the sealed node
   whole — clipped, covered, scrolled out — draws its own instead. The
   facts inside (who asks, which tool, the full input, the choices and
   their keys) are only ever the host's drawing.

## Consequences

- Every built-in drawing passes through `Site::draw`; a site no hook
  matches costs one lookup, as today (ADR-0079 §4).
- Mods ported from Claude Code read the same fifteen as they did there;
  a mod that read bingo's misfilled `Spinner` props sees them change.
- The interaction places — composer, completion, switcher, pickers,
  search, pager, form shell, layout — get sites in a later record, once
  the chat surface's interaction capabilities are known.
- Viewport units stay cells; the desktop client's unit is decided when
  it first asks.

Refs: ADR-0013, ADR-0065, ADR-0078, ADR-0079, ADR-0081.
Supersedes: ADR-0079's non-goal "a mod drawing the permission prompt"
for drawing around it; drawing it stays refused.
