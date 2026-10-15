# ADR-0093 — Schedules are the person's too

Status: proposed · 2026-10-15 · Plan: M179 · Amends: ADR-0019 §6, §8 ("visible")

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

1. **`/schedule add <spec> -- <text> | pause | resume | run | forget
   <id>`.** `add` writes the entry `ScheduleCreate` writes, with this
   session's cwd and mode; `run` fires now through the runner's own
   delivery, held or not; bare `/schedule` is unchanged.
2. **`bingo_services::schedule`**: `list -> { entries, holder }`,
   `pending(session) -> Option<Pending { at, note }>`, and the §1 verbs.
   `Pending` is the one shape of a wake: `wake::payload` serialises it.
3. **The wake is a row of the Status card.** `bingo-mod-status` draws
   `wake in 4m · <note>` from `pending`, counting down while open; `⏎`
   prefills `/wake off`. The slot stays unplaced (M148). §8's bound now
   reads "visible (the Status card, `/wake`)".
4. **`bingo-mod-schedule` is the schedules' plugin view** (ADR-0082 §3);
   its keys run the §1 lines, `d` after a confirmation.

## Consequences

- A wake is visible on demand, not always; the note still arrives under
  the word `wake`.
- `run` in a process not holding the store can race the holder's fire in
  the same minute: at most one extra turn; the reply says so.
- `bingo-schedule` gains a service and switches at the next start.

Refs: ADR-0011, ADR-0019, ADR-0071, ADR-0082, ADR-0086, ADR-0088.
