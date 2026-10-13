#![allow(clippy::unwrap_used, clippy::expect_used)]

use bingo_sdk::{Event, ItemBody};

fn fixture_name(value: &Event) -> (&'static str, &'static str) {
    match value {
        Event::Unknown { .. } => panic!("known fixture became opaque"),
        Event::SessionUpdated { .. } => (
            "sessionUpdated",
            include_str!("../fixtures/record/event/sessionUpdated.json"),
        ),
        Event::SessionClosed { .. } => (
            "sessionClosed",
            include_str!("../fixtures/record/event/sessionClosed.json"),
        ),
        Event::TurnStarted { .. } => (
            "turnStarted",
            include_str!("../fixtures/record/event/turnStarted.json"),
        ),
        Event::TurnRetrying { .. } => (
            "turnRetrying",
            include_str!("../fixtures/record/event/turnRetrying.json"),
        ),
        Event::TurnUsage { .. } => (
            "turnUsage",
            include_str!("../fixtures/record/event/turnUsage.json"),
        ),
        Event::UsageCharged { .. } => (
            "usageCharged", include_str!("../fixtures/record/event/usageCharged.json"),
        ),
        Event::TurnCompleted { .. } => (
            "turnCompleted",
            include_str!("../fixtures/record/event/turnCompleted.json"),
        ),
        Event::ItemStarted { .. } => (
            "itemStarted",
            include_str!("../fixtures/record/event/itemStarted.json"),
        ),
        Event::ItemDelta { .. } => (
            "itemDelta",
            include_str!("../fixtures/record/event/itemDelta.json"),
        ),
        Event::ItemUpdated { .. } => (
            "itemUpdated",
            include_str!("../fixtures/record/event/itemUpdated.json"),
        ),
        Event::ItemCompleted { .. } => (
            "itemCompleted",
            include_str!("../fixtures/record/event/itemCompleted.json"),
        ),
        Event::QueueChanged { .. } => (
            "queueChanged",
            include_str!("../fixtures/record/event/queueChanged.json"),
        ),
        Event::InteractionOpened { .. } => (
            "interactionOpened",
            include_str!("../fixtures/record/event/interactionOpened.json"),
        ),
        Event::InteractionResolved { .. } => (
            "interactionResolved",
            include_str!("../fixtures/record/event/interactionResolved.json"),
        ),
        Event::InteractionCancelled { .. } => (
            "interactionCancelled",
            include_str!("../fixtures/record/event/interactionCancelled.json"),
        ),
        Event::IntentAck { .. } => (
            "intentAck",
            include_str!("../fixtures/record/event/intentAck.json"),
        ),
        Event::Compacted { .. } => (
            "compacted",
            include_str!("../fixtures/record/event/compacted.json"),
        ),
        Event::Rewound { .. } => (
            "rewound",
            include_str!("../fixtures/record/event/rewound.json"),
        ),
        Event::ConfigChanged { .. } => (
            "configChanged",
            include_str!("../fixtures/record/event/configChanged.json"),
        ),
        Event::CatalogChanged { .. } => (
            "catalogChanged",
            include_str!("../fixtures/record/event/catalogChanged.json"),
        ),
        Event::Notice { .. } => (
            "notice",
            include_str!("../fixtures/record/event/notice.json"),
        ),
        Event::Extension { .. } => (
            "extension",
            include_str!("../fixtures/record/event/extension.json"),
        ),
        Event::Signal { .. } => (
            "signal",
            include_str!("../fixtures/record/event/signal.json"),
        ),
        Event::Lagged { .. } => (
            "lagged",
            include_str!("../fixtures/record/event/lagged.json"),
        ),
    }
}

#[test]
fn every_event_variant_has_a_golden_fixture() {
    for entry in std::fs::read_dir(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/record/event"
    ))
    .unwrap()
    {
        let path = entry.unwrap().path();
        let wire = std::fs::read_to_string(&path).unwrap();
        let value: Event = serde_json::from_str(&wire).unwrap();
        let (name, pinned) = fixture_name(&value);
        assert_eq!(path.file_stem().unwrap().to_str().unwrap(), name);
        assert_eq!(wire, pinned);
        assert_eq!(serde_json::to_string(&value).unwrap(), wire.trim_end());
    }
}

fn item_fixture_name(value: &ItemBody) -> (&'static str, &'static str) {
    match value {
        ItemBody::Unknown { .. } => panic!("known fixture became opaque"),
        ItemBody::User { .. } => ("user", include_str!("../fixtures/record/item/user.json")),
        ItemBody::Assistant { .. } => (
            "assistant",
            include_str!("../fixtures/record/item/assistant.json"),
        ),
        ItemBody::Reasoning { .. } => (
            "reasoning",
            include_str!("../fixtures/record/item/reasoning.json"),
        ),
        ItemBody::ToolCall { .. } => (
            "toolCall",
            include_str!("../fixtures/record/item/toolCall.json"),
        ),
        ItemBody::Shell { .. } => ("shell", include_str!("../fixtures/record/item/shell.json")),
        ItemBody::Action { .. } => (
            "action",
            include_str!("../fixtures/record/item/action.json"),
        ),
        ItemBody::Compaction { .. } => (
            "compaction",
            include_str!("../fixtures/record/item/compaction.json"),
        ),
        ItemBody::Rewind { .. } => (
            "rewind",
            include_str!("../fixtures/record/item/rewind.json"),
        ),
        ItemBody::Interruption { .. } => (
            "interruption",
            include_str!("../fixtures/record/item/interruption.json"),
        ),
        ItemBody::Notice { .. } => (
            "notice",
            include_str!("../fixtures/record/item/notice.json"),
        ),
        ItemBody::QuestionAnswer { .. } => (
            "questionAnswer",
            include_str!("../fixtures/record/item/questionAnswer.json"),
        ),
        ItemBody::PermissionReceipt { .. } => (
            "permissionReceipt",
            include_str!("../fixtures/record/item/permissionReceipt.json"),
        ),
        ItemBody::Asset { .. } => ("asset", include_str!("../fixtures/record/item/asset.json")),
    }
}

#[test]
fn every_item_variant_has_a_golden_fixture() {
    for entry in
        std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/record/item")).unwrap()
    {
        let path = entry.unwrap().path();
        let wire = std::fs::read_to_string(&path).unwrap();
        let value: ItemBody = serde_json::from_str(&wire).unwrap();
        let (name, pinned) = item_fixture_name(&value);
        assert_eq!(path.file_stem().unwrap().to_str().unwrap(), name);
        assert_eq!(wire, pinned);
        assert_eq!(serde_json::to_string(&value).unwrap(), wire.trim_end());
    }
}

#[test]
fn quiet_tool_calls_keep_their_flag_and_old_calls_keep_their_shape() {
    for wire in [
        include_str!("../fixtures/record/item/toolCall.json"),
        include_str!("../fixtures/record/compat/quietToolCall.json"),
    ] {
        let body: ItemBody = serde_json::from_str(wire).unwrap();
        assert_eq!(serde_json::to_string(&body).unwrap(), wire.trim_end());
    }
}
