# M135 — One lifecycle vocabulary

## Goal

ADR-0077: what an agent is doing is one of five words — `working`,
`asking`, `idle`, `stored`, `not seated` — decided by one pure function in
`bingo-collab` and spoken the same way to the model (`ListAgents`,
`/team`) and to the person (the TUI tree and roster). The kernel stamps
`SessionSummary.asking` beside `busy`, so a listing can say `asking`
without folding a child's stream. `busy`, `running`, `done`, `Starting…`
and `Needs you` leave every surface and every model-facing text.

## Bricks, in build order

1. **Contract and failing tests.** `SessionSummary.asking: bool`
   (`#[serde(default)]`), with a golden fixture: a summary written at
   `979a6d27` reads `asking: false` (M113's fixtures). A core test that
   fails on the tree: a permission prompt opened in a turn publishes a
   `SessionUpdated` whose summary says `asking: true`, and answering it
   publishes `false`. In `bingo-collab`, `Lifecycle` and its word table
   with a test pinning the five words and their order.
2. **`decide(Facts) -> Option<Lifecycle>`**, pure: `Facts { seated, log,
   open, asking, working }`, the precedence of ADR-0077 §1, a table test
   with one row per branch and the ties (asking while working is
   `asking`; a stored session is never asking). Then the two readers:
   `of_state(&SessionState)` from `interactions` and `turn`, and
   `of_summary(&SessionSummary)` from the stamps with `open: true`.
3. **The stamp.** `session.rs` `stamped()` sets `asking` from the
   actor's `pending` table, and `session/interactions.rs` republishes
   the summary after `InteractionOpened` and after every path that closes
   one (answered, cancelled, expired). Nothing else writes the field;
   complete `SessionSummary { … }` constructors initialize it false;
   update expressions retain the source summary.
4. **The agents plugin.** `list.rs` `state()` → `of_summary(…).word()`;
   the list's tone follows the state; `team/command.rs` `NOT_SEATED` →
   `Lifecycle::Unseated`; `guide.md` and the `ListAgents` description say
   the five words. `stop.rs` and `dismiss.rs` read the state: `StopAgent`
   ends `working` and `asking`; `DismissAgent` refuses both.
5. **The verdict beside `idle`.** A pure `said(lifecycle, last:
   Option<&TurnStatus>, spent) -> String` in the TUI: `idle · completed ·
   3 tools · 1.2k tokens · 40s`, `idle · interrupted`, `idle · failed`,
   and a never-run child's brief beside `idle`. Table tests.
6. **The TUI.** `tree.rs` `Status` and `label()` go for `Lifecycle`;
   `activity()` becomes `said`; `asking()` stays as the attention fact it
   is. `roster.rs` `doing`/`spent`, `seats.rs`, `transcript.rs` child
   rows, `run.rs`'s stored rows and `views/tree.rs` follow; the
   `Wants::Running` filter reads `Working`. `TestBackend` snapshots
   change once, in their own commit.
7. **The ADRs.** Dated notes in ADR-0027, ADR-0029 and ADR-0060 pointing
   here; ADR-0060's "not decided here" line is answered.

## Files

- `crates/bingo-sdk/src/event.rs` (`SessionSummary.asking`), its fixture and snapshots
- `crates/bingo-core/src/{session.rs,session/interactions.rs}`
- `crates/bingo-collab/src/lifecycle.rs` (new), `crates/bingo-collab/src/lib.rs`
- `crates/bingo-agents/src/{list.rs,stop.rs,dismiss.rs,guide.md,team/command.rs}`
- TUI already depends on `bingo-collab`; no dependency change.
- `crates/bingo-surface-tui/src/{tree.rs,views/tree.rs,roster.rs,seats.rs,transcript.rs,run.rs}`
- `SessionSummary` literals across the workspace (`rg -l 'SessionSummary \{' crates`)
- `docs/adr/{0027,0029,0060,0077}-*.md` (dated notes and acceptance)
- Core `session/tests/{asking.rs,../tests.rs}`; SDK `event.rs` compatibility test
- TUI `src/lifecycle.rs` (pure verdict), `src/lib.rs`, affected screen tests/snapshots
- Generated `schema/{rpc,plugin}.json`, `sdk/typescript/src/wire.ts`, SDK
  record/checkpoint fixtures and their regeneration tests; mechanical
  summary constructors and vocabulary fixtures elsewhere (protocol step 6):
  agents `command.rs`; TUI `motion.rs`, `view.rs`, `screens/teams.rs`; SDK
  `state.rs` wire bytes; JSONL `journal.rs`/`fixtures/clean-v3.jsonl`;
  `scripts/tui-smoke.sh` lifecycle text fixture.

## Exit criteria

- [ ] `cargo test -p bingo-collab lifecycle::` passes: the word table and
      the precedence table.
- [ ] `cargo test -p bingo-core asking::`: open → `asking: true`, answer
      → `false`, cancel by esc → `false`.
- [ ] `cargo test -p bingo-agents`: a child with an open prompt lists as
      `asking`; `DismissAgent` refuses it; `/team` prints `not seated` for
      an empty seat.
- [ ] `rg -n '"busy"|"running"|"done"|Needs you|Starting' crates/bingo-agents/src crates/bingo-surface-tui/src --glob '!**/tests/**'`
      finds no lifecycle word (each remaining hit named and explained in
      the Verified section).
- [ ] TUI `TestBackend` tests show `asking`, `working`, `idle · completed`
      rows; `scripts/tui-smoke.sh` passes; all gates of the protocol.

## Non-goals

- Changing `SessionSummary.busy` or the channels' use of it.
- A lifecycle word in rooms; the ear's words (`live`, `listening`).
- A state for waiting on a hook, a deadline or a delivery.
- The TUI's attention and unread marks, or the window title.

## Risks

- `asking` while a holding command asks between turns: the session is not
  `working` but is `asking`; StopAgent then interrupts nothing. The test
  pins that StopAgent cancels the interaction instead, or refuses in
  words — decide at brick 4 and say which in the guide.
- Two extra `SessionUpdated` records per prompt reach every client;
  the TUI must not redraw the whole tree for a summary that changed one
  bool (check the fold's cost, M120).
- The model-facing word changes; a model mid-session may have learned
  `busy`. The `ListAgents` description carries the new words.
- Re-verify at start: M122's `bingo-collab` crate and what it already
  exports; M120's tree fold and M123's slots (where the tree and roster
  rows now live, and whether seats still come through `bingo-collab`);
  M113's fixture mechanism for the summary; M134 (the ACP agent turn
  must open and close interactions through the same table); M111's test
  host for the agents tests. Line references are from `979a6d27`.

## Re-verified (2026-10-13, f9eb788a)

- M108–M125 and M127–M134 are present. M120's tree fold backs `tree.rs`;
  M123's seats read collab; M125's focus/render context needs no change.
- M134 `turn/endpoint.rs` selects Provider/Agent; both permission paths
  reach the actor's `pending` table. Only `stamped()` changes in `session.rs`
  (M136 independently edits hooks); `plugins.rs` is outside this work.
- `interactions.rs` opens, answers and cancels; no expiry scheduler exists.
  All cancellation paths converge on `cancel_each`.
- M113's `fixtures/record/event/sessionUpdated.json` lacks `asking`; keep
  a legacy copy to pin the default before regenerating current fixtures.
- `StopAgent` between turns will refuse in words when no turn is active,
  as Risks permits; the guide will direct the person to answer the prompt.
- No new dependencies, live/credential tests, or shared tmux sessions.
