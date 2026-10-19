# M202 round 4 — missing calls and a bisectable history

## Missing calls and comments

- Supersedes round-3.md's missing-tool policy/message qualification: hooks run first; an absent tool returns Default Deny before policy; resolve_check settles every decision before its match, including Full-tier Allow and a changed call. Hook refusal retains its message; every other case uses the executor's single `tool_not_found(name)` spelling. No new Reason variant.
- `cargo test -p bingo-core missing_tools_settle_after_hooks_without_policy_or_person --locked`: before the fix, `0 passed; 1 failed`, `rule allow: left Allowed, right Denied { message: "tool not found: Missing" }`; after, `1 passed; 0 failed`. Seven table rows cover rule Allow, mode Allow, policy Ask, hook Ask, hook Deny, rule Deny and Full-tier Allow, also rechecks; no policy/prompter call, receipt or allowed_by. The mode row models bypass with neutral vocabulary because the kernel cannot name policy modes.
- `an_unknown_call_fails_without_an_ask_or_stamps` also checks every recorded item frame for no Running. Core suite: `713 passed; 0 failed`.
- Supersedes round-3.md's CLI re-pins: plugins.rs and schedule.rs retain dev's three original message assertions and reason strings byte for byte. Their only diff from fec21d36 is an adjacent `assert_missing_call` invocation, which checks Failed, neither stamp and no interaction, without checking message. CLI: `309 passed; 0 failed`.
- Moved call_allowed/CallAllowed comments back to their items; call_asked/CallAsked explain first-ask ownership. Restored frames_of's comment; assert_missing_call explains no ask or start. Folded into their introducing commits.

## Seven small items

1. SDK `call_waiting` replaces every repeated open-wait predicate, including both turn test hosts; clock_of still reads call_wait's start.
2. SDK `transcript::live_call`/`LiveCall` owns Asked/Running precedence; TUI and crew share it. `only_unsettled_calls_can_be_asked_or_running` tests all live/terminal states with wait and interaction combinations; `a_call_wait_closes_when_it_begins_or_completes` tests both endings. `cargo test -p bingo-sdk --test live_call --locked`: `2 passed; 0 failed`.
3. Actor held comment explains why the call's wait is omitted from receipts and Interruption.
4. Clock doc distinguishes held receipts from old unstamped receipts.
5. mark_gated writes Running and began_at in one Allowed block.
6. Permission/question stop tests have separate setup and timing assertions, sharing stop_opened and assert_stopped_clock; no boolean branching helper. Both stop tests pass in the 713-test core suite.
7. Plan retains its line budget; final proof identity and count are recorded below. round-3.md is unchanged.

## History

- Requested folds: output boxing and RPC schema into the timestamp contract; CLI missing-call assertions into the reworded missing-tool gate; agent body helper into actor/agent stamps; landed formatting into the original loop; no-running frame assertion into the stamp commit. The three misplaced comments follow their introducing commits.
- Boxing conflicts in transcript.rs/turns.rs were resolved at the contract commit with type adaptation only, then at the TUI commit retaining reached-row behavior. No final-tree change resulted from those conflict resolutions.
- Discipline first rejected the test's literal policy mode name; a neutral mode label was fixed up into the gate commit and all seven gates reran successfully. The discipline check was not changed.
- First proof failed Clippy at the original gate split: unused action adapter, prematurely production round module and an oversized Decided enum. Moved action and the production module to the loop commit; folded existing gate boxing into the split and its dereference into the loop. The later boxing commit became empty and was dropped. Final code unchanged. Proof restarted from the beginning.
- Proof harness: plain background nohup did not survive tool-shell cleanup; relaunched nohup in a detached session. Rebase's repository-local Git variables initially leaked into fixture subprocesses. `bingo-context::root::tests::a_directory_in_no_repository_is_its_own_root` failed and passed alone; reproduction with GIT_DIR confirmed the cause. Fixture seed commits had replaced this worktree's detached HEAD/index and set shared test identity values; restored the exact checked tree/index, removed test identity overrides, and aborted cleanly. No named branch or extra worktree was created. The runner now unsets `git rev-parse --local-env-vars` before Cargo, and the proof restarted from the beginning.

## Gates and drives

- Final sequential fmt, workspace check, Clippy `-D warnings`, workspace tests, discipline, budget and deny passed. Workspace: `6569 passed; 0 failed`, 127 summaries. Discipline: `cohesion ok`, `discipline ok`. Budget: 423 dependencies (max 423), zero core rebuilds after touching TUI, `budget ok`; target/debug 13 GB soft-limit warning. Deny: advisories/bans/licenses/sources ok. No dependency added.
- Windows `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc --locked`: SDK/core/crew passed. TUI native stop: aws-lc-sys headers. Binary: aws-lc-sys, rquickjs-sys, tree-sitter and tree-sitter-bash native stops. No Rust platform error before those stops.
- `TUI_SMOKE_SOCKET_PREFIX=m202-drive bash scripts/tui-smoke.sh`: `tui-smoke ok`. `bash docs/verification/m202/drive.sh`: 100×32 and 80×24 passed both turns. Own temporary tmux servers cleaned; no snapshot or drive change.
- Historical test retry at the round-3 plan commit: `cargo test -p bingo --test pty pictures::a_herdr_pane_with_pictures_off_gets_the_chip_and_the_setting --locked -- --exact`: timed out waiting for “herdr: pictures need 0.9” in the workspace run; unchanged isolated rerun `1 passed; 0 failed`; complete workspace rerun passed. No timing adjustment.

- Further proof fix at the round-stamp commit: removed an empty initializer line caught by fmt and moved the permission stop clock expectation to the commit introducing the earlier call stamp. Receipt subset first `5 passed; 1 failed` (elapsed 2ms333µs versus 2ms624µs), then `6 passed; 0 failed`. Later held-ask conflicts retained that commit’s intended final text; final code tree unchanged. Proof restarted from the beginning.

- Retry/harness detail: exporting `RUST_TEST_THREADS=1` altered the gateway child-process stdout (`test child_process ... M129:ready`), causing one timeout and five poisoned-lock failures; all six claim tests passed alone (`cargo test -p bingo-gateway --test claim <name> --locked -- --exact`): `a_claimed_record_naming_a_live_non_gateway_does_not_block_run`, `a_free_claim_is_stale_and_stop_never_signals_its_live_pid`, `a_killed_gateway_can_restart_without_removing_files`, `a_legacy_record_naming_a_live_child_still_refuses`, `a_second_gateway_cannot_overwrite_a_held_claim`, and `doctor_never_removes_the_gateway_claim_file`. A non-inherited serial test flag was tried, then default concurrency restored. `--no-fail-fast` lets every workspace target finish before the one isolated retry. No product test timeout or assertion was weakened.
- The herdr pictures-off PTY test repeatedly timed out in historical workspace runs. At a17cd873 its first isolated retry failed too; subsequent diagnostic direct and detached-nohup runs both passed unchanged. Later isolated environment reproduction found the rebase-expanded PATH caused the failure: identical test failed with that PATH and passed with the original PATH while retaining Git’s other environment differences. The runner now receives M202_PROOF_PATH explicitly and restores it before Cargo. The stopped plan commit’s full suite was rerun; no test source, assertion or timeout changed. Earlier attempts remain documented, not silently counted as clean passes.

## Final proof and range-diff

- `git rebase -x "$PWD/target/m202-round4/proof-step.sh" fec21d36`, launched with nohup in this worktree: exit 0. HEAD before/after: `4aa72371b6b13f3055f13be3b35a7e7b04ea17fb`, byte-for-byte equal. 36 commits: fmt/check/Clippy passed at every commit; full workspace tests passed from the round-3 plan onward (18 commits), with the isolated ACP retry below. Final suite: `6569 passed; 0 failed`, 127 summaries.
- Every row runs `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --locked`, and `cargo clippy --workspace --all-targets --locked -- -D warnings`. Test rows run `cargo test --workspace --locked --no-fail-fast`; the added flag preserves all failures while allowing every target to finish. `—` means tests were not required for reviewed rounds 1–2b.
- At `081594cf`, CLI had `308 passed; 1 failed`: `acp::the_agents_session_id_is_journaled_once_as_an_extension`, acp.rs:525, got `[]` instead of `[("bingo.acp", "session:scripted")]`. `cargo test -p bingo --test cli acp::the_agents_session_id_is_journaled_once_as_an_extension --locked -- --exact`: unchanged isolated rerun `1 passed; 0 failed`. Every other workspace target passed. This row is RETRY, not a clean first-run pass.
- The completed proof used no second checkout, skipped no requested commit, and left no rebase in progress. Logs and the runner live under ignored `target/m202-round4/`; only this branch was rewritten. No push or merge.
- `git range-diff fec21d36..3ec79821 fec21d36..4aa72371`: `d9a0b7b3` gained the early boxing; `d5255fa7` gained the action/module timing fix, dereference and formatting. `59769708` became empty and dropped. `72d83488` absorbed `06accedf` and `beec915c`. `2710ade3` became the reworded `a17cd873`, absorbing `63eb87b8` plus the rule and CLI/doc fixes. `fc4b33d8` gained no-Running/message assertions, historical formatting and stop-clock expectation. `cb64e852` absorbed `4ee4b7d4` and its misplaced docs. `fe6c696d` now adds only the held-specific clock expectation after the earlier move. `c85675b3` lost type adaptations already folded into the contract. All other retained old patches compare equal; no other retained subject/body or relative order changed. Range-diff pairs the rewritten missing-tool commit as removed/added because its patch changed substantially.
- New end commits: `da302921` owns the shared SDK predicates and their callers/tests/docs (small items 1–4); `a9262993` owns the single call-start block and split stop helpers (5–6); `4aa72371` retires the obsolete missing-tool departure. This final records commit follows the proof as requested, bringing the branch to 37 commits above fec21d36.
- Instruction limit: a commit cannot record its own hash. The plan records the final count and exact proven HEAD before this documentation-only commit; the final response names the records commit. The post-proof records commit is not represented as part of the rebase proof. Round-3.md remains byte-for-byte unchanged; the plan remains under 150 lines. No new dependency, snapshot or product behavior departure beyond the requested missing-tool rule.

| Commit | Subject | fmt | check | Clippy | workspace tests |
| --- | --- | --- | --- | --- | --- |
| `d8b6256a` | test(core): Pin round actions and escape during an ask | PASS | PASS | PASS | — |
| `4bbf6d38` | refactor(core): Separate gate decisions from asking | PASS | PASS | PASS | — |
| `045598ed` | test(core): Pin interruption and receipt ownership | PASS | PASS | PASS | — |
| `6b88524b` | feat(core): Reach and land round calls individually | PASS | PASS | PASS | — |
| `3facf74d` | fix(core): Share the ask time before stopping a turn | PASS | PASS | PASS | — |
| `6b00a401` | test(bin): Keep session policy checks in their test | PASS | PASS | PASS | — |
| `fe5a7245` | refactor(core): Let round and recovery own their functions | PASS | PASS | PASS | — |
| `c90d4ba5` | docs(docs): Record M202 round-one verification | PASS | PASS | PASS | — |
| `322e6270` | test(bin): Match concurrent completions to commands | PASS | PASS | PASS | — |
| `5c52bb41` | refactor(core): Give the round walk its own borrow | PASS | PASS | PASS | — |
| `57b359b1` | refactor(core): Share unfinished item recovery frames | PASS | PASS | PASS | — |
| `f3b5fd00` | docs(docs): Record M202 round-two verification | PASS | PASS | PASS | — |
| `375e4b98` | test(core): Keep round assertions in the waiting rig | PASS | PASS | PASS | — |
| `a4008d2c` | refactor(core): Share completed item recording | PASS | PASS | PASS | — |
| `1063f663` | test(core): Recover journals lost during text and asks | PASS | PASS | PASS | — |
| `912bdcd9` | test(bin): Trim the batch completion assertion | PASS | PASS | PASS | — |
| `48811c48` | docs(docs): Record M202 round-2b verification | PASS | PASS | PASS | — |
| `145350ad` | docs(core): Say what recovery closes | PASS | PASS | PASS | — |
| `4ed041f0` | docs(docs): Plan call-owned waits and reached rows | PASS | PASS | PASS | PASS |
| `4507a3e0` | feat(sdk): Add call ask and running timestamps | PASS | PASS | PASS | PASS |
| `382e5604` | feat(sdk): Measure waits held by calls | PASS | PASS | PASS | PASS |
| `a17cd873` | fix(core): Settle missing tools before they ask or run | PASS | PASS | PASS | PASS |
| `081594cf` | feat(core): Stamp calls as the round asks and starts | PASS | PASS | PASS | RETRY |
| `1953eef3` | feat(core): Stamp bridged and agent-owned calls | PASS | PASS | PASS | PASS |
| `87f4e0af` | feat(core): Leave held ask time with the call | PASS | PASS | PASS | PASS |
| `72cb2fb7` | test(sdk): Clarify the shared clock comparison | PASS | PASS | PASS | PASS |
| `ad224686` | refactor(tui): Name the prompt note predicate | PASS | PASS | PASS | PASS |
| `a96d868f` | feat(tui): Draw only calls that asked or began | PASS | PASS | PASS | PASS |
| `90fd2e17` | refactor(tui): State crate visibility plainly | PASS | PASS | PASS | PASS |
| `29aaeb90` | fix(crew): Name only reached calls on agent cards | PASS | PASS | PASS | PASS |
| `fd2966b7` | test(docs): Drive reached calls at both terminal sizes | PASS | PASS | PASS | PASS |
| `ff4d1a06` | docs(docs): Record call waits and reached row evidence | PASS | PASS | PASS | PASS |
| `81836385` | docs(docs): Record the per-commit build proof | PASS | PASS | PASS | PASS |
| `da302921` | refactor(sdk): Share the live state of a call | PASS | PASS | PASS | PASS |
| `a9262993` | refactor(core): Keep call starts and stop tests cohesive | PASS | PASS | PASS | PASS |
| `4aa72371` | docs(docs): Retire the missing-tool departure | PASS | PASS | PASS | PASS |
