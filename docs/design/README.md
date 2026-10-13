# Design documents

These are the sources the ADRs draw from. They were written before the first line of code and are archived verbatim; ADRs and plans are authoritative where they disagree.

- [plan.md](plan.md) — the approved rewrite plan (Chinese): survey, architecture, milestones, verification, conventions, porting checklist.
- [architecture.html](architecture.html) — the architecture page (seven figures: layers, event hub, one turn, turn state machine, session = journal, crate graph, roadmap).
- [kernel-and-sdk.md](kernel-and-sdk.md) — kernel boundary, plugin mechanism, the stable trait set, the event model, the turn state machine, crate layout, where each old feature lands.
- [gateway-and-surfaces.md](gateway-and-surfaces.md) — invariants, session and addressing, the client contract and wire protocol, how each surface is a client, multi-client and durability, ecosystem paths, interactions.
- [delivery.md](delivery.md) — milestones, porting-knowledge checklist with old-code pointers, test and verification strategy, repo conventions, risk register, first session.
- [audit-2026-10.md](audit-2026-10.md) — the 2026-10 architecture audit: the six largest problems, seven bugs, and a comparison with Claude Code's mods.
- [upgrade-2026-10.md](upgrade-2026-10.md) — the upgrade that follows from it: eight phases, milestones M108–M150 (M143–M145 dropped), ADRs 0063–0081 (0080 rejected), the execution protocol and the hand-over prompt.
- [mods-parity-2026-10.md](mods-parity-2026-10.md) — what a Claude Code mod can do, what bingo has, and the Phase 7 milestone that closes each gap.
- [tui-identity-2026-10.md](tui-identity-2026-10.md) — Phase 8: the TUI's rough edges, the rail given back to the person, bingo's ember look and its crew.
- `survey/` — what the old bingo had and where it hurt: [feature inventory](survey/feature-inventory.md), [engine map](survey/engine-map.md), [collaboration layer, protocol and health](survey/collab-protocol-health.md).
- `research/` — verified library and reference-implementation research: [Rust crates](research/rust-crates.md), [TUI ecosystem](research/tui-ecosystem.md), [reference architectures](research/reference-architectures.md), [provider stream model](research/provider-stream-model.md).
