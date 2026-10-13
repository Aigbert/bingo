//! Each module exposes one private action endpoint.
use bingo_sdk::*;
pub(crate) struct Command(pub String);
#[async_trait::async_trait]
impl bingo_sdk::Command for Command {
    fn spec(&self) -> CommandSpec {
        CommandSpec::ui(&self.0)
    }
    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        bingo_plugin_host::host::handler(&self.0, args, cx).await
    }
}
