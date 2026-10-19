# Crew

`/crew` opens the crew sheet in the dock. It lists your teammates first,
ordered by who needs you: those asking, then those working, then the idle
ones. Rooms follow. The sheet looks the same from any session in the tree.

A teammate's row shows its lamp, its name in its own colour, what its live
turn was asked (or its team role when idle) and, on the right, `needs you`,
`working` with the time spent, or `idle`. A room's row shows its purpose and
how many are seated; its lamp breathes while someone it waits on is working.

Up and down move the pick. Enter, or a click, walks into the picked session;
esc there comes back. Tab writes `@name ` into the composer for a teammate.
Esc closes the sheet.

Bare `/agents` opens the same sheet in the terminal; other surfaces keep the
plain table. `/agents` with a verb and `/room` stay as they are. Disable
`bingo.mod.crew` in `enabledPlugins` to drop the sheet; the agent row on the
transcript stays.
