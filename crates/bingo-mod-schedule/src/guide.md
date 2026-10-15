# Schedules panel

`/schedule` opens the panel. The top line names the runner holding the store.
The `by` column says `you`, `model`, or `unconfirmed` (ADR-0094).
Schedules fire only while bingo runs. Another process may hold the store;
this panel can still run an enabled schedule now.

- ↑/↓ selects a row; space pauses or enables it.
- r runs it now; a disabled schedule must be resumed first.
- c confirms a `model` or `unconfirmed` entry as yours after showing its
  explicit mode; Enter accepts, Escape cancels.
- d opens a confirmation; Enter forgets it, Escape cancels.
- → visits its session, or says it has not fired yet.
- Escape closes the panel.

Forgetting leaves the session and its transcript intact. A concurrent runner
fire can add one extra turn when you run now.

With `bingo.mod.schedule` disabled, `/schedule` shows the plain table and
all command lines still work. The wake row belongs to `/status` independently.
