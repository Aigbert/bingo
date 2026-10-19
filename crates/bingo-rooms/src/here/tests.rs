use super::*;
use crate::tests::{Fleet, command_context};

/// A root with two teammates and `#design` under it, scout seated.
async fn design(fleet: &Fleet) -> SessionId {
    let root = fleet.root();
    fleet.child(&root, "scout");
    fleet.child(&root, "ada");
    crate::RoomCommand
        .run("design scout", &command_context(&root, fleet))
        .await
        .expect("a room this crate can open");
    fleet.titled("#design").expect("the room was opened")
}

async fn typed(fleet: &Fleet, room: &SessionId, verb: Verb, args: &str) -> CommandOutcome {
    HereCommand(verb)
        .run(args, &command_context(room, fleet))
        .await
        .expect("a verb this room answers")
}

fn said(outcome: &CommandOutcome) -> (Option<String>, String) {
    let CommandOutcome::Surface { action, fallback } = outcome else {
        panic!("an answer, not {outcome:?}");
    };
    (
        action.args["text"].as_str().map(str::to_string),
        fallback.clone().unwrap_or_default(),
    )
}

#[tokio::test]
async fn a_bare_verb_writes_itself_back_with_what_it_needs() {
    let fleet = Fleet::default();
    let room = design(&fleet).await;
    assert_eq!(
        said(&typed(&fleet, &room, Verb::Seat, "").await),
        (Some("/seat ".into()), "seat who? · ada".into())
    );
    assert_eq!(
        said(&typed(&fleet, &room, Verb::Unseat, " ").await),
        (Some("/unseat ".into()), "unseat who? · scout".into())
    );
    assert_eq!(
        said(&typed(&fleet, &room, Verb::Close, "").await),
        (Some("/close ".into()), "say why the room closes".into())
    );
}

#[tokio::test]
async fn a_name_that_does_not_fit_is_told_why() {
    let fleet = Fleet::default();
    let room = design(&fleet).await;
    let refused = |outcome: CommandOutcome| said(&outcome).1;
    assert_eq!(
        refused(typed(&fleet, &room, Verb::Seat, "ghost").await),
        "no agent called ghost"
    );
    assert_eq!(
        refused(typed(&fleet, &room, Verb::Seat, "scout").await),
        "scout is already here"
    );
    assert_eq!(
        refused(typed(&fleet, &room, Verb::Unseat, "ada").await),
        "ada is not in this room"
    );
    assert_eq!(fleet.members(&room), ["scout"]);
}

#[tokio::test]
async fn seating_unseating_and_closing_change_the_room_on_screen() {
    let fleet = Fleet::default();
    let room = design(&fleet).await;
    let applied = CommandOutcome::Applied { message: None };
    assert_eq!(typed(&fleet, &room, Verb::Seat, "ada").await, applied);
    assert_eq!(fleet.members(&room), ["scout", "ada"]);
    assert_eq!(typed(&fleet, &room, Verb::Unseat, "scout").await, applied);
    assert_eq!(fleet.members(&room), ["ada"]);
    assert_eq!(
        typed(&fleet, &room, Verb::Close, "we chose the queue").await,
        applied
    );
    let state = fleet.state(&room);
    assert!(room::closed_of(&state));
    assert_eq!(
        room::Closed::of_state(&state).and_then(|closed| closed.why),
        Some("we chose the queue".into())
    );
    let error = HereCommand(Verb::Seat)
        .run("scout", &command_context(&room, &fleet))
        .await
        .expect_err("a closed room takes no verb");
    assert!(error.message.contains("this room is closed"), "{error}");
}

#[tokio::test]
async fn outside_a_room_the_verbs_say_where_they_work() {
    let fleet = Fleet::default();
    let root = fleet.root();
    let error = HereCommand(Verb::Seat)
        .run("scout", &command_context(&root, &fleet))
        .await
        .expect_err("home is not a room");
    assert!(error.message.contains("inside a room"), "{error}");
}

#[test]
fn the_three_verbs_are_listed_in_the_rooms_family() {
    for verb in [Verb::Seat, Verb::Unseat, Verb::Close] {
        let spec = HereCommand(verb).spec();
        assert_eq!(spec.family, FAMILY);
        assert!(spec.instant);
    }
}
