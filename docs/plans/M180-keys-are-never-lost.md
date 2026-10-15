# M180 — Keys are never lost

ADR-0002, ADR-0079. A bug fix with a decision in it, so it gets a plan.

## Goal

Input reaches its reader in typed order or stays visible in the composer, never
silently discarded. Input typed elsewhere never enters an ask that takes focus.

## The drop

Previously, `site_keys::drain` discarded queued keys when their site lost focus.
A palette Enter could close the palette and submit `/sessions`, so the remaining
`abc` in `/sessions⏎abc` disappeared. Keys typed after submission but before the
command receipt instead reached the composer, then became the directory's saved
draft rather than its filter. `queue` also discarded keys with no focused site.

The `M141` smoke flake (`/ledger⏎` ending at `/ledge`) is not this drop. Instrumented
runs (45+ rounds) never lost a key there: every key waits for the palette drawing of
the query before it and then a site-key round trip, each 350–900 ms under heavy load,
so the burst outran the smoke's 10 s wait. That is latency; this plan does not reduce it.

## Bricks

1. `run/typeahead.rs` owns keys and pastes in one ordered queue. Each waits with the site it was typed
   at (`None` for the composer). `queue` never drops.
2. A key whose site has gone is addressed again to whatever owns the keyboard now:
   the next site, or the composer. This includes M170's status pane; preserve M173's
   vouching and action origin, and wait for its plugins panel to draw. Wait for a pane
   seat and readable view too: a command receipt can precede its first paint; interrupts bypass this wait.
3. A command line (`/…`) sent to the kernel holds typeahead until its acknowledgement,
   which is when a command's `surface` action has opened its site, or for at most
   `OPENING_WAIT` (1 s); the loop wakes at that deadline.
4. Dialogs: if the keyboard's owner is now an open ask (not set aside), typed-ahead
   inputs are dropped with "Kept N queued inputs out of this dialog" (singular:
   "input"). Count each key or paste once; do not expose content. This also covers
   an already open ask regaining focus and a paste queued at that ask. Record
   actual keyboard ownership, never the identity of an ask set aside. A command
   hold also excludes asks: palette close may restore one before the command ack.
   Queued pastes are always excluded from asks, including ones queued at that ask, to protect
   answers and secret fields. This covers every ask, not only the sealed ones
   (permission, sign-in): a `y` meant for a filter must never approve a tool, and a
   password field must never receive a fragment of a command. Leaving them in the
   composer was rejected: the composer is hidden behind the ask and would carry the
   keys into the next prompt unseen.
5. Escape and ctrl+c clear the command hold as well as queued input, so an active
   turn is interrupted immediately.
6. Applying queued input or its drop notice owes a frame, including after paint
   releases the command deadline; otherwise the edited composer remains invisible.

7. Smoke readiness requires a persistent composer echo after closing Browse.
   The shortcut hint is shared; header absence can match an intermediate redraw.
   Encode closing Escape as CSI 27 u before the probe, avoiding an Alt chord.
   Fixture cleanup uses ctrl+d and waits for process exit; the idle notice case still tests ctrl+c twice. Reply text and the shortcut hint do not prove a turn is idle. The PTY look test waits for settled foreground after the streaming tail.

## Files

- `crates/bingo-surface-tui/src/run/typeahead.rs` (new), `run/typeahead_tests.rs` (new).
- `run/site_keys.rs` sheds focus/blocked/queue/drain; `run.rs` holds `typeahead`; `pane/readiness.rs` checks the pane reader.
- `run/submit.rs` marks a command line; `run/pulling.rs`, `run/site_lifecycle.rs` drain.
- `run/surface.rs` takes the receipt handling out of `run.rs` (size budget).
- `scripts/tui-smoke.sh` verifies composer ownership before `finish`; `crates/bingo/tests/pty/look.rs` waits for settled foreground.

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
- [x] Three passing isolated smoke runs after the readiness fix; earlier 1/3 results retained below.
- [x] M173/M170 rebase: status receipt before paint, pane handoff, ask exclusion, immediate interrupts.
- [ ] Windows cross-check: known missing MSVC C toolchain; see below.

## Non-goals

- Mouse events may act ahead of held input; pointer ordering is outside this round.
- Palette latency per key; the smoke poll count and interval stay unchanged.
- Keys queued at an ask that is then answered are still forgotten with it.
- Stale actions dropped by `release_actions_at` keep their existing notice.

## Risks

- The model picker is not on dev; `/model⏎openai` uses the same command receipt path.
- After a command exceeds the 1 s hold, new input goes to the composer as before M180.
- During the hold the composer does not echo; at most 1 s of keys appear at once.

## Verified

2026-10-15, macOS arm64, `typeahead-fix` from dev ee4ad7dc; shared load 20–270 ("unloaded" means no build of ours).

- `cargo test -p bingo-surface-tui typeahead_tests`: 4 passed. Restoring the discard
  in `drain` fails `keys_typed_behind_a_command_reach_the_site_it_opens` (0 keys
  waiting); re-addressing into dialogs fails `…never_entered_into_it` (an answer went).
- tmux drive (`/sessions` Enter `abc` in one send-keys, own socket, fake provider):
  before, the directory opened with an empty filter; after, `sessions · here · abc`.
- `tui-smoke.sh` ×10, runs 1–6 beside a looping release build: before 6/10 (four exit-hint failures after browse, none at M141); after 10/10.
  This did not demonstrate a ctrl+c drop fixed by M180: the e1bfa6da baseline has the same race. M180 fixed `/sessions⏎abc` and the four review items above; the smoke readiness bug was fixed separately in `3250bdfa`.
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

### Follow-up — 2026-10-15: isolate the browse finish failure

- Scratch copies of only the Browse step: `bash /tmp/m180-investigation.A8CufY/loop.sh <revision> <worktree>`; 20 runs each, socket `m180s`, `-f /dev/null`, temp HOME/data, fake provider, provider env unset. The committed script stayed unchanged during comparison.
- `0f86e36b`: 16/20 passed, 4 failures, load 344–414. Detached worktree `e1bfa6da`: 17/20 passed, 3 failures, load 220–331. These are observed samples under different loads, not a rate comparison.
- All seven failures were `finish` after “ctrl+f browses full blocks”; none reached the Browse-disabled step. Traces: `TERM ctrl+c open=true opening=None → CLEAR → SITE_CTRL_C busy=false`; no queued ctrl+c, no `ARM`, no hint drawing.
- Ctrl+c reached the loop 10–60 ms after send. Largest failed-run poll gap: 287 ms. `EXIT_WINDOW` is 2,000 ms (ordinary notices hold 4 s); successful hints were drawn in 3–77 ms and detected in 6–196 ms. No observed failure was a missed hint window. Baseline run 8, relative to the closing Escape send: shortcut matched +25 ms, Escape drained +51 ms, ctrl+c consumed by Browse +59 ms; no ARM followed.
- The shared shortcut hint matched before Browse closed. Six failures had a normal closing Escape still awaiting its site; one combined Escape plus ctrl+u into ctrl+alt+u during a delayed tty read. Ctrl+c intentionally closes an idle site and is consumed; neither review hold/filter change caused this path.
- A scratch header-disappearance guard passed 19/20, then matched an intermediate redraw before Escape arrived; rejected. Require the persistent `> smoke-ready` composer echo instead, using CSI 27 u for the closing Escape so the probe cannot form an Alt chord. `finish` clears the unsubmitted probe.
- Corrected scratch scenarios: 20/20 passed at load 314–530, with normal arming/exiting in every trace. One run reported `repro ok` then a disposable-home removal race; its scenario passed and its disposable directory was removed during final cleanup. No product change; smoke-only commit `3250bdfa`.
- Temporary source tracing removed in both revisions; smoke binary checked for trace strings. `bash -n scripts/tui-smoke.sh`, fmt, workspace check, Clippy: exit 0; `RUST_TEST_THREADS=4 cargo test --workspace --locked` with provider env unset: 5875 passed, 0 failed, 5 ignored.
- Discipline: `discipline ok`; budget: 417 dependencies, warm core 0s, relink 0, `budget ok` (6 GB soft-size warning); deny: `advisories ok, bans ok, licenses ok, sources ok`.
- After removing instrumentation, `bash scripts/tui-smoke.sh` runs 1, 2, and 3: each exit 0, `tui-smoke ok` (3/3), each on its own socket/temp HOME with provider env unset; poll count/interval and product unchanged.
- Rebase verified — 2026-10-15: onto requested `46843d78`; conflicts in `run.rs`, `site_keys.rs`, `submit.rs` resolved in the introducing commit, preserving M173 vouching/origin/plugins and M170 panes/shared command callers. Status receipt-before-paint regression failed before `45abf8b8`; all 15 typeahead tests now pass. Four provider env vars unset throughout. Sequential fmt and workspace Clippy `-D warnings`: exit 0; `RUST_TEST_THREADS=4 cargo test --workspace --locked`: `6006 passed; 0 failed; 5 ignored`; both `BINGO_UPDATE_SCHEMA=1` generator commands above: 7+1 passed, schema/TypeScript diff empty; discipline: `discipline ok`, run.rs 936 non-test lines, 3 files counted for `impl Run` (2 belong to run::Run); budget: 419 dependencies, warm core 1s, relink 0, `budget ok` (9 GB soft-size warning); deny: `advisories ok, bans ok, licenses ok, sources ok`. Initial PTY foreground failures settled after 176 ms without input: test now waits for settled ink (`6903d011`); one diagnostic rewind timeout subsequently passed in targeted PTY (17/17) and full workspace runs. Initial smoke failed at board cleanup; delayed fake completion reproduced the failure with Ctrl+C reaching busy Composer and interrupting the turn, no queue or armed hint. `fd31d527` waits for persistent process exit via Ctrl+D; idle double-Ctrl+C coverage remains. Temporary tracing removed; clean binary rebuilt; final smoke runs 1/2/3: exit 0, `tui-smoke ok`. Own `tmux -L m180rb -f /dev/null`, temp HOME/data, fake provider, 108x30: single-send `/sessions⏎abc` filtered abc; `/status⏎x` showed pane and composer x; ask plus `/sessions⏎yyyy` excluded four inputs, kept ask unanswered, wrote no file. Windows checks for bingo-surface-tui and bingo: exit 101, known aws-lc-sys missing windows.h/stdlib.h C toolchain. dev advanced externally to `3453487e` during verification; this branch retains the requested base and no other worktree was modified.
