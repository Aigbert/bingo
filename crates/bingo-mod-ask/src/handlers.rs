use crate::States;
use bingo_sdk::*;
use serde_json::Value;
pub struct Handler(pub States);
#[async_trait::async_trait]
impl Command for Handler {
    fn spec(&self) -> CommandSpec {
        CommandSpec::ui("ask")
    }
    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let args: Value = serde_json::from_str(args).map_err(error)?;
        let id = args["handler"].as_str().unwrap_or_default();
        let session: SessionId = serde_json::from_value(args["session"].clone()).map_err(error)?;
        let key = args["value"]["key"].as_str().unwrap_or_default();
        if args["event"] != "press" || key == "esc" || key == "ctrl+c" {
            return Ok(handled(false));
        }
        let attachment = cx
            .host
            .open(
                SessionSelector::ById {
                    id: session.clone(),
                },
                ClientIdentity {
                    name: "ask".into(),
                    surface: "terminal".into(),
                },
                OpenOptions::default(),
            )
            .await?;
        let Some(open) = attachment
            .snapshot
            .interactions
            .iter()
            .find(|open| open.id.as_str() == id)
        else {
            return Ok(handled(false));
        };
        let (used, answer) = answer(&self.0, &session, &attachment.snapshot, open, key)?;
        if let Some(answer) = answer {
            attachment.handle.answer(
                IntentId::mint(),
                open.id.clone(),
                answer,
                Activation::Keyboard,
            );
        }
        cx.host
            .invalidate(
                &session,
                vec![args["request"].as_str().unwrap_or_default().into()],
            )
            .await?;
        Ok(handled(used))
    }
}
fn answer(
    states: &States,
    session: &SessionId,
    snapshot: &SessionState,
    open: &Interaction,
    key: &str,
) -> Result<(bool, Option<Answer>), KernelError> {
    let (questions, form, field_layout) = match &open.kind {
        InteractionKind::Question(question) => (std::slice::from_ref(question), false, false),
        InteractionKind::Form { questions, title } => (
            questions.as_slice(),
            true,
            crate::state::fields(questions.len(), title.is_some()),
        ),
        _ => return Ok((false, None)),
    };
    let mut states = states.lock().map_err(error)?;
    states.retain(|(owner, id), _| {
        owner != session
            || snapshot
                .interactions
                .iter()
                .any(|open| open.id.as_str() == id)
    });
    let (used, answer) = states
        .entry((session.clone(), open.id.to_string()))
        .or_default()
        .key(questions, field_layout, key);
    let answer = answer.map(|answer| {
        if form && !field_layout {
            Answer::Form {
                answers: vec![answer],
            }
        } else {
            answer
        }
    });
    Ok((used, answer))
}

fn handled(handled: bool) -> CommandOutcome {
    CommandOutcome::Handled { handled }
}
fn error(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::InvalidInput, error.to_string())
}
