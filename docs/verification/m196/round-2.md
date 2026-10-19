# M196 part 1 — round 2

Base: `fec21d36`. No merge, rebase, push, new package or dependency. The authorized
HEAD amend is `9683017`: `docs(docs): Record M196 part-one verification and handoff`.

## Review items and evidence

1. **Esc row** — `d19f1d28`. The exact ESCAPES join is the one changed Binding;
   only that row changed in the seven authorized palette snapshots.
   `ctrl_p_is_unbound_and_absent_from_the_palette_keys` and 14 palette tests pass.
2. **Acting scenes** — `f4b98e56`, `3d379a3a`, `279958d5`, `5636e48f`. `Run::now()` uses an injected paired
   clock; motion still comes from detection. Clipboard reads use the same closure seam.
   The four real-clock exceptions document probe spacing, pull latency and update-cache age.
   `acting_full_frames_match_the_prototype` presses the indexed keys through
   `terminal_event`, checks sends/withdrawals/interrupts, folds frame/reply answers and
   compares all ten full frames through `drawn`/`assert_band`. Only the two named spinner
   cells are exempt. The focused final acting run reports `19 passed; 0 failed`, including all ten frames.
   Six real-Host/fake-provider proofs pass: `queued_answer_matches_the_kernel`,
   `withdrawal_answer_matches_the_kernel`, `sent_answer_matches_the_kernel`,
   `stop_answer_matches_the_kernel`, `shell_answer_matches_the_kernel`,
   `steer_answer_matches_the_kernel`. Shell proofs use portable `echo scene`.
   The sole exporter is `m194b/fixtures.cjs`; it exports turns only. The old acting.json
   and m196/fixtures.cjs are deleted. Resending clears the stop toast as submitInput does.
3. **Structured headlines** — `426214a5`, `e32c081d`. ADR-0098, sdk/headline.rs, typed item meta,
   Input and UserMessageProps fields, core/user.rs and regenerated schemas.
   `headlined_and_plain_inputs_round_trip_byte_identically` and
   `a_headline_lives_in_item_meta_and_round_trips_through_accessors` pass.
   Core: `a_logged_input_keeps_its_headline`, `opening_and_queued_inputs_keep_their_headlines`,
   `a_steer_absorbed_at_the_barrier_keeps_its_headline` — `3 passed; 0 failed`.
   `the_fake_provider_reads_text_but_never_the_headline` passes in the existing TUI
   integration test crate, avoiding a new core dependency. notify::finished and its tests
   are restored byte for byte to fec21d36. `markdown_parts_keep_backticks_and_outcome_punctuation`,
   `headlined_code_preserves_embedded_backticks_and_plain_escapes`, and
   `a_notice_has_a_divider_and_its_arrival_keeps_a_deadline_without_room` pass.
   The shell_ran and quiet_notices snapshots have no round-2 diff.
4. **Toast parts/tone** — `0ce88459`. Message stores parts/tone, Level maps once,
   notify_for constructs its hold, and footer/headline share tone::parts. Plugin text is
   one plain part. Slot::Toast/ToastProps gain optional Neutral-default tone and schemas.
   `pulled_panes_and_toasts_round_trip`, `a_notice_keeps_parts_tone_and_hold_from_birth`,
   `a_toast_uses_its_tone_once_and_keeps_its_tail_dim`,
   `attention_inks_only_its_first_part_and_plugin_text_stays_whole`, and
   `completion_toast_has_its_own_kind_tone_and_fixed_deadline` cover the contracts.
   Bash completion has its own job-finished signal and a 2400 ms fixed-clock lifetime.
   `cargo test -p bingo-tool-bash --lib --locked`: `157 passed; 0 failed`.
   Notice tests: `30 passed; 0 failed`. Three error-toast snapshots gain only the requested
   times glyph; the still-motion assertion now checks a bad mark and muted words.
5. **Stop ordering** — `927d2e0c`. One Stop effect, ordered concurrent withdrawals,
   then the shared interrupt path, then restoration. The session id travels with the reply.
   `stopping_withdraws_two_lines_before_completion_can_drain_them`,
   `a_stop_with_only_lost_withdrawals_still_interrupts_once`,
   `withdrawal_scripts_record_the_call_and_return_the_input`, and
   `a_fake_provider_delivery_wins_withdrawal_without_duplication` pass.
   An empty withdrawal set enters the same stopped path synchronously.
6. **Should-fix 6–10** — `8c0299b3`, `0ce88459` (item 7).
   CLEARED_HOLD names 1800 ms; ↑ and alt+m use key_text, without dead-code allowance.
   Shared close helper: `ctrl_c_and_escape_send_the_same_interaction_cancel`.
   One image refusal: `image_refusal_is_checked_once_at_the_run_boundary`.
   One `Draft::rows()` serves demand/render_composer. Live shell step rows:
   `a_live_shell_draws_a_running_step_instead_of_a_settled_well`.
   TUI at that commit: `1427 passed; 0 failed`.
7. **Nits 13–18** — `93799cac`. Escape docs; one bound boolean for split_return;
   activity/queue ownership comments; grid constants replace dock literals; voice-filter
   why-comment restored; `a_mid_line_image_token_leaves_one_space` passes.
   TUI at that commit: `1419 passed; 0 failed`.
8. **Records** — `docs(docs): Record M196 round-two verification`; drive.py is the sole drive entry point; drive.sh is deleted. This record,
   handoff, plan and checks.txt replace the round-one claims with round-two evidence.

## Acting timeline

All times are ms relative to CLOCK_ORIGIN_MS. Setup keys occur at 0 without advancing
clock; every indexed character/key then advances it by 60 ms. A number advances by
that amount; shot entries press nothing. `Q` = `做完以后更新一下 CHANGELOG`;
`S` = `测试名字用英文`; `C` = `再更新 CHANGELOG`; W/H = Delivery::Wake/Hold.
Provider-side tool/reasoning/reply events are fed at the exported turn's timestamps.
Every queued send receives QueueChanged/Queued at its send instant. Every successful
withdrawal receives QueueChanged before its reply; the interrupt's answer contains
Interruption and TurnCompleted. These helpers are checked against the real kernel.

| Scene | Setup / keys | Bingo sends, in order | Kernel answers beyond turn fixture | Frame |
|---|---|---|---|---:|
| live-steered | live starts −2600; S chars 800–1160, Enter 1220; C from 1280, Tab 2060 | S/W@1220, C/H@2060 | queue@1220; steer lands@2000; queue@2060 | 2520 |
| live-stopped | same, Esc 2520 | S/W@1220, C/H@2060 | same; withdraw C and interrupt@2520 | 3180 |
| stop | live starts −2000; Q + Tab@0; Esc1800 | Q/H@0 | queue@0; withdraw Q and interrupt@1800 | 2600 |
| stop-resent | same; Enter2900 | Q/H@0, Q/W@2900 | same; sent user item/TurnStarted@2900 | 3760 |
| steer | live starts −1600; S+Enter, Q+Tab@0 | S/W@0, Q/H@0 | both queue@0; S still waiting at shot | 200 |
| steer-landed | same | S/W@0, Q/H@0 | both queue@0; S lands@450 | 4300 |
| shell | T1; !git status --short+Enter@0 | !git status --short/W@0 | shell item/Applied@380 | 600 |
| shell-ls | same; !ls chars800–920, Enter980 | !git status --short/W@0, !ls/W@980 | shell items/Applied@380,1280 | 1840 |
| image | past(); Ctrl+V and text@0 | none | clipboard double gives PNG, no kernel send | 0 |
| image-sent | same; Enter300 | [image 1] 这个报错什么意思/W@300 + one PNG | sent user item/TurnStarted@300; provider completes@2073 | 2860 |

## Departures and instruction discrepancies

- Shell-ls does **not** start a kernel turn. ADR-0008 and the real Host emit a standalone
  shell item; the helper proof pins that behavior rather than fabricating TurnStarted.
- Image transport retains the existing `[image 1] ` token beside the Image part. The exact
  outgoing assertion includes it; fixture-prompt comparison removes that display token.
  The prototype's prompt contains only the typed words, so raw equality is impossible
  without changing the existing image transport contract.
- Setup supplies the pre-existing session identity/cwd, opus-5.5/anthropic model choice,
  high effort/config, common M194 tool catalogue, and default/acceptEdits permission
  placement. These are setup facts, not state computed from keys. Live frames also need
  the bash plugin's declared background-action Slot::Row; `timeline::promote` supplies
  that plugin fact, which is absent from turn fixtures.
- The original plan's shell-specific placeholder does not exist in the prototype;
  the standard placeholder stays. Shell folding is gone; OUTPUT_ROWS/EXPAND still serve
  action/tool output and generic quiet notices, outside this scope.
- Part 2 remains open: bricks 7–8, palette ranking, and captures palette, palette-st,
  palette-ran, skills, skills-review, skills-sent, mention, mention-inserted,
  small-palette, ascii-palette. Background capture ownership remains M199's.
- No protected M202 function, focus.rs, Open, view::layer/menu, window, mentions or
  completion source changed. Exact shared-site changes are in handoff.md.

## Authorized constructor compatibility edits

Each existing constructor below gained only `tone: bingo_sdk::Tone::Neutral,`.
The palette exception was explicitly authorized after the missing-field compiler error.
The TUI toast module also has its required parts/tone drawing changes.

- crates/bingo-mod-palette/src/lifecycle.rs
- crates/bingo-mod-models/src/pins.rs
- crates/bingo-mods/src/ui.rs
- crates/bingo-surface-tui/src/component/sites_tests.rs
- crates/bingo-surface-tui/src/keybar/tests.rs
- crates/bingo-surface-tui/src/run/builtins_tests.rs
- crates/bingo-surface-tui/src/toast.rs

## Validation corrections

One preliminary cargo fmt --check overlapped an earlier Windows check. Compilation/test
jobs and all final gates/Windows checks were serial; no wall-clock test failed.

The first clippy run rejected the large test Answer enum; `5636e48f` boxes its Event.
The bash completion test formerly selected the last signal indiscriminately; it now
selects jobs and separately asserts the new completion-toast signal. These were direct
consequences of this change, not wall-clock failures. No sealed capture changed.

## Final native gates

All seven pass, serially. `cargo test --workspace --locked` ran with provider credentials
unset: **6552 passed; 0 failed; 0 ignored**, across 126 result lines. TUI: 1435 passed;
core: 692; bash: 157; PTY: 20, including stopping_returns_a_queued_line_to_the_input.
No wall-clock failure or isolated retry occurred in the final gate run.
Formatting exits 0; workspace check and clippy finish successfully; discipline says
`discipline ok`; budget says `423 (max 423)` and `budget ok` (14 GB soft warning);
`cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.
Exact command/result excerpts, schema regeneration and Windows results are in checks.txt.

## Windows

Every touched crate was checked with `--all-targets --target x86_64-pc-windows-msvc
--locked --keep-going`, one Cargo invocation at a time. Of the mandatory five,
bingo-sdk, bingo-core and bingo-tool-bash completed successfully; bingo-surface-tui
and bingo did not reach a complete target build. Additional successes: bingo-surface-rpc,
bingo-mod-models and bingo-mod-palette. No Rust source diagnostic remained.

- bingo-surface-tui, bingo-channels, bingo-surface-print: aws-lc-sys cannot find stdlib.h/windows.h.
- bingo: the same aws-lc-sys failure; tree-sitter/tree-sitter-bash cannot find stdio.h;
  rquickjs-sys invokes cc with MSVC flags including /std:c11, which cc treats as a missing file.
- bingo-mods: aws-lc-sys and rquickjs-sys failures above.

These are native cross-toolchain limitations; no dependency or target bypass was added.

## Terminal verification

`cargo build -p bingo --locked` completed. `TUI_SMOKE_SOCKET=m196-drive
scripts/tui-smoke.sh` reports `tui-smoke ok`. `python3 docs/verification/m196/drive.py`
reports `PASS 100x32` and `PASS 80x24`, each followed by `CLEAN`: the owned server and
its temporary HOME/data/cwd were removed. Both use the fake provider; the drive's PNG
clipboard double is child-local. The refreshed text/ANSI captures are beside this record.
There were no smoke/drive failures or retries. The Python drive is the sole entry point.
