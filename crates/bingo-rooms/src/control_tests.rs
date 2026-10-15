use crate::tests::{Fleet, command_context};
use bingo_sdk::*;
use serde_json::json;
use std::sync::Arc;
async fn answer(verb: &str, wire: bool, peer: bool) -> Result<String, String> {
    let fleet = Fleet::default();
    let root = fleet.root();
    crate::RoomCommand
        .run("design reviewer", &command_context(&root, &fleet))
        .await
        .unwrap();
    let caller = if peer {
        fleet.child(&root, "scout")
    } else {
        root
    };
    let text = if verb == "purpose" {
        "Review storage"
    } else {
        "reviewer"
    };
    if wire {
        return bingo_services::rooms_control::wire(Arc::new(crate::RoomWriter::new(
            fleet.handle(),
        )))
        .call(
            if verb == "purpose" {
                "set_purpose"
            } else {
                verb
            },
            json!({"session":caller,"name":"design","agent":text,"text":text}),
        )
        .await
        .map(|v| v.as_str().unwrap().to_string())
        .map_err(|e| e.0);
    }
    let args = if verb == "close" {
        "close design".into()
    } else {
        format!("{verb} design {text}")
    };
    match crate::RoomCommand
        .run(&args, &command_context(&caller, &fleet))
        .await
    {
        Ok(CommandOutcome::Applied { message }) => Ok(message.unwrap_or_default()),
        Ok(other) => panic!("unexpected {other:?}"),
        Err(error) => Err(error.message),
    }
}
#[tokio::test]
async fn commands_and_services_share_receipts_and_refusals() {
    for verb in ["seat", "unseat", "purpose", "close"] {
        for peer in [false, true] {
            assert_eq!(
                answer(verb, false, peer).await,
                answer(verb, true, peer).await,
                "{verb}"
            );
        }
    }
}

#[tokio::test]
async fn close_has_exactly_the_spelling_guarded_by_the_person_contract() {
    let fleet = Fleet::default();
    let root = fleet.root();
    let cx = command_context(&root, &fleet);
    crate::RoomCommand
        .run("design reviewer", &cx)
        .await
        .unwrap();
    assert!(!crate::RoomCommand.spec().person_only("CLOSE design"));
    assert!(crate::RoomCommand.run("CLOSE design", &cx).await.is_err());
    assert!(!crate::control::rows(&fleet.handle(), &root).await.unwrap()[0].closed);
}

#[tokio::test]
async fn room_rows_name_the_holder_instead_of_showing_a_session_id() {
    let fleet = Fleet::default();
    let root = fleet.root();
    let child = fleet.child(&root, "reviewer");
    for (holder, name) in [(root, "parent"), (child, "reviewer")] {
        let id = fleet.room(&holder, "design");
        let rows = crate::control::rows(&fleet.handle(), &holder)
            .await
            .unwrap();
        assert_eq!(rows[0].id, id);
        assert_eq!(rows[0].holder, name);
    }
}

#[tokio::test]
async fn legacy_verb_names_still_resolve_as_explicit_arguments() {
    let fleet = Fleet::default();
    let root = fleet.root();
    for name in ["seat", "unseat", "purpose"] {
        let id = fleet.room(&root, name);
        let (resolved, _) = crate::door::resolve(&fleet.handle(), &fleet.summary(&root), name)
            .await
            .unwrap();
        assert_eq!(resolved, id);
        crate::RoomCommand
            .run(&format!("close {name}"), &command_context(&root, &fleet))
            .await
            .unwrap();
    }
}
