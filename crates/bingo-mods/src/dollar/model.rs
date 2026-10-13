//! Side questions derive requests without the main turn's contributors.
use bingo_sdk::*;

pub fn classify_request(text: &str, labels: &[String]) -> ModelRequest {
    let mut request = request(vec![Message::text(
        Role::User,
        serde_json::json!({"text":text,"labels":labels}).to_string(),
    )]);
    request.system.push(SystemBlock {
        text: "Choose one supplied label for the text. Return that label alone.".into(),
        cache: false,
    });
    request
}
pub fn pick<'a>(answer: &str, labels: &'a [String]) -> Option<&'a str> {
    labels
        .iter()
        .find(|label| label.as_str() == answer.trim())
        .map(String::as_str)
}
pub fn fork_request(history: &[Item], prompt: &str) -> ModelRequest {
    let mut messages = Vec::new();
    for item in history
        .iter()
        .filter(|item| item.is_terminal() && !item.external())
    {
        history_item(&mut messages, &item.body);
    }
    messages.push(Message::text(Role::User, prompt));
    request(messages)
}
fn history_item(messages: &mut Vec<Message>, item: &ItemBody) {
    match item {
        ItemBody::User { parts, .. } => messages.push(Message::user(parts.clone())),
        ItemBody::Assistant { text } => messages.push(Message::text(Role::Assistant, text)),
        ItemBody::ToolCall {
            call_id,
            name,
            input,
            output: Some(output),
            ..
        } => {
            messages.push(Message::assistant(vec![ContentPart::ToolUse {
                id: call_id.clone(),
                name: name.clone(),
                input: input.clone(),
            }]));
            messages.push(Message::user(vec![ContentPart::ToolResult {
                tool_use_id: call_id.clone(),
                parts: output.parts.clone(),
                is_error: output.is_error,
            }]));
        }
        ItemBody::Compaction { summary, .. } => {
            messages.clear();
            messages.push(Message::text(Role::User, summary));
        }
        ItemBody::QuestionAnswer {
            question, answer, ..
        } => messages.push(Message::text(Role::User, format!("{question}\n{answer}"))),
        _ => (),
    }
}
fn request(messages: Vec<Message>) -> ModelRequest {
    ModelRequest {
        model: String::new(),
        max_tokens: 4096,
        system: Vec::new(),
        messages,
        tools: Vec::new(),
        reasoning: None,
        session: None,
        provider_options: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classification_preserves_labels_and_refuses_an_unlisted_answer() {
        let labels = vec!["yes".into(), "no".into()];
        assert_eq!(pick(" yes\n", &labels), Some("yes"));
        assert_eq!(pick("maybe", &labels), None);
        let request = classify_request("a question", &labels);
        let text = serde_json::to_string(&request.messages).unwrap();
        for word in ["a question", "yes", "no"] {
            assert!(text.contains(word));
        }
        assert!(request.tools.is_empty());
        assert!(request.session.is_none());
    }
    #[test]
    fn a_fork_uses_history_then_the_new_prompt_without_a_main_conversation() {
        let item: Item = serde_json::from_value(serde_json::json!({"id":"i","status":"completed","startedAt":"2026-01-01T00:00:00Z",
            "body":{"kind":"user","parts":[{"type":"text","text":"history"}],"origin":{"surface":"test"}}})).unwrap();
        let request = fork_request(&[item], "side question");
        assert_eq!(
            request.messages,
            vec![
                Message::text(Role::User, "history"),
                Message::text(Role::User, "side question")
            ]
        );
        assert!(request.tools.is_empty());
        assert!(request.session.is_none());
    }
}

pub async fn classify(
    caller: &bingo_plugin_host::ops::Caller,
    input: super::params::Classify,
) -> Result<serde_json::Value, String> {
    if input.labels.is_empty() || input.labels.iter().any(|label| label.trim().is_empty()) {
        return Err("model.classify requires nonempty labels".into());
    }
    let events = complete(
        caller,
        &input.session,
        input.turn.as_ref(),
        classify_request(&input.text, &input.labels),
    )
    .await?;
    let text: String = events
        .iter()
        .filter_map(|event| match event {
            ModelEvent::TextDelta { delta, .. } => Some(delta.as_str()),
            _ => None,
        })
        .collect();
    pick(&text, &input.labels)
        .map(|label| serde_json::json!(label))
        .ok_or_else(|| "model.classify returned no supplied label".into())
}
pub async fn complete(
    caller: &bingo_plugin_host::ops::Caller,
    session: &SessionId,
    turn: Option<&TurnId>,
    request: ModelRequest,
) -> Result<Vec<ModelEvent>, String> {
    caller
        .host
        .complete(
            session,
            turn,
            Completion {
                plugin: caller.plugin.clone(),
                request,
            },
        )
        .await
        .map_err(|error| error.to_string())
}
