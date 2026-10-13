# M123 — Slots II: the TUI names no plugin

## Goal

ADR-0070 §4, §5 and §7, and ADR-0071 §4. After M121 the TUI still
reads four producers by name: the memory directories
(`_bingo.context`/`memory`), the `Skill` tool and the `skill` command
family, the rooms records (`seats.rs`, `roster.rs`, `mentions.rs`),
and the Bash call it promotes (`input.rs`). It also decides how a line
was said from a closed list of producers' surface words
(`transcript/said.rs`). Each producer now stamps a voice or places a
`Row`, the room records are read through `bingo-collab`, and a
discipline check keeps plugin ids and tool names out of TUI production
code. No `HostApi` verb, no new external dependency.

## Bricks, in build order

1. **Contract tests first**, failing:
   - sdk: `Voice` round-trips to golden JSON; an `Origin` without
     `voice` reads as before (M113's origin fixture, unchanged);
   - TUI `said.rs`: a stamped voice wins over the surface; a voiceless
     origin with a legacy surface word reads as it did; any other
     voiceless origin is `Said`;
   - a `Row` with `title`, `badge` and `actions` draws in one rendering
     test, with a fake plugin named nowhere in the TUI;
   - collab readers over M122's room fixtures give the seat columns,
     the unread counts and the `@` targets the TUI shows today.
2. **`Origin.voice: Option<Voice>`** (`bingo-sdk/src/event.rs`),
   omitted when `None`, with `Origin::voiced(Voice)`. Variants `Said`,
   `Note`, `Quiet`, `Aside`, as ADR-0070 §4 defines them.
3. **Producers stamp.** Each sets the voice it means, with a test beside
   its origin:
   - `Quiet`: agents `message::origin`; bash `notify.rs`; schedule
     `runner.rs`; core's command prompt (`session/commands.rs`,
     `Invocation::of`).
   - `Note`: schedule `wake.rs` `origin()`.
   - `Aside`: rooms' nudge (`post.rs`). M122's serial bounce is now a
     `ToolOutput`, with no origin to stamp.
   - `Said`: channels `runner.rs` and agents `hook.rs`, explicitly.
   - Contributor pieces carry none: `Origin::is_context` already omits
     them, so `ContextPiece` is unchanged.
4. **The TUI reads the voice.** `said::voice(origin)` replaces `quiet`.
   `QUIET_SURFACES`, `ROOMS` and `WAKE` become one private table for
   voiceless records, frozen; `ROOMS_READING` goes, since
   `transcript.rs` already drops context. The pending area
   (`activity.rs`) asks `voice` too.
5. **`HookContext.item: Option<ItemId>`.** The gate sets it for
   `before_tool` from `GateInput.item`, and M131’s `chain/tool.rs::after_outcomes` from the
   outcome's item. Every other construction says `None`.
6. **Memory places.** `bingo-context` gains a `before_tool` hook
   (filters `Read`, `Write`, `Edit`; matchers support no alternation). When the path's parent is a memory
   directory, it extends `row:<item>` with
   `Row { item, title: verb, badge: memory name }`. The verbs and the
   name rule move from `memory.rs` with its tests. The `memory`
   directories record stays for the model's prompt and the guide.
7. **Skill places.** Both doors place the same `Row { mark: '❖',
   title: "Skill", badge: name, body: args }` under `row:<item>`: the
   `Skill` tool on `ToolContext.item` when it runs, and a new skills
   hook at `on_turn(Start)` on the prompt item when its first line is
   one of the plugin's own `/name`s. Both doors keep today's `❖`; the
   TUI draws the mark ADR-0070 §1 gives `Row`, and names no skill.
8. **Promote is a placed action.** Bash's jobs signal gains one `Row`
   per foreground call in flight, with the action
   `{ label: "background", action: Action { name: "bash.promote",
   args: call_id }, key: Some('b') }`. ctrl+b fires the newest
   unfinished item's placed action keyed `b`. `PROMOTE` and
   `running_shell` go; `NOTHING_RUNNING` stays the TUI's word.
9. **Rooms through `bingo-collab`.** The TUI depends on `bingo-collab`
   (a plugin may depend on a library). `seats.rs` keeps `Seat`,
   `Counts` and their drawing, and loses `PLUGIN`, the kind and column
   constants and the parsing, which become collab readers beside the
   types M122 moved. `roster.rs` and `mentions.rs` read through them;
   `HOLDER` is `bingo_collab::room::HOLDER`.
10. **The check.** `check_discipline.sh` gains 4f: in
    `crates/bingo-surface-tui/src`, cut at `#[cfg(test)]` and skipping
    the modules `lib.rs` declares under `#[cfg(test)]` (`doubles`,
    `motion`, `painted`, `screens`, `test_lanes`, `test_support`), it
    fails on a `"bingo.`/`"_bingo.` literal and on a quoted 4c tool
    name. It allows the TUI's own manifest id and `acp.rs` (ADR-0076).
    Seed a violation by hand to watch it fail.
11. **Records.** `docs/design/tui.md` §4 and §8 say who places the
    memory row, the skill row and the promote key. ADR-0070 is accepted
    for §4, §5 and §7, ADR-0071 for §4; ADR-0034 §3 and ADR-0044 get
    dated notes.

## Files

- `crates/bingo-sdk/src/{event.rs,hook.rs,slot.rs}`, `fixtures/`
- `crates/bingo-core/src/{gate.rs,turn.rs,chain/tool.rs,session.rs,session/commands.rs,session/invoke.rs}`
- `crates/bingo-context/src/{lib.rs,memory/mod.rs}`, a new `memory/row.rs`
- `crates/bingo-skills/src/{lib.rs,tool.rs}`, a new `hook.rs`
- `crates/bingo-tool-bash/src/{jobs,promote,notify,lib}.rs`
- `crates/bingo-agents/src/{message,hook}.rs`, `crates/bingo-rooms/src/{post,serial}.rs`,
  `crates/bingo-schedule/src/{wake,runner}.rs`, `crates/bingo-channels/src/runner.rs`
- `crates/bingo-collab/src/room.rs` and `room/readers.rs` (readers)
- `crates/bingo-surface-tui/{Cargo.toml,src/{transcript,transcript/said,transcript/slots,pager,blocks,input,seats,roster,mentions,activity,theme}.rs}`;
  `{memory,skill}.rs` deleted; `screens.rs`, `screens/skills.rs`, `test_support.rs`, `lib.rs`
- `Cargo.lock` (TUI library dependency); mechanical `Origin`/`HookContext`
  constructors in core, checkpoints, provider-acp, hooks-shell, plugin-rpc,
  agents, rooms, context, schedule and bin tests; sdk schema golden fixtures.
- `scripts/check_discipline.sh`, `docs/design/tui.md`,
  `docs/adr/{0034,0044,0070,0071}-*.md`

## Exit criteria

- [ ] `cargo test -p bingo-sdk voice` passes; the failing first run is pasted.
- [ ] `rg '"_?bingo\.' crates/bingo-surface-tui/src --glob '!**/tests/**'`
      finds nothing outside `#[cfg(test)]` code but the TUI's own
      manifest id, and check 4f fails on a seeded literal (output pasted).
- [ ] `rg 'QUIET_SURFACES|ROOMS_READING|PROMOTE|fn running_shell' crates/bingo-surface-tui/src`
      finds only the frozen voiceless table.
- [ ] Each producer asserts its voice or its `Row` beside the record;
      the stranger test draws a `Row`'s title, badge and action.
- [ ] `screens/` snapshots are unchanged except the model's skill row
      bullet; the diff is pasted.
- [ ] `scripts/tui-smoke.sh` passes, and a tmux drive shows a memory
      row, `/guide` and a model `Skill` call, ctrl+b on a running Bash
      call, a room's switcher columns and `@` completion.
- [ ] All gates of the upgrade protocol, plus the Windows cross-check
      for the TUI and every producer crate touched.

## Non-goals

- `acp.rs`'s tool names, which ADR-0076 removes.
- Print and channels drawing voices or rows; they keep their own rules.
- Any change to what rooms or memory record.

## Risks

- A `Row` placed by a hook lands a frame after its item opens, so a
  memory call can draw as `Read(…)` for one frame. The snapshot tests
  fold whole frames; the tmux drive is the check.
- One extension kind per placed row grows `extensions` by one small
  entry per memory or skill call. Rewind and compaction keep them, as
  they keep any extension; a long session pays bytes, not frames.
- Decided 2026-10-13: the skill row keeps `❖` on both doors through
  `Row.mark` (ADR-0070 §1), rather than falling back to the tool bullet.
- `on_turn(Start)` must see the prompt item in `items`. If it does not,
  the skills hook reads it at `on_event` for the item instead.
- Re-verified at `e415e97e` (2026-10-13): M121's `slot` module and `Row` fields, M122's
  collab readers and `HOLDER`, the producer origins named above by
  symbol, the gate's `GateInput.item`, and the test-only module list in
  `lib.rs`.

M121 already provides `Row.mark` and `transcript/slots.rs`; no slot wire change.
M131 keeps `HookContext` in `hook.rs`; after-tool dispatch is in `chain/tool.rs`.
M122 has `name::PARENT` and record fixtures; add the `room::HOLDER` alias; TUI presentation readers remain to move.
Turn-start hooks receive the prompt in the transcript. Bash foreground calls are
owned by `promote.rs`; publish their placed actions beside the jobs signal.
