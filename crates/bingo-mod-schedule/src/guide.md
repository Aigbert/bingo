# Schedule sheet

`/schedule` opens the schedules in the dock. ↑/↓ selects a row; space pauses
or resumes it. Enter closes the sheet and sends its prompt in the home
conversation. Escape closes it and restores the draft. Letter keys do nothing.
`/schedule every 30m <prompt>` adds a repeating prompt.

Automatic fires still use the schedule's own session and stored permission
mode. `/schedule run <id>` fires there now; `/schedule confirm <id> [digest]`
confirms an entry as yours, and `/schedule forget <id>` removes it while keeping
its transcript. These commands require the person's own input.

`/wake` opens a separate sheet: six delays from 10 seconds to one hour, then
cancel when a wake is pending. Enter sets or cancels it. `/wake off` cancels it
directly. With `bingo.mod.schedule` disabled, the schedule command shows its
plain table; the wake sheet and all command lines remain available.
