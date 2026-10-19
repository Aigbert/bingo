# M196 part 1 — round 3

Base: `fec21d36`; round-2 head: `a50283c0`. No new dependency or package.

## Review items and evidence

1.1 **Drain proof** — `badf16b6`, `run/withdraw/tests.rs`.
`stopping_withdraws_two_lines_before_completion_can_drain_them` reads a fresh
Host snapshot after `TurnCompleted`, then folds through that mailbox snapshot's
sequence. Only then does it assert one start, an empty queue, one user item and
`first\nsecond` in the composer. The restored test passes. Mutation check:
replace the Esc/withdrawal/reply block with `run.interrupt(attached.session.clone())`;
run the test alone: exit 101, `left: 2`, `right: 1` at the start-count assertion.
The mutation was restored before the full TUI run (`1435 passed; 0 failed`).

1.2 **Streaming stop proof** — `badf16b6`, `run/acting/tests/{proofs,streaming}.rs`.
`stop_answer_matches_the_kernel ... ok`: a test-only adapter holds the fake
provider's `TextEnd` pending. After its last text delta, the real Host interrupts
an active assistant item; its `ItemCompleted` is `Interrupted`. The completion
and marker/turn events equal `answers::stopped`, apart from ids and stamps.
An executing Bash call was tried first: the kernel retains the interruption
error output, rather than clearing it. A pending streamed tool call also does
not emit the helper's proposed completion. Therefore this proof uses the
explicitly permitted streaming-message alternative; it does not claim an
executing tool's output is cleared. No kernel behavior was changed to fit it.

1.3 **Provider isolation belongs to core** — `badf16b6`.
`host::tests::queue::the_provider_reads_text_but_never_the_headline ... ok`
asserts on `ScriptedProvider::requests()`. The TUI now has only
`run/picture_tests.rs::image_refusal_is_checked_once_at_the_run_boundary ... ok`.

2. **Notification modules** — `a70bf9a3`.
`notify/headline.rs` owns the headline and Markdown helpers; `notify/toast.rs`
owns the completion signal and toast. Their tests
`markdown_parts_keep_backticks_and_outcome_punctuation` and
`completion_toast_has_its_own_kind_tone_and_fixed_deadline` pass.
`finished` and its three original tests were compared byte-for-byte with
`git show fec21d36:crates/bingo-tool-bash/src/notify.rs`: identical.
All bash tests: `157 passed; 0 failed`; production precedes `mod tests`.

3. **Catalogue reply** — `081dc139`, `run/acting/setup.rs`.
The tools catalogue enters through `Reply::Catalogue` and `Run::reply`.
`acting_full_frames_match_the_prototype ... ok` checks all ten keyed full frames.

4.1 **Exporter** — `081dc139`, `m194b/fixtures.cjs`: one shared loader and named
`CLOCK_ORIGIN`; regeneration and `git diff --exit-code` for `acting-turns.json`
exit 0 with an empty diff. No sealed prototype/capture changed.
4.2 **Orphan comment** — `90d67d93`: removed above `said::voice_contract`;
all TUI tests and snapshots pass.
4.3 **One input check** — `7a138092`: `journal_prose` returns an optional id;
logging/opening callers use it, without duplicate `Input::Text` matching.
The ADR-0040 image-only why-comment is restored. Tests
`an_image_with_no_words_journals_as_one_image_part`, `a_logged_input_keeps_its_headline`
and `opening_and_queued_inputs_keep_their_headlines` pass.
4.4 **One item recorder** — `7a138092`: `Turn::record_item` serves `record` and
`barrier`; `a_steer_absorbed_at_the_barrier_keeps_its_headline ... ok`.
Core: `693 passed; 0 failed`.
4.5 **Chosen close rung** — `90d67d93`: `close` receives `keys::Escape`;
ctrl+c uses its already-read owner. The pane Esc adapter explains why the
plugin's close action needs that key. Tests
`ctrl_c_and_escape_send_the_same_interaction_cancel` and
`ctrl_c_closes_then_clears_and_an_unbound_alt_enter_splits` pass.
4.6 **Inline negation** — `90d67d93`: removed `keys::split_return`; the input
binding check negates `bound`. `alt_enter_keeps_the_composers_newline_binding`
and `legacy_esc_enter_returns_to_an_aside_ask_then_answers` pass.
4.7 **Toast order** — `90d67d93`: clear only after slash dispatch, with the
p7-input.js:84 why-comment; new test
`a_sent_line_clears_the_toast_after_slash_dispatch ... ok`.
4.8 **Typed text and padding** — `90d67d93`: `Content::{Plain, Markdown}` replaces
the bool; divider padding uses muted. Tests
`attention_inks_only_its_first_part_and_plugin_text_stays_whole`,
`headlined_code_preserves_embedded_backticks_and_plain_escapes` and
`a_notice_has_a_divider_and_its_arrival_keeps_a_deadline_without_room` pass.
4.9 **Read empty once** — `90d67d93`: `ask_to_stop` reads composer emptiness
before filtering; the drain proof and all acting input tests pass.

5. **History** — after unsetting `git rev-parse --local-env-vars`, ran
`git rebase -x 'cargo check --workspace --all-targets --locked' fec21d36`.
`23` successful `Finished dev profile` checks; `Successfully rebased and updated
refs/heads/m196-acting-never-moves.` Before/after ordered hashes are identical.
No failing commit, amendment, base move or conflict. The final records-only
tree passed a separate check with the same workspace command (exit 0).

6. **Records** — this file records round 3; round-2.md stays unchanged. The
plan's Verified is replaced in place; checks.txt holds this round's commands
and outputs. Part 2 remains open. All protected shared sites stay unchanged.

## Final checks and limits

All seven native gates pass; workspace tests: **6553 passed, 0 failed, 0 ignored**
(126 result lines). The PTY suite reports `20 passed`, including
`stopping_returns_a_queued_line_to_the_input`. No final test needed a wall-clock retry.
Discipline and budget pass (423 dependencies; 15 GB target/debug soft warning);
`cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.

Windows all-target checks with `--locked --keep-going` pass all the way through
for **bingo-sdk, bingo-core and bingo-tool-bash**. **bingo-surface-tui** stops in
**aws-lc-sys 0.44.0**: missing `stdlib.h` and `windows.h` in the native cross-build.
The actual Windows TUI build remains owed; no diagnostics were bypassed.

Native binary build passes. `TUI_SMOKE_SOCKET=m196-drive scripts/tui-smoke.sh`:
`tui-smoke ok`. `python3 docs/verification/m196/drive.py`: `PASS 100x32`,
`PASS 80x24`, each followed by `CLEAN` for its owned server and temporary
HOME/data/cwd. This round's terminal captures are committed beside the drive.
No further departures; the streaming/tool distinction is recorded under 1.2.
Full commands and outputs: [checks.txt](checks.txt).
