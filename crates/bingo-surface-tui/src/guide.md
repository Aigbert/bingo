# TUI

The full-screen terminal surface: what `bingo` opens when stdin and stdout are
both a terminal. It holds no session state — it folds the same frames every
other client folds and derives what it draws at render time — so what a person
sees here, a `--print` run or an IM chat on the same session can see too.

The screen is the transcript, an input box that never moves, and one status
line: the permission mode, `N needs you` and `N running` while they are true,
where you are, the window used against the whole, and the model.

## The keys

- `enter` — send the message, or open the focused block.
- `shift+enter` — a newline. `ctrl+j`, `alt+enter` and `\` then `enter` do the
  same, for terminals that cannot tell the first one apart from `enter`.
- `esc` — one ordered stack, innermost first: a sheet, then the card that is
  asking (leaving it is its own cancel or denial), then the dropdown, then the
  running turn. **One `esc` ends a turn**: every call in flight is dropped
  where it stands, a shell command's process group with it, and the activity
  row says `Stopping…` from the frame the key was pressed.
- `esc esc` — on an empty composer, the turns of this transcript newest first,
  and `⏎` to rewind to one. Offered only where the session has a `/rewind`.
- `ctrl+c` — interrupt a running turn; with nothing running, clear the input;
  with nothing to clear it says how to leave, and again within the moment
  leaves. `ctrl+d` — exit on an empty input.
- `tab` — a second ordered stack: take the dropdown's row, else queue the line
  for after the running turn, else move the ring to the next placed row that answers
  keys. `⏎` while a turn runs *steers* it — the turn absorbs the line at its
  next barrier — and `tab` *queues*; the box says so.
- `up/down` — your own prompt history, at the first and last line of the box.
  `down` keeps the history gesture; `ctrl+r` opens sessions in the palette.
  `ctrl+g` returns to a pending ask.
- `ctrl+a/e` — start and end of the line (`home` and `end` too). `alt+b/f` —
  one word. `ctrl+w/u/k` — delete a word, to the line start, to the line end.
- `pgup/pgdn` — scroll the transcript. While it is held back, the row under it
  says `↓ 37 lines below`; `end`, or a click on that row, follows the foot again.
- `v · drag` — select: from the focused block or the first line on the screen
  with the key, from wherever the press landed with the mouse. `↑↓←→` or the
  drag itself take the far end of the run and the view follows it, so a run
  reaches what has scrolled away; held past the top or the foot of the
  transcript, a drag scrolls it a line at a time until you come back inside or
  let go. `y`, `ctrl+c` or letting the button up copies through OSC 52 and
  lets the run go; `esc` lets it go and copies nothing. What is copied is what
  was drawn: the cells as you saw them, trailing spaces trimmed. More than
  100 KiB is refused out loud rather than half-copied.
- `ctrl+t` — show and hide the task list and activity views.
- Plugins draw where they place a view: a transcript row, the activity band,
  or the status line. Legacy panel and live payloads remain data until placed.
- `shift+tab` — cycle the permission mode.
- `1-9 · y/a/n` — answer the open dialog. `ctrl+e` — expand its preview.
- `ctrl+o` — open the newest fold one rung further, and again until it is
  whole. It only ever opens; `esc` comes back.
- `ctrl+b` — background the running command.
- `/ · !` — `/` opens commands in the palette; `!` runs a shell line in the session's directory.
- `click` — on a picture, open it. Mouse reporting has to reach us at all:
  inside tmux that wants `set -g mouse on`.
- `?` — keys in the palette; type to search commands and sessions too.

## The composer

`/clear` and `/exit` (`/quit`) are this surface's own and reach no kernel.
The palette mod owns `/help` and `/resume [id]`. Every other `/name` and every `!line` is submitted
verbatim — the session actor parses commands, not the client — so a plugin's
command and a skill's `/name` are typed here and answered there.

`@` opens a dropdown of names this session can reach and paths under its
working directory. The names are the sub-agents under this session, or, in a
room, the seats on its roster and `@all`. The paths are the directory's own
files, obeying `.gitignore`, so a repository offers its sources and not its
build. A completed mention keeps its `@`, so the line itself says which of its
words are names and which are paths.

Pasting a picture writes it to `<data>/pictures/pasted/`, named by its bytes,
and puts `[image N]` in the line. **The line is the record**: what is sent is
derived from the tokens still in it, so deleting a token takes its picture with
it, and `@shot.png` is the other spelling of the same thing — a word that names
a file.

## Pictures

Whether this terminal draws pixels is asked at start-up, never assumed from
`TERM`. A terminal draws a picture here only if it speaks the kitty graphics
protocol, says how big a cell is, and names itself kitty 0.28, Ghostty 1.0,
iTerm2 3.5.6 or Rio 0.5.27 or newer — the list is a list because the protocol's
own query carries no answer for it, and two terminals that claim the key ignore
it. Everywhere else a picture is `[image: …]`, and a click still opens it in
whatever this system opens pictures with.

A picture fetched from the web is kept under `<data>/pictures/cache/` for
`pictures.cacheDays` days — a fortnight unless a person says otherwise, and `0`
means no cache at all.

## Settings

This surface claims two keys:

- `tui.measure` — the widest a line of prose is drawn, however wide the
  terminal is; prose then wraps at `min(width, measure)`. Absent, and `0`, are
  the terminal's own width. Tables and code never wrap: they fold to the width
  and open in a sheet.
- `update.check` — whether a start asks whether a newer release is out. On
  unless it is set to `false`; the check is once a day, off the start's own
  thread, and silent whenever it cannot answer. What it found is one row of the
  welcome box — `↑ v0.5.0 is out · bingo update` — and `bingo update` is what
  takes it.

The look is chosen from the environment rather than the settings, because it is
chosen before the kernel is up: `NO_COLOR` strips colour, `BINGO_ASCII=1`
strips the glyphs, `BINGO_MOTION=off` stills every animation, `BINGO_THEME` is
`light` or `dark` where the terminal's own background cannot be read.

`--no-print-on-exit` leaves the terminal as it was; without it the last
screenful of the conversation is printed on the way out, under a line saying
how to reopen the session.

A mod pane sits above or beside the transcript, or floats over it. Click
`<title> ▸` in status to reopen a collapsed pane as an overlay; click a waiting
pane to open it. On a focused pane, `alt+p` or its title’s `place ▾` cycles
places that fit. Your moves last until this TUI exits. The pane has its own
wheel scrolling and drag-to-copy selection.

## Choosing a model

Choose `/model` in the palette to list models, with the current one selected.
Its row shows the current provider/model; `/think` shows the requested effort.
Use `/model openai/gpt-5.4` to try a model in this session, or
`/model openai/gpt-5.4 --keep` to also save the provider and model for new
sessions. Project and command-line settings still take precedence at startup.
`--model` and `--provider` never write settings.

`/think high` changes the requested effort for this session. Add `--keep`
to save it for new sessions. `/think off` omits the reasoning parameter;
the provider's default then applies. Model reasoning metadata is informative
and does not prohibit a requested effort.

`/model pin openai/gpt-5.4` adds an offered model to the ordered `pinned`
setting. `/model unpin openai/gpt-5.4` removes it. Both preserve unrelated
user settings and TOML comments, and neither changes the active model.

The model catalogue supplies context and output limits, prices per million
tokens (input, output and cache reads), image input, chat suitability, dated
aliases, distinct supported effort levels and pin state. Missing prices mean
unknown. Unknown models remain visible as chat candidates. Facts come from
the bundled snapshot; listing facts never fetches prices from a provider.

If the current context exceeds the target model's compaction threshold, the
switch waits behind the active turn and compacts on the current model first.
The outcome reports the before and after token counts. Failed or insufficient
compaction leaves the model unchanged. Switching to a model without image
input keeps pictures in the transcript and warns that the model cannot see them.

## Managing providers

`/provider add work openai https://proxy.example` adds a named instance
immediately. Names use only ASCII letters, digits, `_` and `-`; an existing
stored credential must be removed before its name can be reused. With no active
provider or owning-family declaration in any layer, `/provider remove work`
deletes that orphaned credential. Shapes are
`openai`, `anthropic`, and `codex`. Key-based shapes
open a paste dialog; the key goes only to `auth.json`. Cancelling sign-in
keeps the instance, and `/login work` can finish later. URLs must use HTTP(S)
and contain no userinfo, query, or fragment; paste credentials into login.

`/models refresh work` checks one endpoint. The providers catalogue reports
its last reach result and whether its credential came from the store,
settings, or environment. A failed check keeps the instance and its previous
model list. `/provider set work url https://new.example` replaces its address.
A same-origin path change keeps the key. Changing the scheme, host, or port
clears the key and opens a paste dialog. If a project, local, or other non-user
settings file holds its `apiKey`, the change is refused and names that file;
remove the key there first. Existing sessions keep their resolved
endpoint, but a retired origin stops at its next request; choose the provider
again to use its new address.

`/provider remove work` removes its user settings entry and stored credential
and clears its saved provider, model, and matching model pins. Live sessions
using it stop at their next request with `AUTH_REQUIRED` naming the removal; choose another
provider to continue. Default providers cannot be removed.
Instances supplied only by another settings layer must be edited there.
`bingo provider add` uses the same settings writer before a session starts.

Stored keys are bound to the address where they were entered; OAuth tokens
also bind their issuer. If a settings override changes an origin, the instance
stays listed but signed out. The notice names the address and override file:
sign in again for that address, or remove the override there. Old credentials
migrate only from an unambiguous instance declaration in the user settings.
Environment keys follow only the default address or their `*_BASE_URL` variable.
Settings keys follow addresses from their own or more trusted settings layers.
A repository override cannot redirect either key to another origin. Before you
sign in under an override, confirm the destination origin and file shown in the
prompt. Blank addresses mean unset; malformed addresses cannot store a key.

Boot skips invalid or colliding names and names the declaration file to edit.
A credential owned by another family stays with that family; rename or delete
the conflicting declaration. A plugin provider takes precedence over an instance
with the same name; its notice names the plugin and the instance's file.
Shipped family names remain reserved even when their plugins are switched off.
