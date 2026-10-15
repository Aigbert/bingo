# ADR-0094 — A schedule carries who asked for it

Status: accepted · 2026-10-16 · Supersedes: ADR-0019 §3, §4 · Amends: ADR-0093 §1

## Context

ADR-0019 §3 let a fire deliver its text as "a prompt or a `/command`
line", and §4 left a stored `acceptEdits` to "the person's call". Both
assumed the person wrote the entry. They need not have: `ScheduleCreate`
is a model tool, and an entry is a hand-editable file. Two earlier fixes
on this branch closed the hole by making every fire prose and every
stored mode tighten-only. That broke what a person really asked for:
their `/command` entries turned into prose, no schedule could widen the
mode, and a refusal was silent. The fix has to tell the person's entry
from anyone else's, and it has to say when it refuses. No kernel door
opens here. The sdk gains only `command::command_line`, the line grammar
core already parsed, so the scheduler and core read a command line the
same way. The ratchet question does not arise.

## Decision

1. **An entry records who asked for it.** It gets an optional `by` field,
   `person` or `tool`. `ScheduleCreate` writes `tool`. The person writes
   `person` with `/schedule add <spec> [as <mode>] -- <text>`. They can
   also make an existing entry theirs with `/schedule confirm <id>`, which
   keeps its text and mode exactly as `/schedule` lists them. Both verbs
   run only on an asked line, the predicate core already uses for
   person-only words. A model, a mod, a drawn button or a correspondent
   gets PermissionDenied. `/schedule` shows the field as the `by` column:
   `you`, `model`, `unconfirmed`.
2. **The person's entry fires as the person's own line.** It is signed as
   an unattended `print` line, voice Said, with no principal. Its mode is
   submitted as `/permission <mode>`, so it may widen the way a typed
   line may (ADR-0019 §4 restored). Its text is submitted as typed: a
   `/command` line runs as that command, and a `!` line runs in the shell
   (ADR-0019 §3 restored). A `!` line runs because the person typed and
   saw exactly that line when they added or confirmed it. Refusing it
   would only send them back to a hand-made wrapper.
3. **Anyone else's entry is the schedule's word.** Its mode is the
   schedule-origin `permission` action, which only keeps or tightens. Its
   text, command line or not, goes to the model as prose with
   `Delivery::Wake`.
4. **An entry with no `by` is unconfirmed.** Every entry written before
   this record falls here. It fires as in §3 until the person confirms it.
   Nothing on disk shows who wrote it, so the safe reading is the
   narrower one.
5. **A refusal is said.** This covers a refused mode, a refused command
   line, or a command line downgraded to prose. Each time, the runner
   sets the listing's trouble badge, which names the entry and the reason
   and tells the person how to confirm. The first time a process sees a
   given refusal, it also posts a `schedule.refused` warning notice.
   The turn still runs, in the session's own mode, with prose as in §3.

## Consequences

- A person's existing `/command` schedules keep running as commands once
  they are confirmed. Until then the listing and a notice say why not.
- Provenance is only as trustworthy as the store file. A process that can
  write `<data_dir>/schedules/` can write `"by": "person"`. That is the
  same trust bingo already places in `settings.json`, which a turn in
  `acceptEdits` can also write. Protecting the data dir from the model's
  own edits is a separate question; it is not answered here.
- ADR-0093 §1's `add` is this record's `add`. It stores the explicit
  `as <mode>`, or no mode at all, rather than copying the session's mode.
- An unattended `print` line answers no question. A permission prompt on
  a person's fire is declined, as `--print` declines it (ADR-0019 §4).

Refs: ADR-0008, ADR-0019, ADR-0093.
