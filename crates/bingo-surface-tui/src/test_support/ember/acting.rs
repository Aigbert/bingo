use super::Capture;

#[test]
fn acting_full_frames_match_the_prototype() {
    for name in [
        "live-steered",
        "live-stopped",
        "stop",
        "stop-resent",
        "steer",
        "steer-landed",
        "shell",
        "shell-ls",
        "image",
        "image-sent",
    ] {
        let spinners: &[(u16, u16)] = match name {
            "live-steered" => &[(25, 3)],
            "steer-landed" => &[(24, 3)],
            _ => &[],
        };
        let capture = Capture::load(name).spinners(spinners);
        capture.render(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(crate::run::acting::draw(&capture));
        });
    }
}

/// Each sheet capture, its keys pressed into a run whose sheets the owners draw.
fn sheet_frame(name: &str, spinners: &[(u16, u16)]) {
    let capture = Capture::load(name).spinners(spinners);
    capture.render(|| {
        let buffer = pressed(&capture);
        super::assert_band(&capture, super::Band::Frame, &buffer);
    });
}

/// The rows of a sheet capture whose facts are all owned by now, each with
/// the frame row it lands on; a fact another milestone owes leaves its rows out.
fn sheet_rows(name: &str, rows: &[(std::ops::Range<u16>, u16)]) {
    let width = Capture::load(name).size().width;
    let cells: Vec<_> = rows
        .iter()
        .map(|(band, top)| (band.clone(), 0..width, *top))
        .collect();
    sheet_cells(name, &cells);
}

/// The cells of a sheet capture whose facts are owned by now, by rows and
/// columns, each with the frame row its first row lands on.
fn sheet_cells(name: &str, cells: &[(std::ops::Range<u16>, std::ops::Range<u16>, u16)]) {
    let capture = Capture::load(name);
    capture.render(|| {
        let buffer = pressed(&capture);
        for (rows, columns, top) in cells {
            super::assert_cells(&capture, &buffer, (rows.clone(), columns.clone()), *top);
        }
    });
}

fn pressed(capture: &Capture) -> ratatui::buffer::Buffer {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(crate::run::acting::draw_sheet(capture))
}

#[test]
fn m198_model_frames_are_pressed_into_the_owner() {
    for name in ["model", "model-moved", "model-chosen"] {
        sheet_frame(name, &[]);
    }
}

#[test]
fn m198_surface_frames_are_pressed_into_the_surface() {
    for name in ["keys-closed", "rewind"] {
        sheet_frame(name, &[]);
    }
}

/// M199 owes the welcome a rewind to the first prompt comes back to: the
/// wordmark, the branch with `3 changed` and the recent sessions. Until then
/// the kernel's rewind receipt stands where it would be; the prompt put back
/// and the toast are this milestone's.
#[test]
fn m198_rewound_band_waits_for_m199_welcome() {
    sheet_rows("rewind-rewound", &[(27..32, 27)]);
}

/// M200 owes `ctrl+g whoever needs you`: until it lands, `ctrl+c` closes the
/// switch group, every row fits and the sheet has no `15 of 16` to say, so
/// it stands one row shorter.
#[test]
fn m198_keys_band_waits_for_m200_ctrl_g() {
    sheet_rows("keys", &[(12..28, 13), (30..32, 30)]);
}

#[test]
fn m198_task_frames_are_pressed_into_the_owner() {
    for name in ["tasks", "tasks-again"] {
        sheet_frame(name, &[(25, 3), (26, 3)]);
    }
}

#[test]
fn m198_plugin_frames_are_pressed_into_the_owner() {
    for name in ["plugins", "plugins-toggled"] {
        sheet_frame(name, &[]);
    }
}

#[test]
fn m198_session_frames_are_pressed_into_the_owner() {
    for name in ["sessions", "sessions-filtered", "sessions-switched"] {
        sheet_frame(name, &[]);
    }
}

#[test]
fn m198_schedule_frames_are_pressed_into_the_owner() {
    sheet_frame("schedule", &[]);
    sheet_frame("schedule-paused", &[]);
    sheet_frame("schedule-ran", &[(26, 3)]);
}

/// M199 owes the branch and `3 changed` the directory row reads, and M200
/// the `agents` row that opens its crew sheet: until then the directory row
/// says the bare folder and the sheet stands one row shorter, so everything
/// above it sits one row lower.
#[test]
fn m198_status_band_waits_for_m199_branch_and_m200_agents() {
    for name in ["status", "status-moved", "status-entered"] {
        sheet_rows(name, &[(0..26, 1), (27..28, 28), (29..32, 29)]);
    }
}

/// No milestone owes the facts the memory rows end with: the memory service
/// lists an instruction file's path and tokens but not its lines, and a
/// memory's name but not the day it was kept. Until one does, those columns
/// stand empty and everything else is the capture's.
#[test]
fn m198_memory_band_has_no_owner_for_lines_and_dates() {
    for name in ["memory", "memory-forgot"] {
        sheet_cells(
            name,
            &[
                (0..25, 0..100, 0),
                (25..30, 0..70, 25),
                (30..32, 0..100, 30),
            ],
        );
    }
}

#[test]
fn m196_palette_frames_are_pressed_into_the_owner() {
    for name in ["palette", "palette-st", "small-palette", "ascii-palette"] {
        sheet_frame(name, &[]);
    }
}

#[test]
fn m196_skill_frames_are_pressed_into_the_owner() {
    for name in ["skills", "skills-review"] {
        sheet_frame(name, &[]);
    }
}

/// The harness answers the skill whole where the prototype is mid-reveal, so
/// the answer's rows and the rows above them sit one row higher than the
/// capture; the steer rule, the input and the footer are cell for cell.
#[test]
fn m196_skill_sent_is_pressed_into_the_owner() {
    sheet_rows("skills-sent", &[(1..25, 0), (28..32, 28)]);
}

#[test]
fn m196_mention_frame_is_pressed_into_the_surface() {
    sheet_frame("mention", &[]);
}

/// The prototype inserts `@path ` with its space, then draws its caret over
/// that space, since its wrap drops a row's trailing blank; the composer
/// draws the space and the cursor after it. Every other cell is the capture's.
#[test]
fn m196_mention_inserted_keeps_its_space_before_the_cursor() {
    sheet_cells(
        "mention-inserted",
        &[
            (0..29, 0..100, 0),
            (29..30, 0..49, 29),
            (30..32, 0..100, 30),
        ],
    );
}

/// `/st` then ⏎ runs `/status`: its sheet waits on M199 for the branch and
/// `3 changed` and on M200 for the `agents` row, as the status capture does.
#[test]
fn m196_palette_ran_waits_for_m199_branch_and_m200_agents() {
    sheet_rows("palette-ran", &[(0..26, 1), (27..28, 28), (29..32, 29)]);
}

#[test]
fn m197_permission_frames_are_answered_by_keys() {
    for name in [
        "ask-edit",
        "ask-edit-down",
        "ask-bash",
        "ask-bash-change",
        "ask-bash-editing",
        "caution",
        "caution-no",
        "ascii-ask-edit",
        "small-ask-edit",
    ] {
        sheet_frame(name, &[]);
    }
}

#[test]
fn m197_an_allowed_edit_goes_on_with_its_session_rule() {
    sheet_frame("ask-edit-allowed", &[(25, 3)]);
}

#[test]
fn m197_question_frames_are_answered_by_keys() {
    for name in [
        "question",
        "question-preview",
        "question-second",
        "question-ticked",
    ] {
        sheet_frame(name, &[]);
    }
}

#[test]
fn m197_an_answered_question_goes_on_with_the_turn() {
    sheet_frame("question-answered", &[]);
}
