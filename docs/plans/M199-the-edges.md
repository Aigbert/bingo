# M199 — The edges

ADR-0097 §1, §7, §10; ADR-0095 §2 (a toast is never replayed). Spec: `docs/design/ember/`:
- `p5-stream.js`: `welcomeLines`, `markCells`, `retryRow`, `errorLines`, `noticeLines`,
  `shellLines`;
- `p6-dock.js`: `exitFrame`;
- `p7-input.js`: `welcomeKey`, `shellKey`;
- `p4c-act.js`: `backgroundBash`, `jobDone`, `signedIn`, `afterSignIn`, `compact`, `wakeNow`.

After: M193, M194 and M196. The sweep (brick 8) runs last, after M198 and M200 merge.

## Goal

The moments at the edges read as the middle does:
- the first screen before you speak;
- the receipt left in the shell;
- a rate limit, an expired sign-in, a full context, a job finishing in the background, and
  bingo waking itself.

Then every one of the 105 captures passes, and what the old look left behind is deleted.

## Bricks

1. **Welcome** (`welcomeLines`, `markCells`, `welcomeKey`).
   - **The wordmark:** `bingo` in half blocks (`MARK`; `MARK_ASCII` under ASCII), centred in a
     52-column block, 42% down.
   - **Its motion:** over 900 ms it is written left to right behind a glow caret `▍`. The
     newest 6 columns fade glow→fg, and embers rise from the caret (`embers.rs`).
   - **Below it:**
     - model `·` effort `·` mode in its colour;
     - the directory on its branch `·` `N changed`;
     - `recent` with up to three sessions, the time dim on the right.
   - **Keys:** ↑ from an empty input enters the list, ↓ past its end leaves it, ⏎ resumes, and
     any other key goes to the input.
   - **Sending:** the first message dissolves the welcome. Reduced motion shows it settled.
   - **What it replaces:** `opening.rs`'s storyboard, and today's welcome text.
2. **The receipt** (`exitFrame`).
   - Leaving the terminal prints three lines to stdout and nothing else:
     - `● bingo · <title>`: the lamp good, bingo bold, the title muted;
     - `N turns · <duration> · edited N files`, dim;
     - `resume with bingo --resume <id>`, the command in `code`.
   - A session with no turns prints nothing.
   - The receipt always names the home session, as exiting walks home first.
   - It replaces `way_back`. The last screenful stays behind `print_on_exit`, printed above the
     receipt when that is on.
   - `bingo --resume <id>` opens the session as it was and toasts `✓ resumed · <title>`.
3. **Retry** (`retryRow`).
   - A retrying turn shows one row under its steps: the spinner amber, the reason amber,
     `retrying in Ns` muted, and `attempt N of M` dim on the right. The edge line is M193's.
   - The fold keeps `TurnRetrying`'s `delay_ms` and `reason`. Today they are dropped, and the
     countdown is derived at render time from the frame's instant.
   - The row leaves when the turn moves on.
4. **Signed out** (`errorLines`, `signedIn`, `afterSignIn`).
   - An auth failure ends the turn with two rows:
     - `signed out of <provider>` bad `·` the message;
     - `/login` glow ` signs you in again · your message is kept`.
   - The message you sent is kept. `/login` opens the sign-in sheet (the sealed `SignIn` site
     on M198's anatomy, `pick: false`): the device code bold, a spinner, and the URL underlined.
   - On success it toasts `✓ signed in · sending your message again` and sends the message.
5. **Context.**
   - The footer reads `context N%` from 70%, and amber from 80% (M193 draws it).
   - `/compact` leaves a notice `── compacted · context 86% → 18% ──`. Notices are faint rules
     with a muted label that fades in from glow over 400 ms (`noticeLines`).
6. **Background** (`backgroundBash`, `jobDone`).
   - A backgrounded step reads `in background` on the right, and the footer counts `N shells`.
   - When the job ends, a notice reads
     `── ✓ <command> finished in the background · <outcome> ──`, with the toast
     `✓ background job finished · <outcome>`. A failed job reads `×` bad.
7. **Waking** (`wakeNow`).
   - A wake step reads `∞ wake  in 5m … set`. While one is pending, the footer reads
     `wakes in X`, counting down at render time from the wake's instant.
   - The schedule plugin places that fact again through its slot.
   - When it fires, the new turn's band begins with `wake-up` dim before the prompt.
8. **The sweep.**
   - Every capture in `index.json` passes full frame through the harness, in one test module
     that lists them all.
   - Each capture is owned by the milestone that made it pass. This module is the record that
     none regressed.
   - Delete what nothing calls any more: old theme and glyph accessors, `motion.rs`'s cues that
     Ember's motion replaced, `activity.rs`'s leftovers, `fold.rs` where only turns used it,
     and snapshots no test reads. Each deletion is listed in Verified.
   - A deadline says whether it changes a part's size or only its ink, and a part clipped to no
     room keeps only the former (ADR-0097 §10). Tested: the crew lamp in a footer too narrow
     for it schedules nothing; a clipped countdown still wakes when its text shrinks.
   - `docs/design/ember/README.md` says the shipped TUI follows the captures, and how to
     re-export them.

## Files

- `crates/bingo-surface-tui/src/{welcome.rs,opening.rs,opening/,embers.rs,lib.rs,run/,transcript/,
  activity.rs,motion.rs,theme/,fold.rs,snapshots/}`.
- `crates/bingo-sdk/src/state.rs` (the retry fold).
- `crates/bingo-schedule` (the wake fact), `crates/bingo-tool-bash/src/jobs.rs` (the job
  notice), and `crates/bingo` (`--resume`'s toast).
- Assigned subset: SDK props/metadata, core compaction recording, context location facts,
  mod-sessions recent lookup, TUI edge modules and PTY/capture/drive tests; no dependency added.

## Exit criteria

- [ ] Full frame through the harness: `welcome`, `welcome-recent`, `welcome-sent`, `exit-back`,
      `retry`, `retry-recovered`, `signedout`, `signedout-login`, `context`,
      `context-compacted`, `wake`, `wake-later`, `background`, `background-notified` and
      `narrow`.
- [ ] `exit`: its receipt rows match byte for byte in a pty black-box test. The shell's prompt
      rows are the shell's, not bingo's.
- [ ] The sweep module passes all 105 captures on the merged dev, after M198 and M200.
- [ ] A fold test: `TurnRetrying` keeps its delay and reason, and an old journal without them
      still folds.
- [ ] Gates and checks as in M192. tmux drive: a fresh start, ↑ into recent and ⏎, a retry
      with the fake provider's rate limit, `/compact`, ctrl+b then the job's end, and two
      ctrl+c with the receipt read from the pane.

## Non-goals

Sign-in flows other than the device code; changing how compaction works; scheduling beyond what
`/schedule` and `/wake` do today.

## Risks

- **The receipt in stdout.** Scripts that read bingo's stdout after a TUI run see three new
  lines. They print only when stdout is a terminal.
- **The sweep finds a regression late.** Its owner fixes it, never the sweep, and the fix is
  merged before the sweep is ticked.
- **Welcome ownership (ADR-0086), decided.** The drawing stays the surface's `Engine` answer (the
  motion needs cells View lacks); mod-sessions supplies recent sessions through `WelcomeProps`;
  an outside mod replaces the whole welcome through the `Welcome` site.

## Verified
Bricks 1–7; brick 8 waits for M200. Evidence in `docs/verification/m199/checks.txt`.
- [x] Full frames — `edges_full_frames_match_the_prototype ... ok`: welcome, welcome-recent,
  welcome-sent, exit-back, retry, retry-recovered, context, context-compacted, background,
  background-notified, narrow, signedout, signedout-login, wake, wake-later.
- [x] `exit` — `exit_receipt_matches_the_prototype_rows_byte_for_byte ... ok`, waiting on
  the resume toast again; `a_notice_first_drawn_late_still_holds_from_that_frame ... ok`.
- [ ] Sweep — brick 8, after M200.
- [x] Retry fold, compaction window, welcome site — round 2 tests still `ok`.
- [x] Signed out — `an_expired_sign_in_says_what_happened_then_what_to_do`,
  `a_bare_login_after_a_refused_turn_signs_in_again ... ok`.
- [x] Waking — `the_journal_payload_carries_the_wake_and_its_countdown`,
  `a_countdown_wakes_only_when_its_words_change ... ok`.
- [x] Gates — fmt 0; check, clippy `Finished`; `cargo test --workspace --locked` 135 result
  lines, `6659 passed; 0 failed`; discipline ok; budget `423 (max 423)`; deny all `ok`.
- [x] Commits build — `rebase -x 'cargo fmt … && cargo check …' 6a45057e`: 18 `Executing`,
  `Successfully rebased`. dev moved to M195 after the rebase; the next rebase follows it.
- [x] Drive — `drive.py` adds `/login` after a fake auth failure and `/wake`:
  `PASS 100x32`, `PASS 80x24`, both `CLEAN`; footer `fake-1 · wakes in 6s`.
- [ ] Windows — sdk, core, context, skills, mod-sessions, mod-models, provider-fake, schedule
  exit 0; surface-tui and bin owed to CI (aws-lc-sys and tree-sitter C builds).
Departures, each where the prototype or a real run decided:
- Prototype wins: retry-recovered at 4700 ms shows another retry; no edited files at zero;
  a failed turn has no tail gap (M198's gap now skips it).
- Only a device-code login docks; paste and browser stay a card. The fake provider gained a
  scripted device login; core `/login` with no name signs in to the session's provider.
- bingo-mod-models opened its picker on a bare `/login`; it now passes a signed-out one
  through. Not signed out, its fallback still runs the login before the picker (owed).
- Not drawn: the prototype's 900 ms "signed in to X" line inside the sheet. Drawn beyond it:
  the kernel's `login(fake)` receipt row in the transcript.
- `/wake` is folded into `/schedule` in the palette; the prototype lists it as its own row
  (palette owner M196). The wake captures have no keys: the wake fires on its own.
- The schedule journal payload gained the `wakes` status slot (additive); the countdown
  changes each second under an hour.
