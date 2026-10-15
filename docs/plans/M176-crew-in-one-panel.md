# M176 — Crew in one panel

ADR-0090, ADR-0082, ADR-0086, ADR-0088, ADR-0053, ADR-0060, ADR-0068.
Needs M167, M175 (`first_ask` on the summary); M166 for the model row.

## Goal

The agents, rooms and tasks of this session in one panel, acted on
where they are seen. `ctrl+n` or `/crew` opens a pane with three tabs.
Agents: a card per agent with what it was asked, its model, tokens and
cost, the tool it is in, its last line. Rooms: purpose, seats, holder.
Tasks: the board with owners. `⏎` steps into an agent's session, `s`
stops, `x` dismisses, `m` messages it without switching, `+` adds a
task, `space` ticks one. The feature is the built-in mod
`bingo-mod-crew`, a plugin view (ADR-0082 §3); `/agents`, `/rooms` and
`/tasks` keep their tables as the plain fallback.

```
 crew · agents (3) │ rooms (1) │ tasks (4)                 tab switches
❯ reviewer   ● running · Read src/fork.rs             sonnet  12k · $0.04
    asked  review the fork cut for off-by-one turns
    last   the boundary test misses a turn with a Rewind item…
  fixer      ○ idle · 2m ago                          opus    40k · $0.31
  scout      ■ stopped                                haiku    3k · $0.00
 ───────────────────────────────────────────────────────────────────
 ⏎ step in  s stop  x dismiss  m message  ⏎ on model: pick  esc
```

## Bricks

1. **What it cost, one brick.** Lift M170's pure `spent(usage, price)`
   from `crates/bingo-mod-status/src/spent.rs` into
   `crates/bingo-sdk/src/spent.rs`; both mods call it. Its table test
   moves with it.
2. **Typed services, contracts first.** `crates/bingo-services/src/
   {agents_control.rs,rooms_control.rs,tasks_write.rs}`, each a
   `ServiceKey`, capability, `find(host)` and wire face as `room_post.rs`;
   fixtures under `crates/bingo-services/fixtures/{agents_control,
   rooms_control,tasks_write}/` before any implementation:
   - agents: `roster(session) -> [AgentRow { name, summary, state }]`,
     `stop`, `dismiss`, `message(name, text)`, `reopen(name, text)`;
   - rooms: `rooms(session) -> [RoomRow { name, purpose, seats, holder }]`,
     `seat`, `unseat`, `set_purpose`, `close`;
   - tasks: `board(session) -> [TaskRow]`, `add`, `set_status`,
     `reassign`. Each answers its command's refusal (ADR-0060 starter,
     ADR-0053 §5 holder/opener, ADR-0068 owner).
3. **One code path per verb.** `bingo-agents` registers agents-control
   over `stop::stop`, `dismiss::dismiss`, `message.rs`, spawn's `reopen`;
   `bingo-rooms` over `seat.rs` and `service.rs::RoomWriter`;
   `bingo-tasks` over `create.rs`, `update.rs`. The commands gain the
   missing lines and call the same functions: `/agents message <name>
   <text>`, `/rooms seat|unseat <room> <agent>`, `/rooms purpose <room>
   <text>`, `/tasks add <subject> [@owner]`, `/tasks done|open <id>`,
   `/tasks assign <id> <owner>`. Test per verb: command and service
   answer the same text, refusals included.
4. **Cards, pure.** `crates/bingo-mod-crew/src/cards.rs::agent_card(row,
   live, price) -> Card`: state glyph, asked (`summary.first_ask`), model,
   tokens and `spent`, current tool and last line from `live`. Room and
   task rows likewise. Table test per sketch row, incl. a stopped agent
   and a summary without `first_ask`.
5. **Live.** While the pane is open the mod follows the selected agent
   through its handle (`open` ById, bounded snapshot, `events_since`) and
   folds with `SessionState::apply` (ADR-0002); switching rows drops the
   old follow. A stored agent shows its last line from one history page.
6. **Keys as verbs.** `crates/bingo-mod-crew/src/keys.rs`, (tab, row,
   key) → outcome: agents `⏎` → `{"view": session}`, `s`/`x` →
   service, `m` → `Composer` prefill `/agents message <name> ` (a stopped
   agent: `reopen`), `⏎` on the model line → `/model` on that session
   (M166 picker, `scope: session`); rooms `p` → prefill `/rooms purpose`,
   `a`/`u` seat/unseat, `c` close after a confirmation; tasks `+` →
   prefill `/tasks add `, `space` → `set_status` done/open, `o` →
   prefill `/tasks assign`. A refusal is a toast with the service's text.
   Table test.
7. **Drawing.** `draw.rs`: a `Pane` view (`Slot::Pane { id: "crew",
   focus, close_on_escape }`), tabs hidden for an absent service, the
   selected card expanded, others one line. Snapshots at 108×30 and 80×24
   for: three agents, a room with two seats, a board, no crew at all.
8. **Opener.** `/crew` command with `keys: ["ctrl+n"]` (ADR-0086 §5);
   `crates/bingo-surface-tui/src/keys.rs` and `keys/commands.rs` list
   it; a click on the crew strip (`crew.rs`) opens the pane when the mod
   is on, else the palette's conversation group (M168).
9. **Guide.** `crates/bingo-mod-crew/src/guide.md` (<200 lines) by
   `guide.rs`, as `bingo-mod-palette`.

## Files

- `crates/bingo-sdk/src/spent.rs`, `lib.rs`; `crates/bingo-mod-status/src/
  spent.rs` (deleted, callers moved).
- `crates/bingo-services/src/{lib.rs,agents_control.rs,rooms_control.rs,
  tasks_write.rs}` and fixtures.
- `crates/bingo-agents/src/{lib.rs,command.rs,service.rs,message.rs}`;
  `crates/bingo-rooms/src/{lib.rs,command.rs,service.rs}`;
  `crates/bingo-tasks/src/{lib.rs,command.rs,update.rs}`.
- New `crates/bingo-mod-crew/src/{lib.rs,cards.rs,live.rs,keys.rs,draw.rs,
  guide.rs,guide.md}`; listed in `crates/bingo/src/main.rs`; +1 member in
  `scripts/budget.toml`, no dependency.
- TUI: `keys.rs`, `keys/commands.rs`, `crew.rs` (strip click).

## Exit criteria

- [ ] Fixtures for the three services; `find` native and wire tests.
- [ ] Command and service parity test per verb, refusals included.
- [ ] Cards and keys tables; `spent` table green in its new home.
- [ ] `TestBackend`: `ctrl+n` opens; `s` on a running agent stops it and
      the card says `stopped`; `x` by a non-starter is refused with
      ADR-0060's text; `m` prefills and sending does not switch view;
      `space` ticks a task; rooms tab hidden with `bingo-rooms` off.
- [ ] `enabledPlugins: {"bingo.mod.crew": false}`: `ctrl+n` is unbound,
      the strip click opens the palette group, the three tables draw.
- [ ] tmux drive on an isolated socket, temp HOME, fake provider scripted
      to spawn two agents and open one room; captures pasted.
- [ ] `bash scripts/tui-smoke.sh`; `cargo fmt --all -- --check`; `cargo
      check --workspace --all-targets --locked`; `cargo clippy --workspace
      --all-targets --locked -- -D warnings`; `cargo test --workspace
      --locked`; `scripts/check_discipline.sh`; `scripts/budget.sh`;
      `cargo deny check`.
- [ ] `cargo check -p bingo-services -p bingo-mod-crew -p bingo --all-targets
      --target x86_64-pc-windows-msvc --locked`.

## In-flight dependencies

- M166: the model row opens its picker scoped to the agent's session.
- M170: `spent.rs` moves out of `bingo-mod-status`; land after M170.
- M173: the three plugins report `off at next start` (they gain a
  service, ADR-0088 §1); `bingo-mod-crew` switches now.
- M174: card keys resolve against the current drawing, not a stale one.

## Non-goals

Spawning an agent from the panel; editing a room's members list in bulk;
a board across sessions; the desktop's drawing.

## Risks

- Following a live agent's frames costs a stream per open card; only the
  selected one is followed and the follow ends with the pane.
- `ctrl+n` may be read as "new"; the `?` list names it `crew`. A person
  who wants it back rebinds it (ADR-0086 §5).
- A card's tool and last line can lag a frame behind the strip; both fold
  the same stream, so they converge.
