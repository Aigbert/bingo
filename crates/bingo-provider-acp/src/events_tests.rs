use super::*;
use crate::fixtures;
use agent_client_protocol_schema::v1::SessionNotification;

fn update(value: Value) -> SessionUpdate {
    serde_json::from_value::<SessionNotification>(value)
        .unwrap()
        .update
}
fn turn(updates: Vec<Value>) -> Vec<AgentEvent> {
    let mut mapper = Mapper::default();
    let mut events: Vec<_> = updates
        .into_iter()
        .flat_map(|v| mapper.update(update(v)))
        .collect();
    events
        .extend(mapper.finish(&serde_json::from_value(fixtures::prompt_response_bare()).unwrap()));
    events
}
#[test]
fn speech_and_thoughts_remain_model_events() {
    let events = turn(vec![
        fixtures::update_agent_message_chunk(),
        fixtures::update_agent_message_chunk_more(),
    ]);
    let text: String = events
        .iter()
        .filter_map(|e| match e {
            AgentEvent::Said(ModelEvent::TextDelta { delta, .. }) => Some(delta.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text, "Renaming the module.");
    assert!(matches!(
        events.last(),
        Some(AgentEvent::Said(ModelEvent::Finish { .. }))
    ));
}
#[test]
fn external_calls_use_call_update_done_and_never_reasoning() {
    let start = serde_json::json!({"sessionId":"s","update":{"sessionUpdate":"tool_call","toolCallId":"edit-1","title":"Edit note.md","kind":"edit","status":"pending","rawInput":{"path":"note.md"}}});
    let end = serde_json::json!({"sessionId":"s","update":{"sessionUpdate":"tool_call_update","toolCallId":"edit-1","status":"completed","content":[{"type":"content","content":{"type":"text","text":"written"}}]}});
    let events = turn(vec![start, end]);
    assert!(matches!(&events[0], AgentEvent::Call(call) if call.kind == CallKind::Edit));
    assert!(matches!(&events[1], AgentEvent::Update(call) if call.title == "Edit note.md"));
    assert!(
        matches!(&events[2], AgentEvent::Done { call, output } if call.id == "edit-1" && output.parts == vec![bingo_sdk::ContentPart::text("written")])
    );
    assert!(!events.iter().any(|e| matches!(
        e,
        AgentEvent::Said(ModelEvent::ReasoningStart { .. } | ModelEvent::ToolCall { .. })
    )));
}
#[test]
fn a_reading_beats_a_bill_and_a_cut_precedes_the_new_reading() {
    let mut mapper = Mapper::holding(Some(Reading {
        used: 900,
        window: 1000,
    }));
    let reading = serde_json::json!({"sessionId":"s","update":{"sessionUpdate":"usage_update","used":100,"size":1000}});
    assert_eq!(
        mapper.update(update(reading)),
        vec![
            AgentEvent::Said(ModelEvent::Compacted {
                before: 900,
                after: 100
            }),
            AgentEvent::Said(ModelEvent::Context {
                used: 100,
                window: 1000
            }),
        ]
    );
}
#[test]
fn unknown_kinds_are_untrusted_other_calls() {
    let call = serde_json::json!({"sessionId":"s","update":{"sessionUpdate":"tool_call","toolCallId":"x","title":"Other","kind":"other","status":"completed"}});
    assert!(matches!(&turn(vec![call])[0], AgentEvent::Call(call) if call.kind == CallKind::Other));
}

#[test]
fn recorded_adapter_updates_keep_their_meaning() {
    let events = turn(vec![
        fixtures::update_agent_message_chunk_unkeyed(),
        fixtures::update_tool_call(),
        fixtures::update_tool_call_completed(),
        fixtures::update_tool_call_failed(),
        fixtures::update_usage(),
        fixtures::update_usage_bare(),
        fixtures::update_usage_after_compaction(),
        fixtures::update_compacting_banner(),
        fixtures::update_compacted_banner(),
        fixtures::update_compaction_failed_banner(),
        fixtures::update_plan(),
        fixtures::update_available_commands(),
        fixtures::update_current_mode(),
    ]);
    assert!(events.iter().any(|e| matches!(e, AgentEvent::Done { .. })));
    assert!(
        events
            .iter()
            .any(|e| matches!(e, AgentEvent::Said(ModelEvent::Context { .. })))
    );
    let mut mapper = Mapper::default();
    let events =
        mapper.finish(&serde_json::from_value(fixtures::prompt_response_cancelled()).unwrap());
    assert!(matches!(
        events.last(),
        Some(AgentEvent::Said(ModelEvent::Finish { .. }))
    ));
}
