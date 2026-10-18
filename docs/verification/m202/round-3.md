# M202 round 3 — call-owned waits and reached rows

## History

- Rebased only `m202-ask-when-reached` from `8f0016be` onto `fec21d36`, with the requested autosquash todo. No conflict or dev compatibility fix.
- `git range-diff 8f0016be..d38666cb fec21d36..255fc402`: 15 unchanged patches; `38616015` → `d5255fa7` adds the landed-channel diagnostic; `4baee419` → `2194c25d` absorbs `8a69e711`'s unused-wrapper removal; `3ab12e9f` → `74526dc4` puts text last and removes its module-doc edit. `8a69e711` disappears as a separate patch. All other order, subjects and bodies remain.
- `c988b2cd` separately says recovery closes what lost or dropped turns left open.

## Contract and ownership (sections 2–8)

- `optional_provenance_and_ask_times_round_trip` and `old_items_and_rule_reasons_remain_unchanged`: tests first failed E0026 (missing `asked_at`, `began_at`), then `2 passed; 0 failed`. Both stamps survive event serialization and replay; old fixture writes unchanged.
- Generated, never hand-edited: `schema/{events,dollar,plugin,plugin-2,rpc}.json`, TypeScript `{dollar-types,events,wire}.ts`. Used `BINGO_UPDATE_SCHEMA=1` for SDK catalogue, mods dollar schema/types, plugin schema, `events_ts` and `wire_ts` tests. The initial combined generation ran events TypeScript before its catalogue; it failed “events.json must be regenerated first”; ordered generation passed. The full suite also found `schema/rpc.json` drift (surface-rpc schema.rs:132); the RPC-surface generator was absent from the requested list and was run too, adding only the two fields.
- Production ToolCall constructions: core `accumulator::start_call` and `Actor::open_call` mint both stamps `None`. `Turn::external` constructs an agent call, stamps `began_at` from the minted `started_at`, and carries both stamps over each body rebuild. All other Rust literals found by `rg 'ItemBody::ToolCall'` are test fixtures; mutations of existing bodies preserve the fields.
- Production kernel writes of Running on ToolCall: `Round::mark_gated`, `Actor::call_allowed`, `Turn::external`. Each emits `began_at` in the first Running frame. Accumulator Running writes are text/reasoning, not calls. TUI `component/items::changed` changes a render copy and preserves the stamps.
- `an_unknown_tool_settles_before_committing_to_an_ask`, `an_unknown_call_fails_without_an_ask_or_stamps`: missing tool settles before asking; Failed “tool not found”, no interaction or stamps. `Waiting` owns its tool.
- Gate qualification: `check_kernel` previously returned Allow immediately for missing tools, bypassing policy. It now consults policy with default traits, empty subjects and any hook confirmation so the specified unknown-tool policy case is reachable.
- Channel-driven `read_lands_before_edit_asks_and_edit_lands_before_bash_is_reached`: stamp precedes InteractionOpened, read has only began, unreached Bash has neither. `yes_for_the_session_answers_the_next_edit_in_the_same_round`: session-allowed second call has only began. `a_denial_lands_without_waiting_and_does_not_stop_the_walk`: policy denial has neither. `a_persons_no_stamps_only_the_ask`: no began after No. Round suite: `12 passed; 0 failed`.
- `a_bridged_reask_keeps_its_first_stamp_and_starts_with_began_at`: one mailbox orders stamp before ask; changed input re-asks without moving the stamp; Running carries began. `an_agent_journals_one_external_call`: began equals started and persists across update/completion.
- `a_calls_wait_spans_the_answer_until_running_frame_by_frame`, `completed_and_interrupted_calls_close_their_own_waits`, `real_frames_never_restart_the_clock_between_answer_and_start`: `3 passed; 0 failed`; literal 2000 → 2000 → 3500 with deadline 8000, finished 4000, stopped 2000. Existing `turn_clock` literals 7150/9850, 650 and 500 remain (`12 passed`). `call_wait` is the shared wait reader.
- Held receipts re-pinned: `an_answer_republishes_not_asking`, `mismatched_session_scope_records_only_the_final_denial`, `edited_then_session_allowed_records_one_final_interaction`, `denial_feedback_survives_an_immediate_escape_once`, `two_answered_asks_keep_their_own_opening_instants`: receipt asked_at was an interaction instant; now None, with the call retaining its stamp. Actor Deny still names the ask's turn; question receipt tests keep opened_at.
- Stop re-pins: `permission_stop_agrees_in_outcome_hooks_and_journal` and `closing_with_an_ask_agrees_in_outcome_hooks_and_journal` now assert no Interruption ask time in all three views, and an interrupted, completed call owns its wait. `question_stop_agrees_in_outcome_hooks_and_journal` and the no-ask test stay unchanged. `stopping_a_permission_does_not_invent_an_answer` reads the call's wait, not the ask; post-call completion bookkeeping remains working time under the specified clock formula.
- `stopping_a_reasked_call_counts_from_its_first_stamp` pins the new first-ask behavior, replacing the former accepted second-ask cost. `edited_then_session_allowed_records_one_final_interaction` also pins the first stamp across a re-ask.
- Both twice-resume and cut-journal tests stamp `lost_ask_journal`; recovery preserves askedAt, interrupts/completes the call and keeps the stopped clock. Old unstamped Interrupted calls are hidden. Mid-running-call recovery still counts until resume, intentionally unchanged. Stops in either mailbox gap keep the call's stamp.
- Core suite after these changes: `713 passed; 0 failed`.

## Surface, crew and M194 nits (sections 9–12)

- `stamps_and_status_decide_whether_a_step_exists` covers Pending, held Pending/Running, Running, stopped asked/begun, old unstamped stopped, Failed and Completed. Existing denial tests pin said no ahead of other states.
- `a_held_step_stays_until_it_runs_or_stops_and_never_reveals_the_next_call`: read remains landed, edit remains needs you across stamp/open/resolved frames, later call requests no Ledger and stays absent; stopping reached work says stopped. `unreached_calls_are_hidden_even_when_quiet_and_old_stops_stay_hidden`, `an_agent_that_began_is_stopped_and_an_old_open_interaction_still_asks` cover the other visibility edges.
- `a_running_steps_clock_starts_at_began_at` replaces receipt-start timing. `component/live` uses the same open wait, so neither row nor busy state drops between answer and Running.
- `a_card_names_only_a_running_or_waiting_call`: Pending without a wait is not the teammate's current tool. Crew suite `28 passed; 0 failed`.
- Snapshots: none changed. `screens::folds::{ascii_turns,turn_stopped}` initially failed because the synthetic stopped call had no began stamp; correcting its reached fixture restores both captures unchanged. First corrected TUI suite: `1429 passed; 0 failed`.
- Separate Plan M194 commits remove redundant live setup, rename clock to measured, extract `fn note(&Item)`, and make mode_labels explicitly pub(crate).
- Plan now includes brick 8, updated brick 6/7, contracts, clock criteria and risks. Prototype followed; no ink, key or call-order changes.

## ItemUpdated audit (section 13)

- channels `deliver::apply` calls `grew`: only assistant answer text is offered; `Streaming::offer` drops identical text before advancing its clock. A call stamp cannot create text/activity.
- core `context.rs` occurrences at former 41/879/1126 are test-only replacement/oracle cases; production context reads transcript model content, ignores the stamps.
- core `inputs::call_allowed`/`call_asked` are writers, not activity readers; status changes only in call_allowed.
- core `turn::emit_and_fold` upserts the item and sends it; no activity inference.
- core `Turn::external` is an agent-frame writer and preserves beganAt across rebuilds.
- sdk `event.rs` declares ItemUpdated; no activity logic.
- sdk `event/record::restore_item_bytes` only restores unknown raw records; stamps are ordinary known fields.
- sdk `state::apply` delegates to transcript; the other listed state occurrence is a test that message counts change only at completion.
- sdk `Transcript::change` upserts by item id in original order; does not change status or times implicitly.
- print `render` explicitly ignores ItemUpdated; `stream_json` also ignores it in its compatibility envelope.
- RPC `session::event_links` links item ids and marks known item updates state-certain; no activity inference.
- Black-box re-pins: CLI `plugins::a_tool_plugin_switched_off_is_written_listed_and_gone_from_the_model_s_tools`, `plugins::an_external_plugin_is_listed_switched_and_never_spawned`, and `schedule::a_person_who_turned_wakes_off_is_offered_no_wake_tool`. Before: executor error `tool not found: <name>`; after: section 3's gate error `tool not found`. Each now matches the failed call by tool name and pins no stamp or interaction. Initial full run: CLI `306 passed; 3 failed`, at plugins.rs:98/202 and schedule.rs:634. This error-text change is an exception to “only frame order and new fields,” required by the specified gate message.
- Boxing output requires type adaptation only in CLI agents/models/rooms/journal result helpers; their assertions and ordering are unchanged. No print, RPC, ACP or channel behavioral re-pin.

## Gates, drive and per-commit proof

- Workspace check and Clippy `-D warnings`: `Finished dev profile` after boxing ToolCall's optional output. This keeps the two requested timestamp types and the wire shape while avoiding a new large-enum lint exception. Mechanical output constructors/readers and four CLI result helpers changed accordingly.
- One Clippy invocation was launched before the check wrapper returned; Cargo queued it on the build-directory lock. Subsequent Cargo commands wait for the preceding process to exit. No concurrent compiler work was requested beyond Cargo's own normal jobs.
- Final seven gates, one at a time: `cargo fmt --all -- --check` exit 0; `cargo check --workspace --all-targets --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings` → `Finished dev profile`; `cargo test --workspace --locked` → 6,567 passed, zero failed, 126 summaries (core 713, TUI 1431, CLI 309, RPC integration 104, RPC wire 43, print 103, ACP provider 152, channels 116, crew 28).
- `scripts/check_discipline.sh` → `cohesion ok`, `discipline ok`; no exception added. Extracted agent body replacement to keep the changed external-call function below 60 lines. Existing advisories remain.
- `scripts/budget.sh` → `dependencies (unique, normal): 423 (max 423)`, `relink isolation: touching the TUI recompiled 0 crates for core`, `target/debug: 13 GB (soft max 5)`, `budget ok`. No dependency added. `cargo deny check` → `advisories ok, bans ok, licenses ok, sources ok`.
- Incidental retry: CLI `rooms::a_post_that_calls_on_a_patient_holder_wakes_it_at_once` failed rooms.rs:815 (holder's final debt still present) in the first re-pin rerun: `308 passed; 1 failed`. Unchanged isolated command `cargo test -p bingo --test cli rooms::a_post_that_calls_on_a_patient_holder_wakes_it_at_once --locked -- --exact` → `1 passed; 0 failed`; both later full runs passed it. No timing adjustment. The re-pinned permission-stop clock test was also run alone → `1 passed; 0 failed`.
- Windows: `cargo check -p <crate> --all-targets --target x86_64-pc-windows-msvc --locked` passed for bingo-sdk, bingo-core, bingo-mod-crew, bingo-agents, bingo-checkpoints, bingo-context, bingo-rooms, bingo-store-jsonl and bingo-surface-rpc. Native stops: TUI/print/plugin-rpc at aws-lc-sys (stdlib.h/windows.h); mods at rquickjs-sys (MSVC flags unsupported by host clang) and aws-lc-sys; binary also at tree-sitter/tree-sitter-bash headers. No Rust platform error was reported before those stops.
- `bash docs/verification/m202/drive.sh` (no argument) → `100x32: ... passed`, `80x24: ... passed`: own Edit row needs you, later Bash absent, Yes once, Bash spins/lands, second Edit stops, echo never absent from the turn-2 terminal stream. Temporary homes/data/cwds only; own tmux server killed. Initial script fixes: Enter had focused the composer (focus ask then select 1); cursor-diff output does not contain a contiguous prompt (take the byte offset before turn 2, separately assert the pane prompt). Neither required a product change.
- `TUI_SMOKE_SOCKET_PREFIX=m202-drive bash scripts/tui-smoke.sh` → `tui-smoke ok`; no smoke re-pin, own temporary socket cleaned.
- Per-commit proof is pending.
