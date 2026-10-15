# Crew

`ctrl+n` or `/crew` opens the crew panel. `]` switches between agents,
rooms and tasks; a disabled owner plugin hides its tab. Tab moves focus. Up and down select
rows. Escape closes the panel and drops its live follow.

Agents show their first request, model, token usage and known cost. The
selected agent shows its current tool and latest line. A missing price is
`$—`, never a guessed zero. Enter steps into the session. Left or right
selects the model line; Enter opens that session's model picker.

`s` stops a running child, `x` asks before dismissing an idle child and deleting its journal.
Enter confirms; Escape cancels and stays in the list.
Only its starter may do either. `m` fills `/agents message <name> ` in the
composer; finish and send it without changing sessions. For an interrupted
agent it fills `/agents reopen <name> ` instead. An existing draft is kept.

Rooms show their purpose, seats and holder. `p`, `a` and `u` fill the commands
for purpose, seating and unseating. `c` asks before closing; Enter confirms,
Escape cancels and stays in the list. Only the holder or opener may change a room.

Tasks show subject and owner. `+` fills `/tasks add `; append `@owner` to
assign it. Space toggles done/open. `o` fills `/tasks assign <id> `.

The plain tables remain available with `/agents`, `/rooms` and `/tasks`.
The commands also accept `stop|dismiss|message|reopen`,
`seat|unseat|purpose|close`, and `add|done|open|assign` respectively.
Disable `bingo.mod.crew` in `enabledPlugins` to remove its key binding;
the crew strip then opens the palette's conversation group.
