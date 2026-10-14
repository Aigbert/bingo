use bingo_sdk::*;
use serde_json::Value;

pub struct Handler;
fn handled(args: &Value) -> bool {
    args["handler"] == "keys" && args["event"] == "press" && args["value"]["key"] == "x"
}
#[async_trait::async_trait]
impl Command for Handler {
    fn spec(&self) -> CommandSpec {
        CommandSpec::ui("hello")
    }
    async fn run(&self, args: &str, _: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let args: Value = serde_json::from_str(args)
            .map_err(|error| KernelError::new(ErrorCode::InvalidInput, error.to_string()))?;
        Ok(CommandOutcome::Handled {
            handled: handled(&args),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn only_the_minted_handler_and_its_key_are_handled() {
        for (token, event, key, expected) in [
            ("keys", "press", "x", true),
            ("old", "press", "x", false),
            ("keys", "input", "x", false),
            ("keys", "press", "y", false),
        ] {
            assert_eq!(
                handled(&json!({"handler":token,"event":event,"value":{"key":key}})),
                expected
            );
        }
    }
}
