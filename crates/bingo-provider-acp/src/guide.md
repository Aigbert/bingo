# ACP

Another coding agent, with its own conversation. An adapter speaking the Agent
Client Protocol is spawned as a child process. Bingo sends the newest input,
including pictures where supported, and journals its speech and external tool
calls on the same stream every surface reads.

The agent brings its own tools and login. Bingo adds a session, permission
checks when the agent asks, and its shared tools over the bridge below.

## Configuring an adapter

One row per agent under `acp.adapters`, by the name a person types. A new agent
is a new row, never a line of code — which is why no adapter is named in the
build:

```toml
[acp.adapters.claude-acp]
command = "npx"
args = ["-y", "@agentclientprotocol/claude-agent-acp"]
options = { mode = "dontAsk" }

[acp.adapters.codex-acp]
command = "npx"
args = ["-y", "@agentclientprotocol/codex-acp"]
env = { CODEX_APPROVAL_POLICY = "on-request" }
enabled = false
```

- `command`, `args`, `env` — what to run. The environment is added to the one
  this process already has, because the adapter reads its own credentials from
  there.
- `options` — what to set on the agent every time a session with it opens, in
  **the agent's own ids and values**: one `session/set_config_option` each,
  sent before the first prompt. It is the door an adapter with no flag and no
  variable leaves open; bingo knows nothing about what is said through it and
  checks it only against what the agent declared.
- `enabled` — a row kept but not registered today.
- `tools` — an explicit list of what to offer over the bridge, replacing the
  derivation below entirely, exclusions included.
- `forwardMcp` — whether the MCP servers configured for bingo ride
  `session/new` so the agent dials them itself. On by default: one hop instead
  of two, and their tools then leave the bridge so nothing is served twice.

The row's name is an identity: it is what `--provider` and `/model` say, one
word, no `/`, and never `anthropic`, `codex`, `fake` or `openai`, which this
build already answers to. Signing in is the adapter's own (`claude login`,
`codex login`), and so is permission — say what the agent may do in *its* words,
on its row, because the row speaks first.

## Running a turn through one

`bingo --provider claude-acp --model agent`, or `/model claude-acp/agent` in a
session. `agent` is bingo's own label for "whatever model the agent would have
picked itself"; it is always valid and never crosses the wire. Any other model
name is one the agent declared, and `/model` and `/think` reach it as
`session/set_config_option` between turns — the effort ladder is matched
against the values the agent listed, never invented, so a level it does not
offer is one bingo does not send.

One ACP session per bingo session, kept on the agent's side. Reopening climbs:
`session/resume` where the agent can reattach without replaying, else
`session/load` whose replay is swallowed rather than journaled twice, else a
fresh session whose first prompt names a file holding the transcript so far.

The agent's own tool calls are journaled as external tool rows with their titles,
locations and reported outputs. Bingo does not execute them. When the agent
asks permission, the call passes bingo's tool checks, hooks, policy and person.
Rules use names such as `claude-acp:execute(rm:*)`; a `Bash` rule does not grant
permission to an agent's execute call. The adapter's own configuration still
speaks first: an agent told not to ask never reaches bingo's checks.

`/rewind` can restore an asked-for edit, delete or move at its reported locations.
Edits the agent makes without asking are outside those checkpoints.

## The bridge

The agent has hands but no way to speak into this house, so bingo's shared
tools are served to it as an MCP server over a socket, one rendezvous per run
and one token per ACP session. What crosses is the turn's own tool list minus
two things: the machine's own hands, which the agent already brought
(`Read`, `Write`, `Edit`, `Glob`, `Grep`, `Bash`, `BashOutput`, `KillShell`,
`WebFetch`, `WebSearch`, `SpawnAgent`, `AskUserQuestion`), and the servers a
person's own rows hand it directly. A tool registered mid-session still
reaches it: the catalogue moving is `tools/list_changed`.

## What a bridge session cannot do

- **The agent compacts its own context, and takes no side questions.** The
  conversation lives on the agent's side, so the ruler over it is the agent's
  too: bingo draws no line in that window, warns about nothing and cuts
  nothing. When the agent makes room for itself the transcript shows a
  `context compacted (… → … tokens)` row where it happened, and the count
  carries on from there. `/compact` is refused in words — the adapter holds
  this context and compacts it itself — rather than forwarded; a request
  carrying an explicit purpose is refused too, because ACP delegates
  execution and even a text-only summary might run tools on the other side,
  and so is any plugin's question asked beside the conversation.
- **No system prompt, no caching, no token counting.** Usage is the agent's
  own count against the agent's own window, from the first `usage_update` it
  sends — and honestly zero where an adapter reports none. ACP's plans, modes
  and slash commands are unmapped, and `fs/*` and `terminal/*` are declared
  unsupported — the agent reaches the disk as itself.
- **Permission requests pass bingo's tool checks.** Rules and hooks can deny
  them or ask the person. An unanswered request selects the agent's reject
  option; without a reject option, the turn fails with a reason. Even a
  session-wide approval selects `allow_once`: bingo retains its scope and
  the adapter asks again. Free-form `elicitation/create` remains declined.

Interrupting is `session/cancel`. A child that dies before saying anything can
be replaced and asked again within the run. A child that stops mid-answer
fails the turn: its journaled speech and work stay, and nothing is retried.
The dead child is buried with one notice; the next prompt uses a new child.
Every child ends with the process that spawned it.
