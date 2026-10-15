# M180 — Keys are never lost

ADR-0002, ADR-0079. A bug fix with a decision in it, so it gets a plan.

## Goal

A key a person types reaches the place that reads it, in the order typed, or it
stays visible in the composer. No path discards one silently. One exception is
deliberate: input typed elsewhere is never entered into an ask that takes focus.

## The drop

A key typed at a site (palette, directory, browse, an ask) while the site was still
drawing or still judging the key before it waited in `Run::site_queue`, tagged with
the site's id. `site_keys::drain` let them go in order, and a key whose site was no
longer the keyboard's owner was popped and discarded:

```
run.rs  terminal_event → site_keys::blocked → site_keys::queue (tag = palette id)
run.rs  ack(Enter)     → site_keys::answered → palette::result {"run":"/sessions"}
                       → palette::close + input::command_line → Submit(Text)
                       → site_keys::drain: focus ≠ palette → pop_front, continue
```

So `/sessions⏎abc` (or `/model⏎openai`) typed in one burst lost `abc`: the keys
waited for the palette's verdict on Enter, and that verdict closed the palette.
Keys typed after the line went out and before its acknowledgement were not queued
at all: they went to the composer, and the opened directory then took the composer
as its draft, so its filter started empty. `site_keys::queue` also dropped a key
outright when nothing owned the keyboard.

The `M141` smoke flake (`/ledger⏎` ending at `/ledge`) is not this drop. Instrumented
runs (45+ rounds) never lost a key there: every key waits for the palette drawing of
the query before it and then a site-key round trip, each 350–900 ms under heavy load,
so the burst outran the smoke's 10 s wait. That is latency; this plan does not reduce it.

## Bricks

1. `run/typeahead.rs` owns keys and pastes in one ordered queue. Each waits with the site it was typed
   at (`None` for the composer). `queue` never drops.
2. A key whose site has gone is addressed again to whatever owns the keyboard now:
   the next site, or the composer.
3. A command line (`/…`) sent to the kernel holds typeahead until its acknowledgement,
   which is when a command's `surface` action has opened its site, or for at most
   `OPENING_WAIT` (1 s); the loop wakes at that deadline.
4. Dialogs: if the keyboard's owner is now an open ask (not set aside), typed-ahead
   inputs are dropped with "Kept N queued inputs out of this dialog" (singular:
   "input"). Count each key or paste once; do not expose content. This also covers
   an already open ask regaining focus and a paste queued at that ask. Record
   actual keyboard ownership, never the identity of an ask set aside. A command
   hold also excludes asks: palette close may restore one before the command ack.
   Queued pastes
   are always excluded from asks, including ones queued at that ask, to protect
   answers and secret fields. This covers every ask, not only the sealed ones
   (permission, sign-in): a `y` meant for a filter must never approve a tool, and a
   password field must never receive a fragment of a command. Leaving them in the
   composer was rejected: the composer is hidden behind the ask and would carry the
   keys into the next prompt unseen.
5. Escape and ctrl+c clear the command hold as well as queued input, so an active
   turn is interrupted immediately.
6. Applying queued input or its drop notice owes a frame, including after paint
   releases the command deadline; otherwise the edited composer remains invisible.

## Files

- `crates/bingo-surface-tui/src/run/typeahead.rs` (new), `run/typeahead_tests.rs` (new).
- `run/site_keys.rs` sheds focus/blocked/queue/drain; `run.rs` holds `typeahead`.
- `run/submit.rs` marks a command line; `run/pulling.rs`, `run/site_lifecycle.rs` drain.
- `run/surface.rs` takes the receipt handling out of `run.rs` (size budget).

## Exit criteria

- [x] `/ledger⏎` fed as one batch before the palette's first drawing runs `/ledger`.
- [x] `/sessions⏎abc` through the kernel: `abc` waits for the command's answer and
      lands in the directory's filter, each key judged against the filter before it.
- [x] Keys waiting on a palette that closes as a permission or question opens are
      not entered: no answer, no extra action, the notice shows the count.
- [x] Keys wait out a slow command line for no longer than `OPENING_WAIT`.
- [x] Mutation checks: restoring the discard fails the second test; re-addressing
      into dialogs fails the third.
- [x] Isolated tmux drive of `/sessions⏎abc`, before and after.
- [x] Gates and ten smoke runs, five or more beside a parallel cargo build.

- [x] Esc and ctrl+c during a busy command hold interrupt now and leave no queue.
- [x] Deadline release during paint schedules a following frame showing `zq`.
- [x] Held `ab`, paste `X`, `cd` arrive as `abXcd`; asks reject queued pastes.
- [x] Permission ask, `/nosuch⏎y`: the ask remains unanswered, with a truthful count.
- [x] Review native non-smoke gates and regenerated schemas/TypeScript.
- [ ] Three passing isolated smoke runs: 1/3; runs 1 and 3 hit the browse-close timeout.
- [ ] Windows cross-check: known missing MSVC C toolchain; see below.

## Non-goals

- Mouse events during a command hold may act ahead of queued input; pointer
  ordering is outside this review round.
- Palette latency per key (a drawing plus a round trip). The smoke's 10 s wait is
  not raised here either; the numbers are in Verified.
- Keys queued at an ask that is then answered are still forgotten with it.
- Stale actions dropped by `release_actions_at` keep their existing notice.

## Risks

- The model picker is not on dev. It opens the same way (a command whose receipt's
  `surface` action opens a site), so `/model⏎openai` takes this path unchanged.
- A command that takes longer than 1 s to acknowledge releases the keyboard; what
  is typed then goes to the composer, as before M180, and nothing is lost.
- During the hold the composer does not echo; at most 1 s of keys appear at once.

## Verified

2026-10-15, macOS arm64, `typeahead-fix` from dev ee4ad7dc. The host was shared:
load averages 20–270 throughout, so "unloaded" below means only no build of ours.

- `cargo test -p bingo-surface-tui typeahead_tests`: 4 passed. Restoring the discard
  in `drain` fails `keys_typed_behind_a_command_reach_the_site_it_opens` (0 keys
  waiting); re-addressing into dialogs fails `…never_entered_into_it` (an answer went).
- tmux drive (`/sessions` Enter `abc` in one send-keys, own socket, fake provider):
  before, the directory opened with an empty filter; after, `sessions · here · abc`.
- `tui-smoke.sh` ×10, runs 1–6 beside a looping release build in another target dir:
  before 6/10 (all four failures at `finish` after the ctrl+f browse step, waiting for
  the ctrl+c hint; none at M141); after 10/10. The earlier failures fit a ctrl+c
  queued at the closing browse site and then discarded, the drop this plan removes.
- `cargo fmt --all -- --check` 0; `cargo clippy --workspace --all-targets --locked
  -- -D warnings` 0; `RUST_TEST_THREADS=4 cargo test --workspace --locked` 0
  (5868 passed, 0 failed, 5 ignored); `scripts/check_discipline.sh` ok (run.rs 982
  non-test lines); `scripts/budget.sh` ok; `cargo deny check` ok.
- `cargo check -p bingo-surface-tui --all-targets --target x86_64-pc-windows-msvc`:
  stops in `aws-lc-sys` v0.44.0's build script (no MSVC toolchain here), the known
  deferral; CI's windows job is the check.

## Verified — 2026-10-15

- Before fixes, `cargo test -p bingo-surface-tui typeahead_tests --locked`: `4 passed; 7 failed`; all seven new regressions failed at their intended assertions.
- After fixes, the same command: `11 passed; 0 failed`; includes busy Esc/ctrl+c, TestBackend redraw, ordered paste into composer/directory, three ask kinds, and both palette paths.
- `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-sdk -p bingo-plugin-rpc -p bingo-surface-rpc committed --locked`: `7 passed; 0 failed`; no generated diff.
- `BINGO_UPDATE_SCHEMA=1 cargo test -p bingo-mods schema_and_types_are_generated_from_the_same_rows --locked`: `1 passed; 0 failed`; no generated diff.
- `cargo fmt --all -- --check`: exit 0.
- `cargo check --workspace --all-targets --locked`: `Finished dev profile` (32.58s), exit 0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: `Finished dev profile` (17.56s), exit 0.
- Initial workspace tests failed at OpenAI's fixture key-source assertion (inherited environment); after removing provider env, a scheduler shutdown lock reacquisition failed (`WouldBlock`). Neither is changed here.
- `env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u OPENAI_BASE_URL -u ANTHROPIC_BASE_URL RUST_TEST_THREADS=4 cargo test --workspace --locked`: `5875 passed; 0 failed; 5 ignored`, exit 0; both earlier failures passed.
- `bash scripts/check_discipline.sh`: `discipline ok`; run.rs 982 non-test lines; `impl Run` in 2 files. Existing advisory warnings remain.
- `bash scripts/budget.sh`: `417` dependencies, warm core `1s`, relink `0`, `budget ok`; target/debug `8 GB` exceeds the soft 5 GB limit, to be cleaned at the end.
- `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.
- `bash scripts/tui-smoke.sh` run 1: exit 1, `timed out waiting for: press ctrl+c again to exit`, after browse close, matching the earlier recorded intermittent failure.
- `bash scripts/tui-smoke.sh` run 2: exit 0, `tui-smoke ok`.
- `bash scripts/tui-smoke.sh` run 3: exit 1, same browse-close timeout. All three used separate own sockets (`-f /dev/null`), temporary HOME/data, fake provider, and the four provider env variables unset; timeouts unchanged.
- `cargo check -p bingo-surface-tui --all-targets --target x86_64-pc-windows-msvc`: exit 101 in `aws-lc-sys v0.44.0`.
  `fatal error: 'windows.h' file not found` (also `stdlib.h`): known C toolchain deferral; Windows CI remains required.
