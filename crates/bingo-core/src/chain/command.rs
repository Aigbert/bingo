//! Commands retain their supervised owner while links wrap their invocation.
use super::*;

pub(crate) async fn run(
    command: Arc<dyn Command>,
    args: String,
    cx: &CommandContext,
    hooks: &crate::turn::HookSet,
    hook_cx: &HookContext,
    report: Report,
) -> Result<CommandOutcome, KernelError> {
    let input = catalogue::CommandRun {
        name: command.spec().name,
        args,
    };
    json::dispatch(
        catalogue::COMMAND_RUN,
        &hooks.links().await,
        hook_cx,
        input,
        Next::new(|input: catalogue::CommandRun| {
            Box::pin(async move { command.run(&input.args, cx).await })
        }),
        report,
    )
    .await
}
