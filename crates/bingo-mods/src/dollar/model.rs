//! Side questions derive requests without the main turn's contributors.
use bingo_sdk::*;

pub fn classify_request(_: &str, _: &[String]) -> ModelRequest {
    request(Vec::new())
}
pub fn pick<'a>(_: &str, _: &'a [String]) -> Option<&'a str> {
    None
}
pub fn fork_request(_: &[Item], _: &str) -> ModelRequest {
    request(Vec::new())
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
