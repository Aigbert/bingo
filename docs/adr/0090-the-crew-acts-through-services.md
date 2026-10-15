# ADR-0090 — The crew acts through services

Status: accepted · 2026-10-15 · Plan: M176 · Amends: ADR-0082 §3

## Context

Agents, rooms and tasks have separate plugins and tables. A panel that
acts on all three cannot import their plugins. Their existing verbs
already enforce caller authority; no kernel feature door is needed.

## Decision

1. Three typed services in `bingo-services`, each with a key, capability,
   native-or-wire `find(host)`, wire face and fixtures:
   - `agents-control`: roster, stop, dismiss, reopen, message without switching;
   - `rooms-control`: rooms, seat, unseat, set_purpose, close;
   - `tasks-write`: board, add, set_status, reassign.
   Each command and service uses the same owner function and refusal.
2. Agent rows carry only `SessionSummary`. Names and typed activity derive
   through `bingo-collab`; live folds can distinguish interrupted idle work
   as stopped. No string state or name is stored beside the summary.
   Room rows identify their journal and name their holder for display.
3. `bingo-mod-crew` owns the pane, keys and drawing through `ui.render`.
   Tabs hide when their service is absent. `ctrl+n` binds to `/crew`; the
   strip opens it when installed and the palette's conversation group otherwise.
4. Stop, dismiss, message, reopen and room close are person-only commands:
   they end work, delete a journal, or speak for the person. Direct pane
   stop/dismiss/close and model-pick controls require both the owning mod's
   vouch and the person's `asked()` verdict. Stop, dismissal and room close
   ask for confirmation in place. Enter commits;
   Escape cancels in place through the root key handler, not slot dismissal.
   Navigation cannot change a pending confirmation's target.
5. Seat, unseat, purpose and task writes retain existing caller-session
   authority. ADR-0060 limits stop/dismiss/reopen to the starter; ADR-0053
   limits room changes to holder/opener. ADR-0068 concerns supervised futures,
   not board ownership, so no new task-owner restriction is invented.
   Wire stop, dismissal, message/reopen and close are unsafe calls; service access
   conveys no person's provenance. Empty task inputs share validation.
6. The roster includes siblings as ListAgents does; their starter refusal
   remains. Reopen addresses an existing child and never silently spawns one.
   `/room` also accepts `rooms`; its fixed words are seat/unseat/purpose/close.
   Close is exact and person-only; its post is signed by the actual caller.
   All four verbs are reserved for new room names. Legacy journals remain
   addressable as explicit verb arguments; they are neither renamed nor deleted.
7. The selected agent is followed through a bounded attachment and one
   history page, folding `SessionState::apply`. Only displayed-card or usage
   changes invalidate from that stream. Event hooks refresh relevant roster
   and journal dependencies for open panes. No displayed value uses elapsed
   time, so there is no timer. Close, session close and plugin stop drop follows.
   Subscription ids are routing dependencies, not a mirror roster.
8. The follow identifies its surface as `terminal` because it serves a
   terminal pane, as the models bridge does. The client name is the mod;
   kernel-stamped plugin provenance is independent of that surface label.
9. Model opening submits an action signed `plugin:bingo.mod.crew`, never
   typed text. Its render-surface metadata selects the terminal picker
   without granting person authority. The forwarded opener receipt first
   views the child and then focuses its picker, defaulting to session scope.
   Only this client's own Pane/View receipt can change its viewed session.
   Republishing a pane never takes focus. Queued/started replies finish dispatch.
10. Quiet spawn prompts remain absent from M175's `first_ask`; cards show
    `asked —` rather than relabelling agent speech as a person's request.
    Missing model prices remain unknown. Cost arithmetic is shared in sdk.
11. Guides are per-crate registered Pages (ADR-0054). Workspace edges are
    crew → sdk/services/collab, tasks → services and bin → crew. Test-only
    TUI edges exercise real crew/agents/tasks/fake services. No production
    plugin imports another plugin and no external dependency is added.
    The member budget grows 420 → 421.
12. The TUI allows 200 ms for initial bindings: enough for the local catalogue,
    without letting a multi-second plugin handshake delay first paint. The
    same request continues in the background on timeout. Modified shortcuts
    and function keys wait in M180 type-ahead; plain typing and exit stay usable.
    Catalogue errors notify and release input, never fail startup.
    Crew uses `]` to cycle its tabs: Tab belongs to M180's shared focus ring.
13. Crew registers/draws `CommandSpec::ui_name("bingo.mod.crew")`; its own
    handler press is signed as the person by the surface. Other drawers cannot
    register that handler or the built-in name. Values use `Action::with_value`.
    Prefills wait for typed submission; agent delivery goes through the ordinary
    gate and retains its queued prose verdict. There is no crew input queue.
14. Keys own navigation and confirmation; rendering merges only follow and
    subscription ids under one lock, and reads current UI fields for its view.
    A stale render cannot move selection or restore a cancelled confirmation.

## Consequences

- Commands and wire clients retain the plain tables and the same authority.
- Owners gain services and switch at next start (ADR-0088); crew switches now.
- Generic command actions retain render metadata without inheriting person
  attribution. The kernel gains no crew noun or feature-specific operation.
- Only selected live state is retained; a summary without turn status cannot
  claim that an unselected idle agent was stopped.

Refs: ADR-0001, ADR-0053, ADR-0054, ADR-0060, ADR-0068, ADR-0077,
ADR-0082, ADR-0086, ADR-0088.
