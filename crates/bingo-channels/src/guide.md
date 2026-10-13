# Channels

A session in a chat thread. One concurrent surface holds an adapter per
platform. Each chat or topic thread is a session, keyed
`<adapter>/<chat>[/<thread>]` and reopened by that key. The surface folds the
same event stream as the terminal.

## Configuring one

Under `channels`, use one entry per registered adapter. Each platform owns
its settings and credentials. `bingo channels add <adapter>` asks for its
public settings and secret, storing the secret separately from settings.
`bingo channels secret <adapter>` rotates its secret.

```toml
[channels.coalesce]
minChars = 48
intervalMs = 700
```

`coalesce` controls redraws: `minChars` (48) new characters trigger a redraw;
`intervalMs` (700) bounds the wait. `bingo --channels <adapter>[=<value>]`
configures a platform for one run; the platform defines what the value means.
Unknown platforms and invalid settings are refused before listening.

The local `loopback` adapter is disabled unless `BINGO_CHANNEL_LOOPBACK` is
set. It is for tests and inspecting the wire. With the switch set:

```toml
[channels.loopback]
peer = "127.0.0.1:9000"
access = { admins = ["test-user"] }
```

`peer` is the NDJSON endpoint. `edits`, `buttons`, `typing`, `threads`,
`files`, and `acknowledge` enable mechanisms by default; set them false to
exercise the fallback. `mention` defaults to `@bingo`; `maxText` and
`maxActions` set the platform limits. Without a peer it records locally.

## Who may speak

Each adapter takes an `access` block. `admins` lists principals who pass
all rules except `off`. `direct` and `group` hold the default rules;
`rules` replaces the group rule for each named chat.

A rule has `policy`, `list`, and `mention`. Policy is `open`, `allowlist`,
`blocklist`, `admins`, or `off`. `mention: false` engages a group on every
message. Direct chats need no mention. Defaults are `open` and
`mention: true`; refused arrivals receive no response.

## Messages and questions

An open question accepts its answer; other messages submit prompts. `/`
commands and `!` shell lines reach the same session actor as in the terminal.
A question uses native buttons when available, otherwise numbered choices.
Free text works where the question permits it. Resolving a question from any
surface removes its buttons and shows the outcome. Streaming uses cumulative
text, or one final message when editing is unavailable.

Pictures become turn images. Other attachments become local paths for the
file tools; the platform controls download limits and retention.

`SendFile` sends a local `path` with an optional `caption` to this session's
chat. A relative path uses the session's working directory. It is gated by
the path permission and refuses files over 30 MiB or sessions outside a chat.

## Running one

`bingo channels` listens without taking a terminal. `bingo gateway` manages
one resident listener per data directory: `start`, `stop`, `restart`,
`status`, `logs`, `doctor`, `install`, and `uninstall`. The doctor checks
settings, credentials and claims. A gateway needs at least one channel.

Channel claims use OS locks on permanent `channels/*.lock` files. A crash
releases ownership automatically; never remove a modern claim file. An old
bare-PID or empty claim is refused unchanged. Stop every bingo using that
data directory, inspect and remove the named legacy file, then restart.

## Settings layers

Channel settings use the kernel's deep merge. Objects in two layers merge
field by field; a higher layer can turn a lower layer's channel off with
`null` in a JSON settings file (`{"channels": {"feishu": null}}`), or clear
all channels with `{"channels": null}`. An empty object preserves lower
fields. `--channels` still explicitly starts listening.
