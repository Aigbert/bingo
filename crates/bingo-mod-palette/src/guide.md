# One palette

Press `/` on an empty prompt for commands, `?` for keys, or `ctrl+r` for
sessions in this workspace. “This conversation” lists the root, its agents and
rooms; Enter changes the view, including back to the root. Stored root sessions
follow, newest first (up to 20). The crew strip opens the same conversation list. Type to search all three groups. Arrow keys move;
Enter runs the selected command or opens the selected session. Merely typing
never runs anything. Live rows show status, unread posts and owed answers;
stored rows show their message counts.

Commands selected from partial names open their argument values in the same list.
Enter runs an exactly typed command or alias, including its default without arguments.
Tab fills the name and opens argument choices. The current value starts selected. Free text arguments show the command Enter will run. Escape
goes back one argument level, then closes the list. Shift+Tab keeps cycling the
permission mode without opening a list. Ctrl+G returns to a pending ask. An ask
arriving under the palette waits aside until the palette closes. Informational
and interrupt key rows are dim and skipped when choosing a shortcut.

The three commands chosen here most often appear first, under `recent`. The
counts live in your own data directory, never a project's, and only the names
are stored, never arguments. `/help` opens keys; `/resume` opens sessions. Bindings follow the
`keys` setting and include commands contributed by other mods. On nonterminal
surfaces `/help` and `/resume` explain that they are terminal only.

The `bingo.mod.palette` built-in owns this feature. An outside mod can wrap its `Palette`
render with `next`, or replace it with a root `onKey` handler. Set `enabledPlugins["bingo.mod.palette"]` to `false` to keep a names-only fallback.
Typed slash commands, `?`, Ctrl+R and `/resume` remain usable.
