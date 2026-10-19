//! The sweep: every capture in `index.json`, each drawn full frame by the
//! milestone that made it pass. A test is named for its owner, so a failure
//! says whose regression it is.
use std::path::PathBuf;

use crate::{crew_frames, detail, footer, receipt, run::acting::edge_frame};

use super::{Entry, acting};

macro_rules! sweep {
    ($($test:ident: $name:literal by $owner:path;)*) => {
        /// Every capture the sweep draws.
        const SWEPT: &[&str] = &[$($name),*];
        $(
            #[test]
            fn $test() {
                $owner($name);
            }
        )*
    };
}

sweep! {
    m194_live: "live" by footer::tests::full_frame;
    m194_done: "done" by footer::tests::full_frame;
    m194_ascii_live: "ascii-live" by footer::tests::full_frame;
    m194_ascii_done: "ascii-done" by footer::tests::full_frame;
    m194_reduced_live: "reduced-live" by footer::tests::full_frame;
    m194_small_done: "small-done" by footer::tests::full_frame;
    m193_mode: "mode" by footer::tests::full_frame;
    m193_mode_later: "mode-later" by footer::tests::full_frame;
    m193_reduced_mode: "reduced-mode" by footer::tests::full_frame;

    m195_done_detail: "done-detail" by detail::tests::full_frame;
    m195_done_well: "done-well" by detail::tests::full_frame;
    m195_done_folded: "done-folded" by detail::tests::full_frame;
    m195_ledger: "ledger" by detail::tests::full_frame;
    m195_ledger_up: "ledger-up" by detail::tests::full_frame;
    m195_find: "find" by detail::tests::full_frame;
    m195_find_up: "find-up" by detail::tests::full_frame;
    m195_find_back: "find-back" by detail::tests::full_frame;

    m196_live_steered: "live-steered" by acting::full_frame;
    m196_live_stopped: "live-stopped" by acting::full_frame;
    m196_stop: "stop" by acting::full_frame;
    m196_stop_resent: "stop-resent" by acting::full_frame;
    m196_steer: "steer" by acting::full_frame;
    m196_steer_landed: "steer-landed" by acting::full_frame;
    m196_shell: "shell" by acting::full_frame;
    m196_shell_ls: "shell-ls" by acting::full_frame;
    m196_image: "image" by acting::full_frame;
    m196_image_sent: "image-sent" by acting::full_frame;
    m196_palette: "palette" by acting::full_frame;
    m196_palette_st: "palette-st" by acting::full_frame;
    m196_palette_ran: "palette-ran" by acting::full_frame;
    m196_small_palette: "small-palette" by acting::full_frame;
    m196_ascii_palette: "ascii-palette" by acting::full_frame;
    m196_skills: "skills" by acting::full_frame;
    m196_skills_review: "skills-review" by acting::full_frame;
    m196_skills_sent: "skills-sent" by acting::full_frame;
    m196_mention: "mention" by acting::full_frame;
    m196_mention_inserted: "mention-inserted" by acting::full_frame;

    m197_ask_edit: "ask-edit" by acting::full_frame;
    m197_ask_edit_down: "ask-edit-down" by acting::full_frame;
    m197_ask_edit_allowed: "ask-edit-allowed" by acting::full_frame;
    m197_ask_bash: "ask-bash" by acting::full_frame;
    m197_ask_bash_change: "ask-bash-change" by acting::full_frame;
    m197_ask_bash_editing: "ask-bash-editing" by acting::full_frame;
    m197_caution: "caution" by acting::full_frame;
    m197_caution_no: "caution-no" by acting::full_frame;
    m197_ascii_ask_edit: "ascii-ask-edit" by acting::full_frame;
    m197_small_ask_edit: "small-ask-edit" by acting::full_frame;
    m197_question: "question" by acting::full_frame;
    m197_question_preview: "question-preview" by acting::full_frame;
    m197_question_second: "question-second" by acting::full_frame;
    m197_question_ticked: "question-ticked" by acting::full_frame;
    m197_question_answered: "question-answered" by acting::full_frame;

    m198_model: "model" by acting::full_frame;
    m198_model_moved: "model-moved" by acting::full_frame;
    m198_model_chosen: "model-chosen" by acting::full_frame;
    m198_status: "status" by acting::full_frame;
    m198_status_moved: "status-moved" by acting::full_frame;
    m198_status_entered: "status-entered" by acting::full_frame;
    m198_plugins: "plugins" by acting::full_frame;
    m198_plugins_toggled: "plugins-toggled" by acting::full_frame;
    m198_plugins_github: "plugins-github" by acting::full_frame;
    m198_sessions: "sessions" by acting::full_frame;
    m198_sessions_filtered: "sessions-filtered" by acting::full_frame;
    m198_sessions_switched: "sessions-switched" by acting::full_frame;
    m198_rewind: "rewind" by acting::full_frame;
    m198_rewind_rewound: "rewind-rewound" by acting::full_frame;
    m198_keys: "keys" by acting::full_frame;
    m198_keys_closed: "keys-closed" by acting::full_frame;
    m198_tasks: "tasks" by acting::full_frame;
    m198_tasks_again: "tasks-again" by acting::full_frame;
    m198_schedule: "schedule" by acting::full_frame;
    m198_schedule_paused: "schedule-paused" by acting::full_frame;
    m198_schedule_ran: "schedule-ran" by acting::full_frame;
    m198_memory: "memory" by acting::full_frame;
    m198_memory_forgot: "memory-forgot" by acting::full_frame;

    m199_welcome: "welcome" by edge_frame;
    m199_welcome_recent: "welcome-recent" by edge_frame;
    m199_welcome_sent: "welcome-sent" by edge_frame;
    m199_exit: "exit" by receipt::tests::full_frame;
    m199_exit_back: "exit-back" by edge_frame;
    m199_retry: "retry" by edge_frame;
    m199_retry_recovered: "retry-recovered" by edge_frame;
    m199_signedout: "signedout" by edge_frame;
    m199_signedout_login: "signedout-login" by edge_frame;
    m199_context: "context" by edge_frame;
    m199_context_compacted: "context-compacted" by edge_frame;
    m199_wake: "wake" by edge_frame;
    m199_wake_later: "wake-later" by edge_frame;
    m199_background: "background" by edge_frame;
    m199_background_notified: "background-notified" by edge_frame;
    m199_narrow: "narrow" by edge_frame;

    m200_crew: "crew" by crew_frames::full_frame;
    m200_small_crew: "small-crew" by crew_frames::full_frame;
    m200_crew_reviewer: "crew-reviewer" by crew_frames::full_frame;
    m200_crew_allowed: "crew-allowed" by crew_frames::full_frame;
    m200_crew_home: "crew-home" by crew_frames::full_frame;
    m200_agent: "agent" by crew_frames::full_frame;
    m200_agent_reviewer: "agent-reviewer" by crew_frames::full_frame;
    m200_agent_allowed: "agent-allowed" by crew_frames::full_frame;
    m200_agent_home: "agent-home" by crew_frames::full_frame;
    m200_room: "room" by crew_frames::full_frame;
    m200_room_mention: "room-mention" by crew_frames::full_frame;
    m200_room_waiting: "room-waiting" by crew_frames::full_frame;
    m200_room_answered: "room-answered" by crew_frames::full_frame;
    m200_room_home: "room-home" by crew_frames::full_frame;
}

#[test]
fn every_capture_in_the_index_is_swept_once() {
    let index =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/design/ember/scenes/index.json");
    let entries: Vec<Entry> =
        serde_json::from_str(&std::fs::read_to_string(index).unwrap()).unwrap();
    let listed: Vec<_> = entries.iter().map(|entry| entry.file.as_str()).collect();
    let unswept: Vec<_> = listed.iter().filter(|name| !SWEPT.contains(name)).collect();
    assert!(unswept.is_empty(), "captures the sweep misses: {unswept:?}");
    let mut swept = SWEPT.to_vec();
    swept.sort_unstable();
    swept.dedup();
    assert_eq!(swept.len(), SWEPT.len(), "a capture is swept twice");
    assert_eq!(
        SWEPT.len(),
        listed.len(),
        "the sweep names a capture the index lacks"
    );
}
