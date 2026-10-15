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
    let spec = command.spec();
    let entry = point(&spec, &args);
    let input = catalogue::CommandRun {
        name: spec.name,
        args: args.clone(),
    };
    // A link may rewrite the line, but the rewrite is the link's, not the
    // person's, and not a key the built-in's own drawing handed out.
    let asked = bingo_sdk::render::asked();
    let pressed = bingo_sdk::render::vouched();
    let answered = Arc::new(Mutex::new(None));
    let own = answered.clone();
    let reply: catalogue::CommandReply = json::dispatch(
        &entry,
        &hooks.links().await,
        hook_cx,
        input,
        Next::new(|input: catalogue::CommandRun| {
            Box::pin(async move {
                let original = input.args == args;
                let run = bingo_sdk::render::under_press(
                    pressed && original,
                    command.run(&input.args, cx),
                );
                let outcome = bingo_sdk::render::under_input(asked && original, run).await?;
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

/// A line only the person may run keeps its arguments through every link,
/// as every line keeps its command.
fn point(spec: &CommandSpec, args: &str) -> catalogue::Entry {
    if spec.person_only(args) {
        catalogue::COMMAND_RUN.pinning(&["/command", "/args"])
    } else {
        catalogue::COMMAND_RUN.clone()
    }
}
