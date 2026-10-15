# M179 — Schedules in one panel

ADR-0093, ADR-0019, ADR-0071, ADR-0082, ADR-0086, ADR-0088. Needs M167,
M170 (the Status card and its rows), M175 (`ago` and the session jump).

## Goal

What fires later, run by the person and not only by the model.
`/schedule` opens a panel: each schedule's id, spec, next fire, enabled,
text and the session it fires into, with the runner that holds the store
on top. `space` pauses or enables, `r` runs now, `d` forgets after a
confirmation, `→` jumps to the schedule's session. The same verbs are
lines: `/schedule add|pause|resume|run|forget`. A pending wake is a row
of the `/status` card again: `wake in 4m · <note>`, counting down while
the card is open, `⏎` prefills `/wake off`. The feature is the built-in
mod `bingo-mod-schedule`, a plugin view; with it off `/schedule` draws
its table and the verbs still work as lines.

```
 schedules · held by this process (pid 4120)              space r d → esc
❯ 01JA4  daily at 09:00   in 14h · tomorrow 09:00   on   triage new issues
  01JA7  every 2h         in 37m                    on   /review dev
  01J9X  once at 10-20    —                         off  ship the notes
 ─────────────────────────────────────────────────────────────────────
 forget 01JA7 "every 2h /review dev"? its session stays.  ⏎ yes  esc

 /status card, when a wake stands:
   wake       in 3m 40s · check the CI run again           ⏎ /wake off
```

## Bricks

1. **One shape of a wake.** `bingo_services::schedule::Pending { at,
   note }` (new `crates/bingo-services/src/schedule.rs`);
   `crates/bingo-schedule/src/wake.rs::payload` (:79) serialises it, so
   the extension and the service cannot disagree. The fixture of the
   journaled payload is unchanged byte for byte (test), and
   `slots_leave_the_wake_record_unplaced` stays green.
2. **The service, contract first.** `schedule.rs`: `list() -> Listing {
   entries: [Row { id, spec, next_fire, enabled, text, session }],
   holder }`, `pending(session) -> Option<Pending>`, `add(spec, text,
   cwd, mode)`, `pause(id)`, `resume(id)`, `run(id)`, `forget(id)`; id
   prefixes resolved as `tools/forget.rs` does. `find(host)`
   native-or-wire; fixtures `crates/bingo-services/fixtures/schedule/`
   before the implementation.
3. **Rows, pure.** `crates/bingo-schedule/src/render.rs` grows `row_of(
   entry, tz, now, session) -> Row` beside the table's `row`, one
   derivation for both: `next_fire` through `Entry::next_fire`, `session`
   the id whose summary key is `entry.key()` (looked up in one
   `HostApi::sessions` listing, never by `open`, which would create it).
4. **Verbs in the plugin.** `crates/bingo-schedule/src/command/
   schedule.rs` parses `add <spec> -- <text>`, `pause|resume|run|forget
   <id>`; `ArgSpec` values from the store. `add` writes the entry
   `tools/create.rs` writes, with this session's cwd and permission mode.
   `run` dispatches through the runner's delivery (`runner.rs`,
   `Delivery::Wake` on `schedule/<id>`) whether or not this process holds
   the store, and stamps `last_fired`. `pause`/`resume` flip `enabled`
   through `store.rs::save`. `forget` is `store.rs::delete`. Bare
   `/schedule` unchanged. Test per verb in a temp data dir, incl. an
   ambiguous prefix and `run` on a disabled entry (refused: resume first).
5. **The wake row on the card.** `crates/bingo-mod-status/src/rows.rs`
   gains `wake(pending, now) -> Option<Row>` (tone `warn`, `⏎` →
   `Composer` prefill `/wake off`), reading `bingo_services::schedule::
   find(host)?.pending(session)` when the card opens and on each
   `Event::Extension` from `bingo.schedule`; the countdown uses the frame
   clock and the card's existing redraw. Table test: none, 4m, past due.
6. **Keys.** `crates/bingo-mod-schedule/src/keys.rs` through `onKey`:
   `space` → `/schedule pause|resume <id>`, `r` → `/schedule run <id>`,
   `d` → confirmation then `/schedule forget <id>`, `→` → `{"view":
   session}` or the toast `has not fired yet`. Table test.
7. **Drawing.** `draw.rs`: holder line (`held by this process`,
   `dormant — held by another bingo`, `no runner`, as `lock.rs::holder`),
   rows with `next fire` as `in 37m` by `bingo_sdk::until` (in `ago.rs`
   beside M175's `ago`, same units), a disabled row dim. Snapshots at 108×30 and 80×24: held, dormant, empty, confirming.
8. **Guide.** `guide.md` (<200 lines) by `guide.rs`, as in
   `bingo-mod-palette`; `crates/bingo-schedule/src/guide.md` names the
   new lines.

## Files

- `crates/bingo-services/src/{lib.rs,schedule.rs}` and fixtures;
  `crates/bingo-sdk/src/{ago.rs,lib.rs}` (`until`).
- `crates/bingo-schedule/src/{lib.rs,wake.rs,render.rs,store.rs,
  runner.rs,command/schedule.rs,guide.md}`.
- `crates/bingo-mod-status/src/{rows.rs,live.rs}` (wake row).
- New `crates/bingo-mod-schedule/src/{lib.rs,keys.rs,draw.rs,guide.rs,
  guide.md}`; listed in `crates/bingo/src/main.rs`; +1 member in
  `scripts/budget.toml`, no dependency.
- `docs/adr/0019-schedules.md` gains the amendment line for §6 and §8.

## Exit criteria

- [ ] Service fixtures; payload byte-for-byte test; rows and keys
      tables; a test per verb, refusals included.
- [ ] `TestBackend`: `/schedule` opens the panel; `space` dims a row and
      `/schedule` table agrees; `r` opens a turn on `schedule/<id>`; `d`
      then `⏎` removes the row; `→` views the session.
- [ ] `TestBackend`: the fake provider calls `Wake{after:"4m"}`; `/status`
      shows the wake row counting down; `⏎` prefills `/wake off`; after
      `/wake off` the row is gone.
- [ ] Two processes on one data dir: the second's panel says dormant;
      its `r` still fires once (black-box, temp HOME).
- [ ] `enabledPlugins: {"bingo.mod.schedule": false}`: the table draws,
      the lines work; the wake row still shows (it is the status mod's).
- [ ] tmux drive on an isolated socket, temp HOME, fake provider: add,
      pause, run, forget, and a wake on the card; captures pasted.
- [ ] `bash scripts/tui-smoke.sh`; `cargo fmt --all -- --check`; `cargo
      check --workspace --all-targets --locked`; `cargo clippy --workspace
      --all-targets --locked -- -D warnings`; `cargo test --workspace
      --locked`; `scripts/check_discipline.sh`; `scripts/budget.sh`;
      `cargo deny check`.
- [ ] `cargo check -p bingo-schedule -p bingo-mod-schedule -p
      bingo-mod-status --all-targets --target x86_64-pc-windows-msvc
      --locked` (the lock and the clock).

## In-flight dependencies

- M166: none.
- M170: `bingo-mod-status` rows, live redraw and `Composer`; land after.
- M173: `bingo-schedule` gains a service, so its switch says `off at next
  start`; `bingo-mod-schedule` switches now.
- M174: `d`'s confirmation resolves against the current drawing; a row
  that fired and was forgotten meanwhile forgets nothing twice.

## Non-goals

Cron expressions; editing a schedule's spec or text in place (forget and
add); an OS daemon; wakes in the panel (they are the session's, on the
card and `/wake`).

## Risks

- `run` from a process that does not hold the store can race the
  holder's fire in the same minute; `last_fired` is written by both, the
  later one wins, and at most one extra turn runs. Stated in the reply.
- The wake moving from always-on to the card weakens ADR-0019 §8's
  passive visibility; the note still arrives under `wake`, and `/wake`
  stays. Recorded in ADR-0093.
- `add` with `--` splitting spec and text is new grammar; a spec without
  `--` is `INVALID_INPUT` with the three forms of ADR-0019 §2 spelled.
