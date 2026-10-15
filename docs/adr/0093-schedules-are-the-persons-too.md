# ADR-0093 — Schedules are the person's too

Status: accepted · 2026-10-15 · Plan: M179 · Amends: ADR-0019 §6, §8 ("visible")

## Context

A person can read schedules (`/schedule`) and end a wake (`/wake off`),
nothing else: adding, pausing, running now and forgetting are the
model's tools or a hand edit of `<data_dir>/schedules/<id>.json`.
ADR-0019 §8 bounds a wake by being visible on the status line; since
M148 (`slots_leave_the_wake_record_unplaced`) the wake is unplaced, so
that bound holds only through `/wake`. M170 makes `/status` a card of
rows, where a pending wake belongs. No kernel door; the ratchet does
not arise.

## Decision

1. **Person-only schedule lines.** ADR-0094 owns `add <spec> [as <mode>]
   -- <text>` and `confirm <id>`: explicit mode only, `by: person`.
   `pause | resume | run | forget <id>` use the same asked-line predicate.
   `run` uses the runner's signed delivery whether this process holds the
   store or not. No mutation accepts a mod's or model's own invocation.
2. **`bingo_services::schedule`**: `list -> { entries, holder, trouble }`,
   `pending(session) -> Option<Pending { at, note }>`, pause/resume/run/forget.
   Add and confirm stay on the owner's command path. Rows carry `by` and
   the explicit mode. `Pending` is the wake payload's one shape.
3. **The wake is a row of the Status card.** `bingo-mod-status` draws
   `wake in 4m · <note>` from `pending`, counting down while open; `⏎`
   prefills `/wake off`. The slot stays unplaced (M148). §8's bound now
   reads "visible (the Status card, `/wake`)".
4. **`bingo-mod-schedule` is the schedules' plugin view** (ADR-0082 §3);
   its keys return §1 lines to the client. `d` confirms forgetting; `c`
   confirms taking a model/unconfirmed entry as the person's own. All mutation lines
   require person input; wire service mutations cannot inherit it. Generic
   pane replies, including messages and session jumps, require that pane's
   owner in `drawnBy`. The client then submits an asked line; a plugin
   cannot mint consent by invoking a service or drawing a button.

## Consequences

- A wake is visible on demand, not always; the note still arrives under
  the word `wake`.
- `run` in a process not holding the store can race the holder's fire in
  the same minute: at most one extra turn; the reply says so.
- `bingo-schedule` gains a service and switches at the next start.

Refs: ADR-0011, ADR-0019, ADR-0071, ADR-0082, ADR-0086, ADR-0088.

## Amendment — 2026-10-16

[ADR-0094](0094-a-schedule-carries-who-asked.md) §1 takes over `add`:
`/schedule add <spec> [as <mode>] -- <text>`, an asked line only. The
entry stores the explicit mode or none; it does not copy the session's.

A fire moves `last_fired` once its session is open, before the line is
submitted; a refused line is spent and said (ADR-0094 §5). Each entry has
one independent attempt in flight; a session that does not open within 30
seconds leaves it unspent until a rescan. The stamp lands on the entry as
reread then, and is skipped if the entry is gone, paused or fired already.
