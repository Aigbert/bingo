# M176 — Crew in one panel

ADR-0090; related: ADR-0082, ADR-0086, ADR-0088, ADR-0053, ADR-0060, ADR-0068.

## Goal

A built-in `bingo-mod-crew` pane for this session's agents, rooms and tasks.
`ctrl+n`, `/crew` and the crew strip open it through the shared pane path.
The three plain tables remain available. Focus comes from the opener receipt.

## Bricks

1. Move M170's pure cost arithmetic and table test to sdk `spent.rs`.
2. Define the three service contracts and fixtures before implementing their
   native/wire adapters; ADR-0090 specifies the methods and authority.
3. Register services in the owners; commands use the same functions and
   parity tests cover receipts and refusals.
4. Pure cards derive names/activity, request, model, tokens and known cost.
5. Follow only the selected agent with bounded snapshot/history and the one
   event fold; invalidate for changed display facts and relevant journal events.
6. Pure key routing: view, confirmed stop/dismiss, message prefill and model
   picker; room edits/confirmed close; task add/status/owner. No view switch on send.
7. Shared pane drawing, selected card expanded, absent services hide tabs.
8. Catalogue-based opener and strip fallback; bounded initial bindings retain early shortcuts.
9. Per-crate guide Page, below 200 lines. Model choice remains documented in
   `bingo-surface-tui/src/guide.md`, “Choosing a model”.

## Files

- sdk `spent.rs`; status caller; services contracts/fixtures; collab name/lifecycle.
- agents, rooms and tasks: service adapters and shared command verbs/tests.
- New mod crew; bin registration; workspace edges and budget in ADR-0090 §11.
- TUI `crew.rs`, `pointer.rs`, generic `run/surface.rs`; initial command loading
  in `run.rs`/`run/catalogues.rs` and `run/typeahead.rs`; integration tests and TestBackend snapshots.
- Core `session/commands.rs`: generic action render metadata, no feature nouns.

## Exit criteria

- [x] Three service fixtures, native/wire lookup, command/service refusal parity.
- [x] Pure card/key/cost tests; typed shared names/activity, no duplicate row facts.
- [x] TestBackend at 108×30/80×24: agents, room seats, tasks, empty state;
  live stop, non-starter refusal, prefill/no view switch, task tick, hidden rooms.
- [x] Disabled mod removes ctrl+n; strip falls back; plain tables remain.
- [x] Review regressions: signed action, non-person Pick refusal, confirmed
  dismissal, event-only refresh, render/key interleaving, bounded startup/error
  handling, held shortcuts, Escape cancellation and own-receipt focus.
- [x] Isolated fake-provider tmux drive of model picker, dismissal and first ctrl+n.
- [x] Sequential fmt/check/Clippy/workspace tests, discipline, budget, deny,
  TypeScript tests and PTY smoke.
- [x] Autosquash onto dev 7a759d8a; own handler/signature, valued actions,
  generated-press refusal and every crew verb exercised end to end.
- [ ] Windows cross-check for each touched crate; known C-toolchain deferrals
  remain recorded rather than marked passed.

## Non-goals

Spawning from the pane, bulk room edits, cross-session task boards and desktop UI.

## Risks

Only a live fold knows an interrupted turn; an idle summary alone cannot say
stopped. One selected follow costs one stream. Caller and vouch checks remain
in force even after confirmation. Details and scope decisions live in ADR-0090.

## Review decisions

Keys own navigation/confirmation; renders merge only follow/subscriptions.
Allow 200 ms for local bindings, then paint and keep the same request alive;
modified shortcuts wait in type-ahead, while ordinary typing remains available.
Catalogue errors notify rather than fail startup. Esc cancels confirmation in
place. New room names reserve all verbs; existing journals remain addressable
as explicit arguments, with no rename or deletion. Only own Pane/View receipts
switch views; all person-only wire writes are unsafe. On dev 7a759d8a, person
controls require both asked and the drawer's vouch; the own handler is
`bingo.mod.crew:ui`, and values use `Action::with_value`. Stop now confirms in
place as requested; Escape leaves work running. Prefills remain typed input,
and service delivery uses the gate's retained queue verdict. No private queue,
name reservation exception or new dependency is needed. See ADR-0090 for why.

## Verified — 2026-10-15

Round 3 (2026-10-16): sequential fmt/check/Clippy exited 0; `RUST_TEST_THREADS=4 cargo test --workspace --locked`: 6116 passed, 0 failed, 6 ignored; `discipline ok`, `budget ok` (421 dependencies; target size advisory), deny: `advisories ok, bans ok, licenses ok, sources ok`; TypeScript: 33 passed; `tui-smoke ok` ×3. `BINGO_UPDATE_SCHEMA=1` generator tests exited 0, no schema/TypeScript drift. Isolated fake-provider tmux at 108×30/80×24: prompt painted while `command.describe` waited, first Ctrl+N replayed after release; Escape cancelled both confirmations in place, subsequent dismissal succeeded. Windows collab/crew/rooms/services: `Finished dev`; TUI deferred in aws-lc-sys (`stdlib.h`/`windows.h` missing), matching the prior TUI/bin C-toolchain deferral. Fixups remain unsquashed; no rebase, push or merge.

## Verified — 2026-10-16

Ran `GIT_SEQUENCE_EDITOR=: git rebase -i --autosquash dev` onto 7a759d8a; all five fixups folded, no merge commits. Sequential `cargo fmt/check/clippy`: exit 0; `RUST_TEST_THREADS=4 cargo test --workspace --locked`: 6180 passed, 0 failed, 6 ignored; `discipline ok`; `budget ok` (421 dependencies, core 0s, relink 0 crates, 9 GB advisory); deny: `advisories ok, bans ok, licenses ok, sources ok`; TypeScript: 33 passed; `tui-smoke ok`. `BINGO_UPDATE_SCHEMA=1` generator tests: exit 0, no drift. Windows all-targets checks: sdk/services/collab/core/agents/rooms/tasks/crew/status finished; TUI/bin exit 101, known aws-lc-sys missing stdlib.h/windows.h and rquickjs-sys MSVC flags rejected by clang. Isolated fake-provider tmux at 108×30 showed a working reviewer and its latest line; Escape cancelled stop and kept the list/work active; Enter confirmed stop and the followed card changed to stopped. Owned tmux/home/data removed; no push or merge.
