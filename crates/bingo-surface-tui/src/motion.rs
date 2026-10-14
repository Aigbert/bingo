//! Every row of `docs/design/tui.md` §6, frame by frame on an injected clock.
//!
//! A cue is a pure function of [`crate::clock::Now`] and state, so each row
//! below is a handful of samples at named instants rather than something to
//! watch for. What the samples assert is the *cue* — a glyph, a style, a row
//! that is there or is not — because that is what §6 promises; where the cue
//! is drawn is `screens.rs`'s business.
//!
//! The rule the whole file is here to hold: every motion reports a state
//! change, stillness is the default, and `BINGO_MOTION=off` stills all of it.

use std::time::Duration;

use bingo_sdk::{Event, ItemStatus, TurnStatus};
use ratatui::style::Style;

use crate::clock::{FRAME, Now};
use crate::test_support::*;
use crate::tree::Tree;
use crate::{keys, theme};

/// The style of the run of cells carrying `needle` on a drawn frame.
fn style_of(tree: &Tree, ui: &mut crate::ui::Ui, now: Now, needle: &str) -> Style {
    let painted = crate::painted::painted(80, 24, tree, ui, now);
    painted
        .row(needle)
        .into_iter()
        .find(|(text, _)| text.contains(needle))
        .map(|(_, style)| style)
        .unwrap_or_else(|| panic!("no run carries {needle:?}"))
}

/// A screen at one instant, as text.
fn screen(tree: &Tree, ui: &mut crate::ui::Ui, now: Now) -> String {
    draw_tree(80, 24, tree, ui, now)
}

/// The style of the first run of the row carrying `needle` — the sparkle on
/// the activity row, the border on the input box's own row.
fn leading_style(tree: &Tree, ui: &mut crate::ui::Ui, now: Now, needle: &str) -> Style {
    crate::painted::painted(80, 24, tree, ui, now)
        .row(needle)
        .first()
        .map(|(_, style)| *style)
        .unwrap_or_else(|| panic!("no row carries {needle:?}"))
}

/// One row of it, without the quotes a test backend prints around each.
fn row(tree: &Tree, ui: &mut crate::ui::Ui, now: Now, needle: &str) -> String {
    screen(tree, ui, now)
        .lines()
        .find(|line| line.contains(needle))
        .map(|line| line.trim_matches('"').trim_end().to_string())
        .unwrap_or_else(|| panic!("no row carries {needle:?}"))
}

/// What a block does as it lands: it takes its own room and nothing more, one
/// light crosses the name of a call that came back, and one that came back
/// wrong cools out of `bad`.
mod landing;

// ---- presence: the sparkle and the breath -------------------------------

/// A turn that has been running long enough to have a row of its own.
fn turning() -> Tree {
    solo(&folded(vec![frame(1, started("trn_1"))]))
}

#[test]
fn waiting_caret_blinks_without_changing_its_glyph() {
    let tree = turning();
    let (mut ui, now) = scene();
    for ms in [530, 1060, 1590] {
        let at = later(now, ms);
        let style = leading_style(&tree, &mut ui, at, "esc to interrupt");
        let expected = if ms / 530 % 2 == 1 {
            theme::dim()
        } else {
            theme::presence()
        };
        assert_eq!(style, theme::as_drawn(expected));
        assert!(row(&tree, &mut ui, at, "esc to interrupt").starts_with('▍'));
    }
}

#[test]
fn the_blocked_caret_breathes_on_the_tool_period() {
    let (mut ui, now) = mid_turn();
    theme::with(crate::painted::truecolor(), || {
        for ms in [0, 550, 1100, 1650, 2200] {
            let at = later(now, ms);
            assert_eq!(
                leading_style(&solo(&calling()), &mut ui, at, "esc to interrupt"),
                theme::as_drawn(theme::breath(crate::clock::breath(
                    at,
                    Duration::from_millis(2200)
                )))
            );
        }
    });
}

/// A state whose one item is still arriving, and one whose one item is a
/// call the turn is waiting on.
fn answering() -> bingo_sdk::SessionState {
    let mut state = folded(vec![frame(1, started("trn_1"))]);
    state.apply(&frame(
        2,
        Event::ItemStarted {
            item: assistant("itm_1", "half a sen", ItemStatus::Running),
        },
    ));
    state
}

fn calling() -> bingo_sdk::SessionState {
    folded(running_bash())
}

/// §6's first principle is that every motion reports a state change, and a
/// breath at one fixed period reports only "a turn is running" — which the
/// row's presence already says. So the period is what the turn is *doing*.
#[test]
fn the_breath_quickens_while_words_arrive_and_slows_while_a_tool_holds_the_turn() {
    let ms = Duration::from_millis;
    assert_eq!(crate::activity::breath_of(&answering()), ms(900));
    assert_eq!(crate::activity::breath_of(&calling()), ms(2_200));
    assert_eq!(
        crate::activity::breath_of(&folded(vec![frame(1, started("trn_1"))])),
        ms(1_600),
        "thinking is the pace between them, and the one a turn starts at"
    );
    assert_eq!(
        crate::activity::breath_of(&state()),
        ms(1_600),
        "and an idle session breathes at the same pace it would think at"
    );
}

/// The row is drawn on that period and not on a constant, which is the half
/// of the cue a unit test cannot see.
#[test]
fn arriving_words_hold_the_caret_steady_while_tools_breathe() {
    let (mut ui, now) = mid_turn();
    let at = later(now, 250);
    crate::theme::with(crate::painted::truecolor(), || {
        let mut drawn = |state: &bingo_sdk::SessionState| {
            leading_style(&solo(state), &mut ui, at, "esc to interrupt")
        };
        let on = |period| {
            theme::as_drawn(theme::breath(crate::clock::breath(
                at,
                Duration::from_millis(period),
            )))
        };
        assert_eq!(drawn(&answering()), theme::as_drawn(theme::glow()));
        assert_eq!(drawn(&calling()), on(2_200));
        assert_ne!(
            drawn(&answering()),
            drawn(&calling()),
            "and the two rhythms are told apart on the screen"
        );
    });
}

#[test]
fn the_input_box_glows_on_the_same_breath_and_is_dim_when_idle() {
    let (mut ui, now) = mid_turn();
    // The box is found by what it offers, and a busy box offers the two keys
    // that send (M68).
    let working = leading_style(&turning(), &mut ui, now, keys::BUSY_PLACEHOLDER);
    assert_eq!(
        working,
        theme::as_drawn(crate::activity::breathing(turning().viewed(), now))
    );

    let idle = solo(&state());
    assert_eq!(
        leading_style(&idle, &mut ui, now, keys::PLACEHOLDER),
        theme::as_drawn(theme::dim()),
        "and it is dim while nothing is happening"
    );
}

/// The wait row has no turn to read a pace from, so it takes the one that
/// already means the waiting is somebody else's (M82) — and the box stays
/// dim, because nothing is arriving *here*.
#[test]
fn the_wait_breathes_at_the_pace_a_tool_holds_a_turn_and_leaves_the_box_dim() {
    let (mut ui, now) = scene();
    let at = later(now, 250);
    let tree = waited_on(1);
    crate::theme::with(crate::painted::truecolor(), || {
        assert_eq!(
            leading_style(&tree, &mut ui, at, "Waiting for"),
            theme::as_drawn(theme::breath(crate::clock::breath(
                at,
                Duration::from_millis(2_200)
            ))),
        );
    });
    assert_eq!(
        leading_style(&tree, &mut ui, at, keys::PLACEHOLDER),
        theme::as_drawn(theme::dim()),
        "nothing is arriving here"
    );
}

#[test]
fn nothing_of_the_presence_is_on_screen_while_no_turn_runs() {
    let (mut ui, now) = mid_turn();
    let screen = screen(&solo(&state()), &mut ui, now);
    assert!(!screen.contains("esc to interrupt"), "{screen}");
    assert!(
        !screen.contains('▍') || screen.contains("█▀▀▀▀█"),
        "{screen}"
    );
}

// ---- the person's own gesture -------------------------------------------

/// The line the box's border wears, run by run, so a light crossing it can be
/// told from the one style it rests in.
fn border_runs(tree: &Tree, ui: &mut crate::ui::Ui, now: Now) -> Vec<Style> {
    crate::painted::painted(80, 24, tree, ui, now)
        .row(keys::PLACEHOLDER)
        .into_iter()
        .map(|(_, style)| style)
        .collect()
}

/// `⏎` is the most repeated gesture in the whole surface and had no answer at
/// all: the row simply existed. It runs one light along the box's border now,
/// over six frames, and the border is back where it was on the seventh. The
/// row read here is the box's own text row, so the two runs it carries are
/// the border's left edge and its right — which is the light's first cell and
/// its last, and so tells the direction as well as the fact.
#[test]
fn a_sent_line_runs_one_light_along_the_boxs_border() {
    let state = state();
    let tree = solo(&state);
    let (mut ui, now) = scene();
    let at_rest = crate::theme::with(crate::painted::truecolor(), || {
        border_runs(&tree, &mut ui, now)
    });

    write(&mut ui, &state, "say hello", now);
    crate::input::on_key(&mut ui, &tree, key(crossterm::event::KeyCode::Enter), now);
    assert_eq!(ui.sending(now), Some(0.0), "the light starts on the key");
    crate::theme::with(crate::painted::truecolor(), || {
        let mut lit = |ms| border_runs(&tree, &mut ui, later(now, ms));
        assert_ne!(lit(40), at_rest, "forty in, the near edge is lit");
        assert_eq!(lit(40).last(), at_rest.last(), "and the far edge is not");
        assert_ne!(lit(150), at_rest, "a hundred and fifty in, the far one is");
        assert_eq!(lit(150).first(), at_rest.first(), "and the near one is not");
        assert_eq!(lit(198), at_rest, "back at rest on the seventh frame");
    });

    // A second `⏎` is a second gesture: the light starts again rather than
    // carrying on from where the first one had got to.
    let half = later(now, 100);
    write(&mut ui, &state, "again", half);
    crate::input::on_key(&mut ui, &tree, key(crossterm::event::KeyCode::Enter), half);
    assert_eq!(ui.sending(half), Some(0.0));
    assert!(ui.sending(later(now, 298)).is_none(), "and ends six on");
}

#[test]
fn a_still_surface_sends_a_line_with_no_light_at_all() {
    let state = state();
    let tree = solo(&state);
    let (mut ui, now) = scene();
    let at_rest = border_runs(&tree, &mut ui, still(now));
    write(&mut ui, &state, "say hello", now);
    crate::input::on_key(&mut ui, &tree, key(crossterm::event::KeyCode::Enter), now);
    assert_eq!(ui.sending(still(now)), None);
    for ms in [0i64, 33, 100, 198] {
        assert_eq!(
            border_runs(&tree, &mut ui, still(later(now, ms))),
            at_rest,
            "at {ms} ms the border is exactly what a still surface draws"
        );
    }
}

// ---- streaming: the comet tail ------------------------------------------

/// An answer that is still arriving, with a tail long enough to ramp.
fn streaming() -> Tree {
    solo(&folded(vec![
        frame(1, started("trn_1")),
        frame(
            2,
            Event::ItemStarted {
                item: assistant("itm_1", "", ItemStatus::Running),
            },
        ),
        frame(
            3,
            Event::ItemDelta {
                item: bingo_sdk::ItemId::from_raw("itm_1"),
                n: 0,
                kind: bingo_sdk::DeltaKind::Text,
                data: "the last cells are still warm".into(),
            },
        ),
    ]))
}

/// The styles of the last eight cells of the answer, oldest first.
fn tail(ui: &mut crate::ui::Ui, now: Now, at: Now) -> Vec<Style> {
    let tree = streaming();
    // The block is rendered once at `now` so the cache dates its arrival
    // there, and again at `at` so the tail has aged.
    crate::painted::painted(80, 24, &tree, ui, now);
    let painted = crate::painted::painted(80, 24, &tree, ui, at);
    let mut runs: Vec<Style> = painted
        .row("still warm")
        .into_iter()
        .rev()
        .filter(|(text, _)| !text.trim().is_empty())
        .take(8)
        .map(|(_, style)| style)
        .collect();
    runs.reverse();
    runs
}

#[test]
fn a_comet_tail_cools_from_the_glow_to_the_text_behind_it() {
    let (mut ui, now) = mid_turn();
    crate::theme::with(crate::painted::truecolor(), || {
        let fresh = tail(&mut ui, now, now);
        assert_eq!(
            fresh.last().copied(),
            Some(theme::as_drawn(theme::comet(0.0))),
            "the cell that just landed wears the glow"
        );
        assert_ne!(fresh.first(), fresh.last(), "and the ramp is a ramp");

        let warm = tail(&mut ui, now, later(now, 75));
        assert_ne!(warm, fresh, "the whole tail cools as it ages");
        let cool = tail(&mut ui, now, later(now, 150));
        assert_ne!(cool, warm);
        let cold = tail(&mut ui, now, later(now, 180));
        assert_eq!(
            cold,
            tail(&mut ui, now, still(later(now, 180))),
            "and after 180 ms the row is drawn as a still surface draws it"
        );
    });
}

#[test]
fn the_tail_is_style_and_never_text() {
    let (mut ui, now) = mid_turn();
    let tree = streaming();
    assert_eq!(
        screen(&tree, &mut ui, now),
        screen(&tree, &mut ui, later(now, 90)),
        "what a `--print` would see does not move"
    );
}

// ---- a tool running, and finishing --------------------------------------

fn running_bash() -> Vec<bingo_sdk::Frame> {
    vec![
        frame(1, started("trn_1")),
        started_tool(2, running_tool("itm_1", "Bash", "compiling…")),
    ]
}

#[test]
fn a_live_bullet_pulses_between_presence_and_its_glow() {
    let tree = solo(&folded(running_bash()));
    let (mut ui, now) = mid_turn();
    crate::theme::with(crate::painted::truecolor(), || {
        let mut at = |ms| style_of(&tree, &mut ui, later(now, ms), "⏺");
        let (start, half, whole) = (at(0), at(600), at(1_200));
        assert_ne!(start, half, "it is somewhere else half a pulse in");
        assert_eq!(start, whole, "and back where it began after 1.2 s");
    });
}

// ---- the activity row ---------------------------------------------------

#[test]
fn a_turn_that_answers_at_once_never_flashes_a_row() {
    let tree = turning();
    let (mut ui, now) = scene();
    for ms in [0i64, 100, 200, 299] {
        let screen = screen(&tree, &mut ui, later(now, ms));
        assert!(!screen.contains("esc to interrupt"), "at {ms} ms: {screen}");
    }
    assert!(
        screen(&tree, &mut ui, later(now, 300)).contains("esc to interrupt"),
        "and it appears at 300 ms"
    );
}

#[test]
fn the_activity_row_says_a_verb_a_clock_and_what_the_turn_has_said() {
    let mut state = folded(vec![frame(1, started("trn_1"))]);
    state.apply(&frame(
        2,
        Event::TurnUsage {
            turn: bingo_sdk::TurnId::from_raw("trn_1"),
            usage: bingo_sdk::Usage {
                output_tokens: 1_200,
                ..Default::default()
            },
            context: Default::default(),
        },
    ));
    let (mut ui, now) = scene();
    let tree = solo(&state);
    let mut row = |ms| row(&tree, &mut ui, later(now, ms), "esc to interrupt");
    assert_eq!(
        row(4_000),
        "▍ Burrowing… (esc to interrupt · 4s · ↓ 1.2k)",
        "the verb is drawn from the turn's own id, so the row never changes its mind"
    );
    assert!(
        row(5_000).contains("· 5s ·"),
        "the clock ticks once a second"
    );
}

/// The answer to the key, on the key's own frame. The row keeps everything
/// that says the turn is still alive — the sparkle breathes, the clock ticks —
/// and loses only the hint, whose key has just been pressed.
#[test]
fn a_turn_asked_to_stop_says_so_and_keeps_its_sparkle_and_its_clock() {
    let state = folded(vec![frame(1, started("trn_1"))]);
    let tree = solo(&state);
    let (mut ui, now) = scene();
    ui.stop_asked = Some(bingo_sdk::TurnId::from_raw("trn_1"));
    let mut row = |ms| row(&tree, &mut ui, later(now, ms), crate::activity::STOPPING);
    assert_eq!(row(4_000), "▍ Stopping… (4s)");
    // The sparkle is at another of its four glyphs a second later, which is
    // the point: the row is still alive while it winds down.
    assert!(
        row(5_000).ends_with(" Stopping… (5s)"),
        "the clock ticks on: {}",
        row(5_000)
    );
    assert!(
        !row(4_000).contains("esc to interrupt"),
        "the key it named has been pressed"
    );
    assert_eq!(
        leading_style(&tree, &mut ui, later(now, 4_000), crate::activity::STOPPING),
        theme::as_drawn(theme::dim())
    );
}

/// The row that answers a key is never held back by the delay that spares a
/// fast turn its flash: a person who pressed `esc` at 40 ms must see it.
#[test]
fn stopping_outruns_the_activity_row_s_own_delay() {
    let state = folded(vec![frame(1, started("trn_1"))]);
    let tree = solo(&state);
    let (mut ui, now) = scene();
    assert!(
        !screen(&tree, &mut ui, later(now, 40)).contains("esc to interrupt"),
        "a turn this young says nothing on its own"
    );
    ui.stop_asked = Some(bingo_sdk::TurnId::from_raw("trn_1"));
    assert!(
        screen(&tree, &mut ui, later(now, 40)).contains(crate::activity::STOPPING),
        "but the answer to a key is not a cue, and does not wait"
    );
}

/// The flag is a fact about one keypress and one turn. The turn after it is
/// another turn, and reads as one.
#[test]
fn the_next_turn_is_not_the_one_that_was_stopped() {
    let state = folded(vec![frame(1, started("trn_2"))]);
    let (mut ui, now) = scene();
    ui.stop_asked = Some(bingo_sdk::TurnId::from_raw("trn_1"));
    let drawn = screen(&solo(&state), &mut ui, later(now, 4_000));
    assert!(drawn.contains("esc to interrupt"), "{drawn}");
    assert!(!drawn.contains(crate::activity::STOPPING), "{drawn}");
}

#[test]
fn every_verb_is_one_of_bingos_own() {
    let words = [
        "Kindling",
        "Stoking",
        "Smouldering",
        "Foraging",
        "Tracking",
        "Burrowing",
        "Sniffing around",
        "Pouncing",
        "Sifting",
        "Tinkering",
    ];
    let (mut ui, now) = mid_turn();
    for id in ["trn_1", "trn_2", "trn_9", "trn_ffff", "trn_01J"] {
        let state = folded(vec![frame(1, started(id))]);
        let row = screen(&solo(&state), &mut ui, now)
            .lines()
            .find(|line| line.contains("esc to interrupt"))
            .expect("the activity row")
            .to_string();
        assert!(words.iter().any(|word| row.contains(word)), "{id}: {row}");
    }
}

// ---- a card's guard -----------------------------------------------------

/// A permission the kernel guards for 400 ms, as it does every card.
fn guarded() -> bingo_sdk::Interaction {
    let mut asking = permission(Some("Edit(src/)"), None);
    asking.guard_until = Some(ts() + jiff::SignedDuration::from_millis(400));
    asking
}

#[test]
fn a_cards_rows_are_dim_until_the_guard_lifts_and_plain_the_moment_it_does() {
    let state = folded(vec![frame(1, opened(guarded()))]);
    let (mut ui, now) = settled();
    ui.dialog.focus_on(state.interactions.first());
    let tree = solo(&state);
    // The card is drawn from the wall clock the kernel stated the guard in.
    let mut at = |ms| style_of(&tree, &mut ui, later(now, ms), "Do you want to");
    assert_eq!(
        at(199),
        theme::as_drawn(theme::dim()),
        "a key pressed now would be dropped, and the card says so"
    );
    assert_eq!(
        at(200),
        theme::as_drawn(theme::text()),
        "and it brightens in one frame as the guard lifts"
    );
}

#[test]
fn a_card_reveals_top_down_and_a_sheet_slides_up() {
    let asked = folded(vec![frame(1, opened(permission(Some("Edit(src/)"), None)))]);
    let (mut ui, now) = scene();
    ui.dialog.focus_on(asked.interactions.first());
    let tree = solo(&asked);
    let card: Vec<String> = (0..4)
        .map(|f| screen(&tree, &mut ui, later(now, f * 33)))
        .collect();
    insta::assert_snapshot!("card_reveal", card.join("\n"));

    let (mut sheet, now) = scene();
    sheet
        .focus
        .show(crate::ui::Open::Rewind(Default::default()), now.instant);
    let plain = solo(&state());
    let frames: Vec<String> = (0..5)
        .map(|f| screen(&plain, &mut sheet, later(now, f * 33)))
        .collect();
    insta::assert_snapshot!("sheet_slide", frames.join("\n"));
}

#[test]
fn esc_runs_a_sheet_back_down_the_way_it_came() {
    let (mut ui, now) = scene();
    shown(&mut ui, crate::ui::Open::Rewind(Default::default()), now);
    ui.focus.close(now.instant);
    let tree = solo(&state());
    let closing: Vec<String> = (0..5)
        .map(|f| screen(&tree, &mut ui, later(now, f * 33)))
        .collect();
    assert_eq!(
        closing[4],
        screen(&tree, &mut crate::ui::Ui::new(Vec::new(), now.instant), now),
        "by the last frame it has gone"
    );
    insta::assert_snapshot!("sheet_close", closing.join("\n"));
}

// ---- notices ------------------------------------------------------------

#[test]
fn a_notice_arrives_out_of_dim_and_leaves_into_it() {
    let tree = solo(&state());
    let (mut ui, now) = scene();
    ui.notify(bingo_sdk::Level::Error, "unknown command: /x", now.instant);
    let mut at = |ms| style_of(&tree, &mut ui, later(now, ms), "unknown command");
    assert_eq!(
        at(0),
        theme::as_drawn(theme::fading(bingo_sdk::Level::Error, 0.0))
    );
    assert_eq!(
        at(33),
        theme::as_drawn(theme::fading(bingo_sdk::Level::Error, 0.5))
    );
    assert_eq!(
        at(66),
        theme::as_drawn(theme::bad()),
        "and it is there to read"
    );
    assert_eq!(
        at(4_099),
        theme::as_drawn(theme::fading(bingo_sdk::Level::Error, 0.5))
    );
}

#[test]
fn a_refused_line_is_named_after_the_reason_that_refused_it() {
    let tree = solo(&state());
    let (mut ui, now) = scene();
    ui.notify_about(
        bingo_sdk::Level::Error,
        "unknown command: /x".into(),
        "/x the whole line".into(),
        now.instant,
    );
    let screen = screen(&tree, &mut ui, later(now, 66));
    assert!(
        screen.contains("unknown command: /x · /x the whole line"),
        "{screen}"
    );
    assert_eq!(
        style_of(&tree, &mut ui, later(now, 66), "/x the whole"),
        theme::as_drawn(theme::dim()),
        "what it was about is said quietly"
    );
}

// ---- what wants a person ------------------------------------------------

#[test]
fn what_wants_a_person_alternates_on_the_second() {
    let tree = folded_tree(vec![
        child_frame(1, announced("reviewer")),
        child_frame(2, opened(child_permission())),
    ]);
    // The beat is the wall clock's own second, so the samples start on one.
    let (ui, now) = scene();
    // The status line itself: a card is up over the screen, and everything
    // behind it is dim by then (§3), so the slot is read where it is written.
    let at = |ms| {
        crate::status::styles(&crate::status::line(&tree, &ui, 80, later(now, ms)))
            .into_iter()
            .find(|(text, _)| text.contains(crate::face::face("reviewer").sigil()))
            .map(|(_, style)| style)
            .expect("the notice")
    };
    assert_eq!(at(0), crate::face::style("reviewer"));
    assert_eq!(at(999), at(0));
    assert_ne!(at(1_000), at(0), "on the second");
    assert_eq!(at(2_000), at(0));
}

/// The card's border is the fourth place that beat is said, and no longer
/// the exception: a card is on the screen exactly while it is unanswered, so
/// it asks for the whole of its life and stops asking by going.
#[test]
fn an_inline_ask_keeps_its_attention_marker_without_a_border() {
    let state = folded(vec![frame(1, opened(permission(None, None)))]);
    let (mut ui, now) = settled();
    ui.dialog.focus_on(state.interactions.first());
    let tree = solo(&state);
    for ms in [0, 1000, 2000] {
        assert!(draw_tree(80, 24, &tree, &mut ui, later(now, ms)).contains("needs you"));
    }
}

// ---- the context notice -------------------------------------------------

/// A session that has used `share` per cent of its compaction trigger.
fn context(share: u64) -> Tree {
    solo(&folded(vec![frame(
        1,
        Event::TurnUsage {
            turn: bingo_sdk::TurnId::from_raw("trn_1"),
            usage: Default::default(),
            context: bingo_sdk::ContextUsage {
                used: share * 1_000,
                window: 200_000,
                trigger: 100_000,
            },
        },
    )]))
}

/// The notice is a notice until the way out joins it, and then it is bad — the
/// same in every look since M73, because what it warms *from* is the
/// terminal's own dim and no colour of ours can be mixed towards that. The
/// turn lands where the words do, at 90 % (§4).
#[test]
fn the_context_count_is_always_there_and_warms_across_the_last_fifth() {
    let (mut ui, now) = scene();
    assert!(
        screen(&context(41), &mut ui, now).contains("41k/200k"),
        "the count is on the line as soon as a turn has run"
    );
    for look in [crate::painted::truecolor(), crate::painted::ascii()] {
        crate::theme::with(look, || {
            let mut at = |share| style_of(&context(share), &mut ui, now, "/200k");
            assert_eq!(at(79), theme::as_drawn(theme::dim()), "{:?}", look.colors);
            assert_eq!(at(89), theme::as_drawn(theme::dim()), "{:?}", look.colors);
            assert_eq!(
                at(90),
                theme::as_drawn(theme::bad()),
                "the way out joins it, and so does the colour"
            );
            assert_eq!(at(100), theme::as_drawn(theme::bad()), "{:?}", look.colors);
        });
    }
}

// ---- stepping into another session --------------------------------------

#[test]
fn the_transcript_crossfades_through_dim_into_the_session_stepped_into() {
    let mut tree = spawned_tree(busy_child("reviewer"));
    tree.show(&child_id());
    let (mut ui, now) = scene();
    let row = |ui: &mut crate::ui::Ui, at| {
        crate::painted::painted(80, 24, &tree, ui, at).row("Read(src/lib.rs)")
    };
    let plain = row(&mut ui, now);

    ui.switched = Some(now.instant);
    assert_ne!(
        row(&mut ui, now),
        plain,
        "everything recedes for the two frames of the crossfade"
    );
    assert_eq!(
        row(&mut ui, later(now, 66)),
        plain,
        "and comes back up as it was"
    );
}

// ---- reduced motion -----------------------------------------------------

#[test]
fn motion_off_holds_every_cue_at_its_resting_frame() {
    let tree = turning();
    let (mut ui, now) = mid_turn();
    let still = still(later(now, 450));
    assert_eq!(
        leading_style(&tree, &mut ui, still, "esc to interrupt"),
        theme::as_drawn(theme::presence()),
        "the sparkle rests at presence"
    );
    let drawn = row(&tree, &mut ui, still, "esc to interrupt");
    assert!(drawn.starts_with('▍'), "and holds its first glyph: {drawn}");

    // A live bullet and a tail are both absent rather than frozen.
    let bash = solo(&folded(running_bash()));
    assert_eq!(
        style_of(&bash, &mut ui, still, "⏺"),
        theme::as_drawn(theme::presence()),
        "a live bullet does not pulse"
    );
    let tail = crate::painted::painted(80, 24, &streaming(), &mut ui, still);
    assert!(
        tail.coloured("still warm")
            .iter()
            .all(|text| text.trim() == "▍"),
        "and streaming text carries no tail"
    );
}

#[test]
fn motion_off_puts_a_layer_up_whole_and_takes_it_away_at_once() {
    let (mut ui, now) = scene();
    ui.focus
        .show(crate::ui::Open::Rewind(Default::default()), now.instant);
    let still = still(now);
    let tree = solo(&state());
    assert_eq!(
        screen(&tree, &mut ui, still),
        screen(&tree, &mut ui, later(now, 200)),
        "a sheet is whole on its first frame"
    );
    ui.focus.close(now.instant);
    assert!(
        ui.focus.drawn(still).is_none(),
        "and gone on the frame it closes"
    );
}

#[test]
fn a_still_notice_is_said_at_once_and_still_leaves() {
    let tree = solo(&state());
    let (mut ui, now) = scene();
    ui.notify(bingo_sdk::Level::Error, "unknown command: /x", now.instant);
    assert_eq!(
        style_of(&tree, &mut ui, still(now), "unknown command"),
        theme::as_drawn(theme::bad()),
        "no fade, but the same words"
    );
    ui.expire(still(later(now, 4_132)));
    assert!(
        !screen(&tree, &mut ui, still(later(now, 4_132))).contains("unknown command"),
        "and it still goes when its time is up"
    );
}

// ---- what a completed turn leaves behind --------------------------------

#[test]
fn a_finished_turn_takes_the_whole_of_the_presence_with_it() {
    let state = folded(vec![
        frame(1, started("trn_1")),
        frame(2, completed("trn_1", TurnStatus::Completed)),
    ]);
    let (mut ui, now) = mid_turn();
    let screen = screen(&solo(&state), &mut ui, now);
    assert!(!screen.contains("esc to interrupt"), "{screen}");
    assert_eq!(
        leading_style(&solo(&state), &mut ui, now, keys::PLACEHOLDER),
        theme::as_drawn(theme::dim()),
        "and the box's border is dim again"
    );
}

/// §6's budget: nothing moves when nothing is happening. The loop's own
/// counter proves it (`run::tests`); this proves the frame is identical, so
/// even a redraw would cost nothing.
#[test]
fn an_idle_frame_is_the_same_frame_a_second_later() {
    let tree = solo(&folded(vec![frame(
        1,
        Event::ItemCompleted {
            item: assistant("itm_1", "All green.", ItemStatus::Completed),
        },
    )]));
    let (mut ui, now) = scene();
    let first = screen(&tree, &mut ui, now);
    for ms in [33i64, 150, 1_000, 4_000] {
        assert_eq!(first, screen(&tree, &mut ui, later(now, ms)), "at {ms} ms");
    }
}

/// A duration named in §6 is named once, here, so a change of rhythm is a
/// change of one line rather than a hunt.
#[test]
fn the_rhythms_are_the_ones_the_design_names() {
    assert_eq!(FRAME, Duration::from_millis(33));
    assert_eq!(crate::transcript::COMET, Duration::from_millis(180));
    assert_eq!(theme::PULSE, Duration::from_secs(1));
    assert_eq!(crate::ui::NOTICE, Duration::from_secs(4));
    assert_eq!(crate::ui::NOTICE_FADE, FRAME * 2);
    assert_eq!(crate::ui::SWITCH, FRAME * 2);
}
