//! Commands retain their supervised owner while links wrap their invocation.
use super::*;

/// A command's outcome, and whether it is the command's own answer to the
/// arguments it was given rather than one a link rewrote or replaced.
pub(crate) struct Ran {
    pub(crate) outcome: CommandOutcome,
    pub(crate) own: bool,
}

pub(crate) async fn run(
    command: Arc<dyn Command>,
    args: String,
    cx: &CommandContext,
    hooks: &crate::turn::HookSet,
    hook_cx: &HookContext,
    report: Report,
) -> Result<Ran, KernelError> {
    let input = catalogue::CommandRun {
        name: command.spec().name,
        args: args.clone(),
    };
    // A link may rewrite the line, but the rewrite is the link's, not the person's.
    let asked = bingo_sdk::render::asked();
    let answered = Arc::new(Mutex::new(None));
    let own = answered.clone();
    let reply: catalogue::CommandReply = json::dispatch(
        catalogue::COMMAND_RUN,
        &hooks.links().await,
        hook_cx,
        input,
        Next::new(|input: catalogue::CommandRun| {
            Box::pin(async move {
                let original = input.args == args;
                let outcome =
                    bingo_sdk::render::under_input(asked && original, command.run(&input.args, cx))
                        .await?;
                if let Ok(mut own) = own.lock() {
                    *own = original.then(|| outcome.clone());
                }
                Ok(catalogue::CommandReply::Outcome(Box::new(outcome)))
            })
        }),
        report,
    )
    .await?;
    let outcome = CommandOutcome::from(reply);
    let own = answered
        .lock()
        .ok()
        .is_some_and(|answered| answered.as_ref() == Some(&outcome));
    Ok(Ran { outcome, own })
}
