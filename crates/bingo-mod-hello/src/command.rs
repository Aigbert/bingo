use bingo_sdk::*;

pub struct Hello;
#[async_trait::async_trait]
impl Command for Hello {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            opens: false,
            name: "hello".into(),
            keys: vec!["ctrl+x".into()],
            hint: "Open the built-in fixture".into(),
            aliases: vec![],
            args: ArgSpec::None,
            instant: true,
            hidden: false,
            family: "fixture".into(),
        }
    }
    async fn run(&self, _: &str, _: &CommandContext) -> Result<CommandOutcome, KernelError> {
        Ok(CommandOutcome::View {
            view: super::drawing::tree(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_is_visible_and_has_one_default_key() {
        let spec = Hello.spec();
        assert!(!spec.hidden);
        assert_eq!(spec.keys, [KeyBinding::from("ctrl+x")]);
        assert_eq!(spec.name, "hello");
    }
}
