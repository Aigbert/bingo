# One palette

Press `/` on an empty prompt for commands, `?` for keys, or `ctrl+r` for
sessions in this workspace. Type to search all three groups. Arrow keys move;
Enter runs the selected command or opens the selected session. Merely typing
never runs anything. Session rows show their stored message counts.

Commands with arguments open their values in the same list. The current value
starts selected. Free text arguments show the command Enter will run. Escape
goes back one argument level, then closes the list. Shift+Tab keeps cycling the
permission mode without opening a list. Ctrl+G returns to a pending ask.

The last five commands chosen here appear first. Only their names are stored,
never arguments. `/help` opens keys; `/resume` opens sessions. Bindings follow the
`keys` setting and include commands contributed by other mods.

The `palette` built-in owns this feature. An outside mod can wrap its `Palette`
render with `next`, or replace it with a root `onKey` handler. Disable `palette`
to keep a plain command-name fallback; typed slash commands continue to run.
