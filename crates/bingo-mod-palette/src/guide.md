# One palette

Press `/` on an empty prompt for commands and skills, or `ctrl+r` for
sessions in this workspace. The list rides in the dock over the prompt and
shows eight rows at most; `N of M` counts it when more match.

Typing ranks the names: a name that starts with what you typed comes first,
then one that holds it, then one that holds its letters in order; ties keep
the list's own order. The letters that matched are lit. Arrow keys move;
Enter runs the command or sends the skill; Tab fills the name in the prompt
so you can add to it. Merely typing never runs anything. A row shows the key
that also runs its command, or the shelf a skill was read from.

Commands chosen from partial names open their argument values in the same
list. Enter runs an exactly typed command or alias, including its default
without arguments. The current value starts selected. Free text arguments
show the command Enter will run. Escape goes back one argument level, then
closes the list.

In the sessions list, "this conversation" lists the root, its agents and
rooms; Enter changes the view, including back to the root. Stored root
sessions follow, newest first (up to 20). Live rows show status, unread posts
and owed answers; stored rows show their message counts.

The `bingo.mod.palette` built-in owns this feature. An outside mod can wrap
its `Palette` render with `next`, or replace it with a root `onKey` handler.
Set `enabledPlugins["bingo.mod.palette"]` to `false` to keep a names-only
fallback. Typed slash commands, Ctrl+R and `/resume` remain usable. On
nonterminal surfaces `/help` and `/resume` explain that they are terminal
only.
