//! A model or effort switch is a line in the transcript whichever client asked
//! for it: the receipt is recorded, and its ack still carries the words.
use bingo_sdk::*;
use serde_json::Value;
pub struct Receipts;
#[async_trait::async_trait]
impl Interceptor for Receipts {
    fn id(&self) -> &str {
        "models.receipts"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher::at(catalogue::COMMAND_RUN)
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        _: &HookContext,
        value: Value,
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let switches = matches!(value["command"].as_str(), Some("model" | "think"));
        let value = next.run(value).await?;
        if switches { outcome(value) } else { Ok(value) }
    }
}
fn outcome(value: Value) -> Result<Value, KernelError> {
    if value["kind"] != "applied" {
        return Ok(value);
    }
    let Some(message) = value["message"].as_str() else {
        return Ok(value);
    };
    let text = if message.starts_with('⇄') {
        message.to_owned()
    } else if let Some(effort) = message.strip_prefix("thinking: ") {
        format!("⇄ thinking {effort}")
    } else {
        return Ok(value);
    };
    serde_json::to_value(CommandOutcome::Record {
        body: Box::new(ItemBody::Notice {
            level: Level::Info,
            code: "model.choice".into(),
            text,
        }),
    })
    .map_err(crate::error)
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn choice_receipts_are_recorded_and_other_outcomes_are_unchanged() {
        let value = outcome(json!({"kind":"applied","message":"⇄ small · here"})).unwrap();
        assert!(
            matches!(serde_json::from_value::<CommandOutcome>(value).unwrap(), CommandOutcome::Record { body } if matches!(body.as_ref(), ItemBody::Notice {text, ..} if text == "⇄ small · here"))
        );
        let other = json!({"kind":"applied","message":"pinned"});
        assert_eq!(outcome(other.clone()).unwrap(), other);
    }
}
