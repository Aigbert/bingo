# M196 part 1 — round 5

Base: `d00ff6c7`. No new dependency or package.

## Review items and evidence

1. **Ctrl+c never goes home** — `f1407ba6`, TUI `keys.rs` and `input.rs`.
`Escape::Back` is outside `Close`. The close order lives in `ESCAPES`;
`closing` selects from it for both gestures. The unlisted Pane/Search prefix
is gone. Both gestures close an open child-view card; with no card, only Esc
returns home. Ctrl+c arms with its toast, then exits on the second press.
Tests pass:
- `ctrl_c_and_escape_close_the_same_child_interaction`;
- `escape_goes_home_but_ctrl_c_arms_exit_in_an_idle_child`;
- `ctrl_c_twice_on_an_empty_line_in_a_child_exits`;
- `every_acting_rung_obeys_the_policy_order`.
The existing run-level ask test now checks cancellation regardless of whether
the view is the root or child. To print exactly the obeyed order, the BINDINGS
Esc description and only that row in seven palette snapshots gain pane/search.
No palette implementation or other shared UI site changed.

2.1 **Doc comment** — `f1407ba6`: `clears_toast` is below `own_line`, so the
pictures doc comment belongs to `own_line` again. Toast-dispatch tests pass.
2.2 **Queued versus streamed calls** — `fdbb3145`, `2e0bc937`.
`answers::stopped` takes the test scenario's explicit `StopPhase`: the folded
snapshot does not expose whether the provider is still streaming. Only a
pending call in that phase is skipped. A queued chain call completes with the
tool marker and duration zero; running duration comes from `began_at`.
`stop_answer_leaves_streamed_tool_input_without_a_completion` and
`stop_answer_matches_the_kernel_for_a_queued_chain_call` compare with the real
Host/fake provider. The latter asserts a pending, unstarted second Bash call
completes with duration 0 after the first call is interrupted. Both pass, as do
the running-tool and streaming-assistant proofs and the queued-call unit test.
2.3 **One marker** — `fdbb3145`: the interruption item reads
`bingo_core::turn::INTERRUPTED_MARKER`; the tools use the executor's constant.
2.4 **Reachable fallback** — `f1407ba6`: removed the unconditional Arm entry;
the ordered interrupt table's fallback is now the sole Arm result.

3. **History scope** — `170256d7` rewords old `1b701c13` to
`test(docs): Wait for the background answer in the drive`, within the authorized
base rebase, with no extra rebase.
- `0ce88459` (now `20e1bf49`), `feat(tui): Carry toast parts and tone explicitly`, also changes sdk/tool-bash; accepted without splitting as directed.
- `927d2e0c` (now `7dc0331a`), `fix(tui): Withdraw queued lines before stopping the turn`, also changes sdk; accepted without splitting as directed.

4. **Rebase and conflicts** — ran `git rebase d00ff6c7`.
`turn/tests.rs` conflicted at the headline addition and its later extraction:
kept dev's round tests and the headline test, ending with both `mod round`
and `mod headline`. The headline commit also needed `self.fresh` changed to
dev's free `mint`; its corrected workspace check passed.
`turn.rs` conflicted at the shared recorder refactor: one free
`record_item(transcript, host, item)` emits/folds and returns the id;
dev's free `record` mints then calls it, and the barrier calls it directly.
There is no `Turn::record_item`. Each conflict resolution passed a workspace
all-target check before continuing. `7eb6edf0` applies rustfmt to those resolutions.

Then, after `unset $(git rev-parse --local-env-vars)`, ran
`git rebase -x 'cargo check --workspace --all-targets --locked' d00ff6c7`:
**32 successful checks**, including corrected reruns; rebase completed.
Two commits failed against dev's boxed ToolOutput and were amended in place:
- old `f20f5bb7`, checked as `27e9fab0`, became `bd1d3103`: box the helper's output;
- old `1f33bd2b`, checked as `11d35d06`, became `fdbb3145`: compare the dereferenced test output.
No other proof failure or amendment. Later fixture/formatting and records trees
were checked separately; no further rebase was run.

**M202 fixtures** — `2e0bc937`: story tool items receive `began_at` at their
prototype start instant; stop duration uses that stamp. All nine real-Host
answer proofs pass. `acting_full_frames_match_the_prototype ... ok` verifies
all ten keyed captures. No sealed capture changed, and no protected M202 drawing
function was edited.

5. **Records** — this file, replaced checks.txt, refreshed drive captures and
Verified rewritten in place. The plan stays at most 150 lines. Earlier round
records remain historical; part 2's ten named captures remain open.

## Final checks and limits

All seven native gates pass, serially. Workspace: **6594 passed, 0 failed,
0 ignored** (128 result lines); TUI 1446, core 717, tool-bash 157, PTY 20.
No wall-clock failure or isolated retry. Discipline and budget pass:
423 dependencies; target/debug 17 GB is a soft warning. Cargo deny:
`advisories ok, bans ok, licenses ok, sources ok`.

Windows all-target checks with `--locked --keep-going` pass fully for
**bingo-sdk, bingo-core, bingo-tool-bash**. The additional TUI check stops in
**aws-lc-sys 0.44.0**, missing `stdlib.h` / `windows.h`; Windows TUI remains
unverified here. No diagnostic bypassed.

Native binary build passes. `TUI_SMOKE_SOCKET=m196-drive scripts/tui-smoke.sh`:
`tui-smoke ok`. `python3 docs/verification/m196/drive.py`: `PASS 100x32`,
`PASS 80x24`, with both `CLEAN` lines confirming removal of the owned server
and temporary HOME/data/cwd. Captures are refreshed beside the drive.
Full commands and outputs: [checks.txt](checks.txt).

Departures: the shared Esc description/seven snapshot rows changed only to make
the printed policy truthful; the test helper's phase is explicit because it is
not a SessionState fact. The boxed output and free mint adaptations were needed
by the requested base. No new kernel fact, dependency, sealed capture or protected
implementation change. No other instruction issue found.
