# M196 part 1 — round 4

Base: `fec21d36`, unchanged. No new dependency or package.

## Review items and evidence

1. **Running-tool stop double** — `f20f5bb7`,
`test_support/ember/answers.rs` and `run/acting/tests/proofs.rs`.
The double uses the public `bingo_core::executor::INTERRUPTED_MARKER`,
keeps an error output, clears progress, and sets duration for a running call.
Pending tool input is left alone. The proof normalizes `durationMs`.
`stop_answer_matches_the_kernel_for_a_running_tool ... ok` compares completed
items with the real Host after Bash is running; the streaming proof still passes.
The initial bare `sleep 60` fixture failed because Bash rejects foreground
sleep alone; `sleep 60 && echo done` reaches the running path and passes.
This was a fixture failure, not a wall-clock failure.
`acting_full_frames_match_the_prototype ... ok`: all ten frames, including
`live-stopped`, remain identical. No sealed capture changed.

2. **One close choice** — `fc403bed`, `keys.rs` and `input.rs`.
`Escape::Close(Close)` and `Interrupt::Close(Close)` carry the same selected
place to an exhaustive `close`; the second owner-to-rung table is gone.
One facts helper supplies the root-only card condition to both gestures.
`Close::Back` lets a child view return to its parent without cancelling its ask.
`ctrl_c_and_escape_leave_the_same_child_with_an_open_interaction ... ok`;
`ctrl_c_and_escape_send_the_same_interaction_cancel ... ok`;
`every_acting_rung_obeys_the_policy_order ... ok`.
The BINDINGS words and protected shared sites are unchanged.

3.1 **Background provider isolation** — `1550d0ad`,
`bingo-core/src/host/tests/queue.rs`.
`the_provider_reads_text_but_never_the_headline ... ok` now delivers a headlined
background notice with `Delivery::Wake` while a turn waits on a tool.
The next recorded provider request contains its words, and no request contains
either the opening headline or the background headline.

3.2–4 **Small TUI fixes** — `fc403bed`.
The drain proof comment names `CompletionReady`; `clears_toast` owns the
send predicate beside `own_line`; notice padding computes muted once.
`stopping_withdraws_two_lines_before_completion_can_drain_them ... ok`,
`a_sent_line_clears_the_toast_after_slash_dispatch ... ok`, and
`a_notice_has_a_divider_and_its_arrival_keeps_a_deadline_without_room ... ok`.

3.5 **Drive waits for the answer** — `1b701c13`, `drive.py`.
The background-done capture waits for `The background tests passed.`.
Both drives and their refreshed captures are recorded below.

3.6 **History split and proof** — the fixup removes only the core test from
old `badf16b6` (now `9bb78ee6`); `1550d0ad` restores and extends it under
`test(core)`. Autosquash kept the final tree byte-identical.
After `unset $(git rev-parse --local-env-vars)`, ran
`git rebase -x 'cargo check --workspace --all-targets --locked' fec21d36`:
**28 successful checks**, `Successfully rebased and updated
refs/heads/m196-acting-never-moves.` The build proof left its before/after
ordered hashes identical. No failing commit needed amendment; base unchanged.
Historical round-3 hashes remain in that record; only the authorized split
and its descendants changed. The final records tree passes a separate
`cargo check --workspace --all-targets --locked` (exit 0).

4. **Records** — this file, replaced `checks.txt`, refreshed drive captures,
and the plan's Verified rewritten in place. Earlier round records stay unchanged;
the plan stays below 150 lines and names all deferred part-2 captures.

## Final verification and limits

All seven native gates pass, serially. Workspace tests: **6555 passed,
0 failed, 0 ignored** (126 result lines); TUI 1437, core 693, bash 157,
PTY 20. No wall-clock test needed a retry. Discipline and budget pass
(423 dependencies; target/debug 16 GB soft warning). Cargo deny:
`advisories ok, bans ok, licenses ok, sources ok`.

Windows all-target checks with `--locked --keep-going` finish successfully
for **bingo-sdk, bingo-core, bingo-tool-bash**. The additional TUI check stops
in **aws-lc-sys 0.44.0**: missing `stdlib.h` / `windows.h`; the Windows TUI
build remains unverified here. No diagnostic was bypassed.

`cargo build -p bingo --locked`: exit 0.
`TUI_SMOKE_SOCKET=m196-drive scripts/tui-smoke.sh`: `tui-smoke ok`.
`python3 docs/verification/m196/drive.py`: `PASS 100x32`, `PASS 80x24`;
both `CLEAN` lines confirm removal of the owned server and temporary dirs.
Both background-done captures contain the awaited provider answer; the
100×32 capture now also shows the idle composer after that answer.

The local `dev` ref was observed at `d00ff6c7` during verification, but
`git merge-base HEAD dev` is still `fec21d36`. No rebase onto that newer dev
or change to another branch/worktree was made.

No further departures or instruction errors found. The corrected kernel premise
is implemented without changing kernel behavior; the Bash fixture adjustment is
explained under item 1. No prototype, sealed capture, dependency or protected-site
changes. Part 2 remains deferred. Full command evidence: [checks.txt](checks.txt).
