# M179 — Schedules in one panel

ADR-0093, ADR-0019, ADR-0071, ADR-0082, ADR-0086, ADR-0088; reconciled with ADR-0094.
Prerequisites: M167, M170, M173, M175. Existing `m179-schedules` rebased with `git rebase dev`
onto `7a759d8a`; no new branch, merge, push or PR.

## Goal

`/schedule` opens the built-in panel: id, spec, next fire, enabled, by, text and destination.
Space pauses/resumes, r runs now, c confirms ownership, d forgets, → visits the session.
Both c and d require confirmation. The same verbs work as lines; disabling the mod retains the table.
A pending wake appears in Status, counts down, and prefills `/wake off`.

## Bricks (implemented in order)

1. **One wake shape.** `bingo_services::schedule::Pending { at, note }` supplies the unchanged
   journal payload, including empty slots. Wakes remain process-local.
2. **Service, contracts first.** Fixtures pin Pending, Listing and mutation refusals. Rows include
   by and explicit mode; Listing includes trouble. Native-or-wire list/pending/pause/resume/run/forget.
   Add/confirm remain on dev's command path, avoiding a second way to grant consent.
3. **Pure rows.** `row_of` derives table/service rows from Entry; one sessions listing resolves
   `entry.key()`. Reading never opens a session; relative time belongs to the drawing.
4. **Verbs.** Keep dev's add/confirm grammar and asked-line predicate. Pause/resume/run/forget
   share it and the prefix resolver. Run uses the runner's signed delivery regardless of the claim.
   It waits for acknowledgement, then stamps last_fired; disabled run is refused.
5. **Wake on Status.** Read pending on draw; invalidate on extensions/live ticks. Place near
   context so 80×24 shows it. Enter calls Composer to prefill the line.
6. **Keys.** Pure mapping and full-row confirmation; stale modes/provenance/text cannot be accepted.
   Missing destination says `has not fired yet`. All client replies require the pane owner's drawnBy.
7. **Drawing.** Shared ui.render/Pane path; holder, by, deadline, dim disabled rows, explicit mode,
   trouble badge and scrolling. Ten TestBackend snapshots at 108×30 and 80×24.
8. **Guide.** Owning-crate guide.md Pages registered in guide.rs, each below 200 lines (ADR-0054).

## Files

- bingo-services: contract and fixtures; bingo-sdk: generic `until` beside `ago` only.
- bingo-schedule: service, verbs, rows, payload, shared signed delivery, concurrent runner, guide/tests.
- bingo-mod-status: wake row, prefill and invalidation; new bingo-mod-schedule: pane and owner keys.
- TUI: generic pane reply handling and tests; binary registration and real RPC/CLI tests.
- Cargo manifests/lock and budget: keep both dev's models mod and schedule mod; max_dependencies 421.
- ADR-0019/0093 amendments; gateway test spawn/probe isolation retained from the original verification.

## Decisions — 2026-10-16

- ADR-0094 owns add/confirm: by person, explicit `as <mode>` only, never copied from session mode.
  Every mutation uses dev's `render::asked()` predicate and PermissionDenied refusal. Wire mutations
  clear that context. Model tools keep their existing permission-gated contracts and tool provenance.
- c confirms a model/unconfirmed row through `/schedule confirm <id>` after showing its explicit mode.
  The built-in's vouched press returns the line; the client submits it as asked. A plugin cannot
  invoke the service or fabricate a button to grant consent. d uses the same stale-row check.
- Run and automatic fire share dev's signing and refusal notice/trouble. Each automatic entry has
  one independent in-flight attempt, bounded at 30 seconds. Failed delivery remains unspent and
  retries after a rescan, so one stalled entry cannot block the rest or spin. A rejected signed line
  remains unspent. Before stamping, reread the entry and skip it if changed/deleted.
- Generic pane replies require the pressed owner's drawnBy for run, view and message alike.
  Existing destination sessions outside the current tree are opened by id.
- RFC3339 at/next_fire stay on the wire; row_of needs no now. Full row equality pins confirmation;
  live ticks cover other processes/hand edits. Disabling the mod clears its panes.
- Existing jiff/tokio edges supply clock/redraw; no new resolved external dependency or allowlist change.
- TestBackend controls the clock/styles; real RPC covers fake Wake, delivery/navigation and confirmation.
  This replaces timing-sensitive UI waits in the original combined test. Guides never use docs/guide/.

## Exit criteria

- [x] Fixtures, byte-for-byte wake payload, row/key tables, all verbs and unasked/disabled/ambiguous refusals.
- [x] Panel/Table agreement, by column, ownership confirmation with stale-mode rejection, and forget.
- [x] Fake Wake(4m), Status countdown, Composer prefill and cancellation; original RPC/tmux coverage retained.
- [x] Two-process dormant panel can run once; original run/navigation and disabled-mod fallback remain covered.
- [x] Failed acknowledgement stays unspent; stalled entry does not block another; refusal notice is shared.
- [x] Foreign/unvouched pane messages and session jumps have no effect.
- [x] New isolated tmux drive: add, confirm, pause, resume, forget at 108×30 and 80×24.
- [x] Final sequential native gates pass; output recorded below.
- [ ] All Windows checks pass: tui/bingo/gateway remain deferred by the permitted C-toolchain
  failures; the other five crates pass. Individual outputs below.

## Non-goals and risks

No cron, in-place editing, daemon, or wakes in the schedule panel. Manual and automatic fire can race
and add one extra turn; the reply says so. Wakes are visible on demand. A delivery timeout is an
unknown acceptance result, so a later retry can duplicate a turn whose acknowledgement was lost.

## Verified — 2026-10-15

Original branch: generators passed without generated diff; focused RPC tests: 4 passed.
Native gates: fmt/check/Clippy passed; workspace 6011 passed, 0 failed, 5 ignored; discipline ok;
budget 420/420, core 0s, relink 0; deny advisories/bans/licenses/sources ok; tui-smoke ok.
Windows: sdk/services/schedule/mod-schedule/mod-status passed; tui/bingo/gateway deferred at aws-lc-sys.
The gateway probe/fork race was isolated without serializing resident children. Final cargo clean ran.
Original tmux `-L m179 -f /dev/null`, fake provider, temp HOME, both sizes:

```text
❯ 4b3wp8bm  every 2h  —  off  review build
 forget 4b3wp8bm "every 2h review build"? its session stays.  ⏎ yes  esc
❯ 9hqq2tvc  every 2h  in 2h  on  review build
 ❯ wake       in 3m 57s · check CI again                 ⏎ /wake off
│ > /wake off
 the wake set for 2026-10-15 20:10 is off
```

## Verified — 2026-10-16

Every run unsets OPENAI_API_KEY, OPENAI_BASE_URL, ANTHROPIC_API_KEY, ANTHROPIC_BASE_URL.
Only fake/local fixtures, temporary homes/data, and owned tmux sockets; no live credential tests.
Focused tests: services 20 passed; owner 130 passed; panel keys 4 passed; RPC panel 4 passed.
TestBackend snapshots include ownership confirmation with all three by labels at both sizes.
Tmux `-L m179r -f /dev/null`: fake ScheduleCreate produced the model entry, the person added another,
c/Enter recorded by person, space paused/resumed, d/Enter deleted only the selected entry. Captures:

```text
❯ sy0mst0t  every 2h  in 1h  on  model  model review
 confirm sy0mst0t as yours, mode acceptEdits?  ⏎ yes  esc
❯ sy0mst0t  every 2h  —  off  you  model review
❯ sy0mst0t  every 2h  in 1h  on  you  model review
❯ qwb1zq7j  every 3h  in 2h  on  you  person review
```

The tmux server was stopped. Captures/logs stay outside commits; owned temporary data is removed
and cargo clean is run at completion. Commands below ran one at a time.

Generators: `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk -p bingo-plugin-rpc -p bingo-surface-rpc --lib`,
then `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk --test record --locked`: all passed, no generated diff.
Windows uses `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc --locked`.

```text
cargo fmt --all -- --check: exit 0
cargo check --workspace --all-targets --locked: Finished dev profile (0.52s)
cargo clippy --workspace --all-targets --locked -- -D warnings: Finished dev profile (12.32s)
cargo test --workspace --locked: 6157 passed; 0 failed; 6 ignored (194 groups)
bash scripts/check_discipline.sh: discipline ok; advisory warnings remain
bash scripts/budget.sh: dependencies 421/421; core 1s; relink 0; budget ok
  target/debug 8 GB exceeds soft max 5; final cargo clean removes build output
cargo deny check: advisories ok, bans ok, licenses ok, sources ok
bash scripts/tui-smoke.sh: tui-smoke ok
Windows bingo-sdk: Finished dev profile (5.88s); exit 0
Windows bingo-services: Finished dev profile (8.25s); exit 0
Windows bingo-schedule: Finished dev profile (6.13s); exit 0
Windows bingo-mod-schedule: Finished dev profile (0.34s); exit 0
Windows bingo-mod-status: Finished dev profile (0.47s); exit 0
Windows bingo-surface-tui: aws-lc-sys build failed; stdlib.h/windows.h missing; exit 101
Windows bingo: aws-lc-sys and rquickjs-sys build failed; headers/MSVC C flags unavailable; exit 101
Windows bingo-gateway: aws-lc-sys build failed; stdlib.h/windows.h missing; exit 101
```
