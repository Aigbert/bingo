# Architecture upgrade — 2026-10

> The work [audit-2026-10.md](audit-2026-10.md) leads to: 28 milestones
> (M108–M135) in six phases, 15 proposed ADRs (0063–0077), and the protocol
> an executing agent follows. Each milestone has a plan in `docs/plans/`;
> each ADR is `Status: proposed` until the milestone that implements it
> lands. Written 2026-10-13 at `979a6d27`. The ordering principle: fix what
> is false first, then make the journal and the turn cheap and honest, then
> give every seam a type, then share mechanism, then open the platform.

## Phases

**Phase 1 — Truth and the floor.** No new architecture. The six audit bugs,
the Windows shell, CI that tests `dev`, and a test host every crate shares.
Everything later stands on a CI that runs.

**Phase 2 — The journal and the turn.** One fold, a versioned record, a
checkpoint, pictures kept once, supervised tasks, deadlines, one owner for
the session spec. After it, a turn's cost does not grow with the session's
age and esc reaches every await by construction.

**Phase 3 — Seams the compiler sees.** Typed services, one tree fold, slots
(plugins place their own views), the collaboration library, a TUI whose
layout is a pass, typed composition. After it, the TUI names no plugin.

**Phase 4 — Mechanism in the library tier.** One HTTP stack, one process
spawner, one JSON-RPC stack, one claim, one file shelf, channel adapters.

**Phase 5 — Mods.** Interceptor chains, plugin protocol 2, the author kit.
The acceptance test: a plugin written outside the workspace, in
TypeScript, places a status entry, intercepts a tool call, and keeps its
state across a hot reload.

**Phase 6 — Agents.** The ACP seam split from `Provider`; one lifecycle
vocabulary.

## Milestones

| M | Title | ADR | After |
|---|---|---|---|
| 108 | The model sees what the person sees; the journal is written first | — | — |
| 109 | Five plugin bugs and the drift | — | — |
| 110 | The shell on Windows; CI tests `dev` | 0063 | — |
| 111 | One test host, no wall clock | — | — |
| 112 | One transcript fold | 0064 | 108 |
| 113 | The record is versioned | 0065 | 112 |
| 114 | Resume starts at a checkpoint | 0066 | 113 |
| 115 | The picture is kept once | 0067 | 113 |
| 116 | Every task has an owner | 0068 | 112 |
| 117 | Every await on another process has a deadline | 0068 | 116 |
| 118 | The session spec has one owner | — | 116 |
| 119 | A service has a type | 0069 | 111 |
| 120 | One tree fold | 0064 | 112 |
| 121 | Slots I: the mechanism, tasks, shells, wake, mode | 0070 | 113, 119 |
| 122 | The collaboration library | 0071 | 111, 116, 119 |
| 123 | Slots II: the TUI names no plugin | 0070 | 113, 121, 122 |
| 124 | Layout is a pass | — | 121, 123 |
| 125 | One focus, one render context | — | 124 |
| 126 | Composition is typed | 0069 | 119 |
| 127 | One HTTP stack | 0072 | 111, 117 |
| 128 | One process spawner, one JSON-RPC stack | 0072 | 110, 111, 117 |
| 129 | One claim, one shelf | 0072 | 109 |
| 130 | A channel is an adapter | 0073 | 109 |
| 131 | Every dispatch is a chain | 0074 | 116 |
| 132 | Plugin protocol 2 | 0075 | 121, 128, 129, 131 |
| 133 | The author kit | 0075 | 132 |
| 134 | An agent is not a model | 0076 | 112, 113, 131 |
| 135 | One lifecycle vocabulary | 0077 | 120, 122, 123, 134 |

Run in table order. "After" is the hard dependency; anything not listed is
ordered only so that two milestones do not edit the same files at once.
Phase 1 plans are written against the audited tree. Later plans were
written at the same commit and **must be re-verified at their start**
(protocol step 2).

## New crates

Eleven, all `tier = "library"` except the Feishu platform, with no new
external dependency; each plan raises `scripts/budget.toml` by its own.

| Crate | Milestone | Holds |
|---|---|---|
| `bingo-claim` | 109 | the one persistent claim (lock, stale after a kill) |
| `bingo-process` | 110 | the shell and, from M128, the process tree |
| `bingo-digest` | 114 | SHA-256 for checkpoints and blobs |
| `bingo-services` | 119 | typed service keys and their traits |
| `bingo-collab` | 122 | agent and room names, seats, readings |
| `bingo-alarm` | 122 | durable wake-ups |
| `bingo-http` | 127 | bounded clients, SSE, metered streams |
| `bingo-jsonrpc` | 128 | the codec and the peer |
| `bingo-shelf` | 129 | file shelves, ids, unified diff |
| `bingo-channel-adapter` | 130 | the platform contract and the loopback |
| `bingo-channel-feishu` | 130 | the Feishu platform (a plugin) |

## ADRs (proposed)

| ADR | Title | Amends |
|---|---|---|
| 0063 | The shell on Windows is Git Bash | — |
| 0064 | One fold for the transcript, one for the tree | 0002 §3 |
| 0065 | The record is not the live type | 0005 §2–3 |
| 0066 | Resume starts at a checkpoint | 0005 |
| 0067 | The picture is kept once | 0005 §8, 0040 §3, 0061 §1, 0062 §1 |
| 0068 | Every task has an owner | 0002 §2, 0008 §2 §7, 0035 §6 |
| 0069 | A seam has a type | 0001 §5, 0031 §2 §5, 0054 §1, 0057 |
| 0070 | A plugin places its own view | 0013 §2 §4, 0019 §8 |
| 0071 | Collaboration is one library | 0021, 0022 §3, 0025 §2, 0029 §3, 0034 §3, 0019 §8 |
| 0072 | Mechanism lives in the library tier | supersedes 0056 §5 |
| 0073 | A channel is an adapter | 0016 |
| 0074 | Every dispatch is a chain | 0032 §4 |
| 0075 | Plugin protocol 2 | 0015 §3, 0030, 0031 |
| 0076 | An agent is not a model | 0035 §1, §4, §5; 0039 §3; 0055 §1 |
| 0077 | One lifecycle vocabulary for an agent | 0027, 0029, 0060 |

A proposed ADR's Decision is the contract the milestone implements. When the
milestone lands, its status becomes `accepted · <date> · Plan: M<n>` and
each amended ADR gets a dated one-line note pointing at it. A Decision that
turns out wrong during implementation is **not** edited silently: stop and
ask (protocol step 6).

## What done looks like

- `rg '"_?bingo\.' crates/bingo-surface-tui/src --glob '!**/tests/**'`
  finds nothing outside `#[cfg(test)]` but the TUI's own manifest id, and
  `check_discipline.sh` holds it (M123).
- Starting a turn copies no journal; a 10 000-item session starts a turn in
  the time a 10-item one does (M112). Resuming it reads the journal from the
  last checkpoint (M114).
- Every `Event` and `ItemBody` variant has a golden fixture; a journal
  written by a newer binary opens with unknown records kept (M113).
- An interrupt reaches a hung hook, a hung policy prompt, a hung compactor
  and a hung command (M116).
- Windows `cargo test` is blocking in CI and the Bash tool runs there
  (M110).
- One crate each for HTTP streaming, process trees, JSON-RPC, claims and
  file shelves (M127–M129).
- `examples/plugins/ts-status` passes `bingo plugins test` and does the
  acceptance test above (M133).

## Execution protocol

For the agent that executes a milestone (written for Codex; any agent).

1. **Read first.** `AGENTS.md`, this file, the milestone's plan, every ADR
   the plan names, and the audit section it cites. Do not read `../bingo`.
2. **Re-verify the plan against the tree.** Check every path, symbol and
   line the plan names. If Files or Bricks are stale but the Decision
   holds, correct the plan in its own commit
   (`docs(plans): re-verify M<n> against <short-sha>`) before code. If the
   tree contradicts a Decision, stop (step 6).
3. **Branch.** From the current `dev`: `git switch -c m<n>-<slug> dev`. If
   the worktree is dirty with changes that are not yours, stop and report;
   never stash, reset or check out over someone else's work.
4. **Contracts and failing tests first.** A trait, a wire shape, a persisted
   record or a fixture is committed with its test before its
   implementation, and the test is seen failing for the right reason. Paste
   the failing output in the Verified section later.
5. **Build in the plan's brick order.** One commit per brick or per
   coherent step, Conventional Commits with the crate scope, subject ≤60
   characters, footer `Plan: M<n>` and `Refs: ADR-<n>` where one applies.
   No new dependency unless the plan or ADR names it; then run
   `scripts/budget.sh`. Split functions as AGENTS.md says; the discipline
   check fails at 120 lines.
6. **Stop and ask** — write what you found and the options in the plan's
   Risks, commit the plan, and end the run — when: a Decision of an ADR
   would have to change; a milestone would need a crate dependency the
   layering forbids; a gate fails and the fix is outside the plan's Files;
   two attempts at the same failure have not moved it; anything would touch
   a user's live binary, journal, settings or a process you did not start.
7. **Gates.** All must pass, run one at a time (a second cargo job beside
   them makes wall-clock tests flake):
   ```
   cargo fmt --all -- --check
   cargo check --workspace --all-targets --locked
   cargo clippy --workspace --all-targets --locked -- -D warnings
   cargo test --workspace --locked
   scripts/check_discipline.sh
   scripts/budget.sh
   cargo deny check
   cargo check -p <each touched crate> --all-targets --target x86_64-pc-windows-msvc
   ```
   A test known to hang under parallel runs (the ACP bridge token test) is
   run alone, never skipped silently. TUI-visible changes also run
   `scripts/tui-smoke.sh`. Never launch a terminal application, never send
   a signal to a process you did not start.
8. **Verify and record.** Tick each exit criterion with the command and its
   output pasted (trimmed to the lines that prove it) in a `## Verified
   (<date>)` section at the end of the plan; keep the plan ≤150 lines.
   Flip the ADR to accepted and add the dated notes to the ADRs it amends.
   A criterion not met stays unticked with the reason written beside it.
9. **Finish.** Commit `docs(plans): verify M<n>`. Do not push, do not merge,
   do not open a PR. Report: branch, commits, gate results, anything
   unticked, anything you stopped on. Then continue with the next milestone
   in the range you were given, branching from the previous milestone's
   branch, or stop at the range's end.

## Prompt to hand over

```
You are executing the bingo architecture upgrade in
/Users/yexrob/Episodes/Projects/bingo-inc/bingo-improve.

Read AGENTS.md, docs/design/upgrade-2026-10.md (follow its "Execution
protocol" exactly), and docs/design/audit-2026-10.md.

Execute milestones M108 through M111, in order. For each: read its plan in
docs/plans/, the ADRs it names, re-verify the plan against the tree, work
on a branch named m<n>-<slug> (the first from dev, each next one from the
previous), contracts and failing tests first, all gates, a Verified
section with pasted output. Never push, merge or open a PR. Stop and
report when the protocol says stop, and at the end of the range.
```

Change the range for each phase. Phase 2 and later should start only after
the previous phase's branches are merged into `dev` by a person.
