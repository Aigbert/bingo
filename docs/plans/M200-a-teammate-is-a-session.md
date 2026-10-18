# M200 — A teammate is a session

ADR-0097 §9, §11; ADR-0086; ADR-0002. Spec: `docs/design/ember/prototype/`:
- `p4d-crew.js`: `enterView` … `nextNeed`, `post`, `seatAgent`, `unseatAgent`, `closeRoom`,
  `mentionPool`, `crewRow`, `SHEETS.crew`;
- `p5-stream.js`: the agent branches of `markSeg`/`verbSeg`/`argSegs`, `theirPostLines`,
  `yourPostLines`, `owedOn`, `mentionSegs`, `typingLines`;
- `p6-dock.js`: `edgeLine`, `needSegs`, `viewSegs`, `crewSegs`, `ambientSegs`, `viewHints`,
  `placeholder`; `p8-page.js`: the window title.

After: M193, M194, M196, M197 and M198.

## Goal

Every teammate and every room is a session of its own, and you look at one at a time. A
teammate's session reads as yours, in its colour, and esc takes you home. Whoever waits on you
leads the footer, and ctrl+g walks you there. A task bingo hands out is one row, and it comes
home as the teammate's own words. In a room, whoever you @ owes you an answer until they give it.

## Bricks

1. **One colour per name** (`bingo-collab`, new `face.rs`).
   - `slot(name, earlier) -> u8` is FNV-1a, `(h >> 8) % 6`. When a teammate listed earlier holds
     that slot, the next free one is taken; past six, the hash slot. The list is the home tree's
     named sessions by creation (`created_at`, then id), rooms excluded.
   - It moves out of the TUI's `face.rs`, so the surface and the mods compute the same ink.
     `fixtures/faces.json` moves with it, gains the sixth hue and the probing cases, and stays
     the wire clients' contract. The sigils go: `◆` now means "waits for you".
   - Inks: a teammate `crew0`–`crew5`, a room `plan`, home `glow`, you `fg`.
   - The prototype's `ROSTER` drops its hand-picked colours for this rule. The crew, agent, room
     and small-crew captures are re-exported before this milestone starts, and the person sees
     them first.
2. **Walking in** (`enterView`, `walkTo`).
   - A view is `Tree::show`. Each session keeps its draft, scroll and dock, and an ask waiting
     there re-arms its 400 ms guard (M197).
   - The view's ink replaces glow in M194's side light, M193's comet and the 900 ms edge flash on
     walking in. Both read one `view_ink`; this milestone adds the teammate's and the room's.
   - A teammate's stream begins with the notice `<name> · <role>` (its definition's
     description). The window title reads `<home title, or bingo> › <name> — <dir>`. A click on
     a teammate's name in a post, an agent row or a crew row walks in.
   - Placeholders: `Message <name>`; `Steer <name>, or queue what comes next`;
     `Answer above, or type to tell <name> something else`; in a room
     `Post to <room>, @ to call on someone`, or `This room is closed`.
3. **Keys away from home** (`viewHints`; `keys.rs` stays the policy table).
   - esc clears the draft, then goes home, and never stops anything: the `Back` rung takes any
     viewed session, and `Interrupt` applies only at home. ctrl+c stops the viewed session's
     turn. ⏎ sends, steers or posts into the viewed session, as M196 does at home.
   - Without a draft, at most three pairs: `esc back`; `ctrl+g answer` when someone waits;
     `ctrl+c stop` while a turn runs; then `@ call on` in an open room, or `/ commands`.
   - With a draft: `⏎ steer`, `tab queue`, `esc clear` while a turn runs; otherwise `⏎ send`
     (`⏎ post` in a room) and `esc clear`.
4. **Who needs you** (`needSegs`, `nextNeed`, `crewSegs`).
   - A teammate's ask waits in its own session and never takes your dock.
   - Everywhere else the footer leads with `◆ ` in amberGlow, then the names in their inks
     (`bingo` in glow for home), or past two the count in amber, then `needs you` or `need you`
     in amber.
   - ctrl+g walks to the next session after this one that waits on you, in the tree's order,
     wrapping; with none, home; at home with nobody waiting, the toast
     `nobody is waiting on you`. At home with someone waiting and no draft, the pairs are
     `ctrl+g answer` and the last two of M193's.
   - At home with nobody waiting, the footer shows one lamp per working teammate (at most four),
     each breathing in its ink over 1.8 s, offset by 450 ms, then `N agents` muted.
5. **The footer away** (`viewSegs`, `ambientSegs`): a teammate's name bold in its ink, its model
   muted and effort dim, then the facts home shows; or a room's name bold in `plan` and each
   seated name in its ink, the writer breathing, or `<room> · closed`.
6. **The crew sheet** (`SHEETS.crew`, `crewRow`; owner `bingo-mod-crew`, on M198's anatomy).
   - `/agents` opens it (`/crew` stays an alias). Head `Crew`, fact `N needs you · N working`
     or `all idle`. Teammates by need (asking, working, idle), then rooms.
   - A teammate's row: its state lamp in its ink (`stateMark`), its name at 11 cells (bold when
     picked), then what it is doing muted (its live turn's prompt, or `writing in <room>`), or
     its role dim. Right: `working <dur>` muted, `needs you` amber, or `idle` dim.
   - A room's row: its purpose dim, and `N seated` or `closed` on the right.
   - Keys `↑↓ who`, `⏎ open`, `tab mention` (teammates only), `esc close`. ⏎ walks in; tab
     closes the sheet and writes `@name `.
   - Today's crew pane, its tabs and cards go. Tasks are M198's sheet.
7. **Agent rows, drawn by the mod.** `bingo-mod-crew` answers `Ledger` for a step that started a
   session, row by row (M194).
   - The surface adds to `Step` the session it started, from `Tree::spawned_by`:
     `spawned: Option<Spawned { name, ink, reported_at }>`. No tool is named.
   - The row: `@` and its name as the verb, both in its ink; the task as the target; the
     spinner in its ink while the session works. Right: `started`, then `reported` once a post
     from that session follows the step, landing glow→ink over 420 ms from `reported_at`.
   - With `bingo-mod-crew` off, mod-ledger draws the agents tool's own look.
8. **Posts** (`theirPostLines`, `yourPostLines`, `typingLines`).
   - An input whose origin names a principal is a post, not your band. Agent reports and room
     posts already carry their author as `origin.principal`. A turn that a post wakes keeps the
     post where your words would be; its steps and answer follow as in M194.
   - Theirs: `▎` in their ink at column 0, the name bold in their ink at 3, the words at 14
     within the measure, fading in from the ground over 260 ms. Consecutive posts by one voice
     join without a blank row or a second name.
   - Yours, in a room: on the `you` ground, `you` dim at 3, the words at 14 with each `@name` in
     its ink (`@all` in `plan`). While anyone named still owes (`bingo_collab::owed`), it reads
     `waiting on <names>` dim on the right, and its side light breathes in the first owed
     name's ink. Unseating someone, or closing the room, drops what they owed.
   - Writing: a seated member who owes and whose session runs a turn reads, at the end of the
     room's stream, `▎` in its faded ink, `<name>`, then `<spinner> writing`. The comet runs in
     its ink.
9. **Rooms** (`seatAgent`, `unseatAgent`, `closeRoom`, `mentionPool`; owner `bingo-rooms`).
   - Notices, from the room's records (`bingo_collab::room`) where they happened: the head
     `<room> · <purpose>`, `<name> joined`, `<name> left`, `closed · <why>`.
   - In a room, `/seat`, `/unseat` and `/close` sit beside `/room`, which stays. `/seat` with no
     name writes `/seat ` and toasts `seat who? <free names>`. `/close` with no reason writes
     `/close ` and toasts `say why the room closes`; the reason is recorded as `why`.
   - In a closed room, posting toasts `this room is closed` and keeps the draft.
   - The palette, told whether the viewed session is a room as it is told the sessions today,
     hides skills there and lists the room commands while the room is open; elsewhere it never
     lists them.
   - `@` in a room lists the seated and `@all`; elsewhere the crew, then files (M196). Naming
     someone not seated toasts `<name> is not in this room · /seat <name> brings them in`; a
     post that names nobody toasts `posted · @ someone to ask for an answer`.
10. **What goes:** the crew strip (`crew.rs`), the sigils, the crew pane's cards and tabs, the
    room grouping in `transcript/` that posts replace, and `Escape::Back`'s idle-only rule.

## Files

- `crates/bingo-collab/src/{face.rs,owed.rs,room.rs}`, and `fixtures/faces.json` (moved).
- `crates/bingo-surface-tui/src/{tree.rs,face.rs,crew.rs,crew_contract.rs,seats.rs,keys.rs,
  keybar.rs,terminal.rs,turns.rs,dock.rs,transcript/}`.
- `crates/bingo-sdk/src/component/ledger.rs` (`Spawned`), and the regenerated schemas.
- `crates/bingo-mod-crew/src/{draw.rs,cards.rs,keys.rs,command.rs,state.rs}`, and new `rows.rs`.
- `crates/bingo-rooms/src/command.rs`, `crates/bingo-mod-palette/src/rows.rs`, `p4d-crew.js`.

## Exit criteria

- [ ] Full frame through the harness: `crew`, `crew-reviewer`, `crew-allowed`, `crew-home`,
      `agent`, `agent-reviewer`, `agent-allowed`, `agent-home`, `room`, `room-mention`,
      `room-waiting`, `room-answered`, `room-home` and `small-crew`.
- [ ] Unit tests: the face fixture (slots, probing, past six); ctrl+g's order and wrap; esc away
      from home never interrupts; what is owed clears when the named member posts and when they
      are unseated; room commands only in an open room; posts join by voice.
- [ ] The agent row is drawn by `bingo-mod-crew` when it is on, and by mod-ledger when it is off.
- [ ] RPC and `--print` are unchanged except the additive `Step.spawned`.
- [ ] Gates and checks as in M192. tmux drive with the fake provider: hire two teammates, walk
      in from the crew sheet, ctrl+g to an ask, answer it, esc home; in a room, seat, post with
      an @, read the answer, unseat, and close.

## Non-goals

Defining or hiring agents; room storage and delivery; teams beyond their members appearing as
teammates; a colour chosen by the person.

## Risks

- **Colours change.** A name hashed `% 5` today reads in another hue. The fixture is the
  contract, and wire clients take the new one.
- **Probing depends on the list.** When a teammate created earlier leaves the tree, its slot
  goes back to the pool. Accepted.
- **A Ledger interception that fails** falls back to mod-ledger's row, never a blank one.
- **IM messages become posts.** A channel's input names its sender as `principal`, so it reads
  as a post in that sender's ink. This is intended, and the Feishu tests pin it.
