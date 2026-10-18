use crate::cards::*;
use bingo_sdk::*;
use bingo_services::agents_control::AgentRow;
pub(crate) fn row(name: &str, state: &str) -> AgentRow {
    AgentRow {summary:serde_json::from_value(serde_json::json!({
        "id":"ses_child","title":name,"cwd":"/work","createdAt":"2026-10-15T00:00:00Z","updatedAt":"2026-10-15T00:00:00Z","busy":state=="working","asking":state=="asking","model":"sonnet","firstAsk":"review the fork cut"})).unwrap()}
}
#[test]
fn sketch_rows_include_asked_model_and_unknown_cost() {
    for (state, glyph) in [
        ("working", "●"),
        ("idle", "○"),
        ("stored", "○"),
        ("asking", "?"),
    ] {
        let card = agent_card(&row("reviewer", state), None, None);
        assert!(card.line.contains(glyph));
        assert!(card.line.contains("sonnet"));
        assert!(card.line.contains("$—"));
        assert!(
            card.details
                .iter()
                .any(|s| s.contains("review the fork cut"))
        );
    }
    let mut old = row("scout", "idle");
    old.summary.history.first_ask = None;
    assert!(
        agent_card(&old, None, None)
            .details
            .iter()
            .any(|s| s == "asked  —")
    );
}
#[test]
fn an_interrupted_fold_is_stopped_without_a_second_flag() {
    let row = row("scout", "idle");
    let mut live = SessionState::new(row.summary.clone());
    live.last_turn=Some(serde_json::from_value(serde_json::json!({"id":"trn_1","status":{"kind":"interrupted","reason":"userCancel"},"startedAt":"2026-10-15T00:00:00Z","endedAt":"2026-10-15T00:00:01Z"})).unwrap());
    assert!(
        agent_card(&row, Some(&live), None)
            .line
            .contains("■ stopped")
    );
}

#[test]
fn team_cards_include_seated_and_unseated_roles() {
    let seated = bingo_services::agents_control::TeamRow {
        role: "reviewer".into(),
        agent: Some("reviewer".into()),
        summary: Some(row("reviewer", "working").summary),
    };
    let card = team_card(&seated);
    assert_eq!(card.line, "reviewer   working");
    assert_eq!(card.details, ["agent    reviewer", "session  ses_child"]);
    let unseated = bingo_services::agents_control::TeamRow {
        role: "scout".into(),
        agent: None,
        summary: None,
    };
    let card = team_card(&unseated);
    assert_eq!(card.line, "scout   not seated");
    assert_eq!(card.details, ["agent    -", "session  -"]);
}

#[test]
fn a_card_names_only_a_running_or_waiting_call() {
    let row = row("reviewer", "working");
    let mut state = SessionState::new(row.summary.clone());
    for (status, asked, expected) in [
        ("pending", false, None),
        ("pending", true, Some("Edit")),
        ("running", false, Some("Edit")),
        ("interrupted", true, None),
    ] {
        let mut value = serde_json::json!({"id":"call","status":status,
            "startedAt":"2026-10-15T00:00:00Z","body":{"kind":"toolCall","callId":"c","name":"Edit","input":{}}});
        if asked {
            value["body"]["askedAt"] = serde_json::json!("2026-10-15T00:00:01Z");
        }
        if status == "interrupted" {
            value["completedAt"] = serde_json::json!("2026-10-15T00:00:02Z");
        }
        state.transcript.apply(&Event::ItemUpdated {
            item: serde_json::from_value(value).unwrap(),
        });
        let card = agent_card(&row, Some(&state), None);
        let tool = card
            .details
            .iter()
            .find_map(|line| line.strip_prefix("tool   "));
        assert_eq!(tool, expected, "{status}/{asked}");
    }
}
