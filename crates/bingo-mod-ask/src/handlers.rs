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
        let props: component::QuestionProps =
            serde_json::from_value(args["props"].clone()).map_err(error)?;
        let (used, answer) = answer(&self.0, &session, id, &props, key)?;
        cx.host
            .invalidate(
                &session,
                vec![args["request"].as_str().unwrap_or_default().into()],
            )
            .await?;
        Ok(answer.map_or_else(|| handled(used), |answer| CommandOutcome::Answer { answer }))
    }
}
fn answer(
    states: &States,
    session: &SessionId,
    id: &str,
    props: &component::QuestionProps,
    key: &str,
) -> Result<(bool, Option<Answer>), KernelError> {
    let questions = &props.questions;
    let form = props.form;
    let field_layout =
        form && crate::state::fields(questions.len(), props.metadata_source.is_some());
    let mut states = states.lock().map_err(error)?;
    let Some(state) = states.get_mut(&(session.clone(), id.into())) else {
        return Ok((false, None));
    };
    let (used, answer) = state.key(questions, field_layout, key);
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn a_key_returns_the_answer_without_opening_a_session() {
        let states = States::default();
        let session = SessionId::from_raw("s");
        states
            .lock()
            .unwrap()
            .insert((session.clone(), "q".into()), Default::default());
        let args = json!({"handler":"q","session":"s","event":"press","request":"q", "value":{"key":"2"},
            "props":{"tool":"", "questions":[{"question":"Route?", "options":[{"id":"a","label":"Alpha"},{"id":"b","label":"Beta"}],"multi":false,"freeText":false}]}});
        let handler = Handler(states);
        let args = args.to_string();
        let cx = CommandContext {
            session,
            cwd: "/unused".into(),
            host: bingo_sdk::testing::NoHost::handle(),
            cancel: Default::default(),
        };
        let mut future = std::pin::pin!(handler.run(&args, &cx));
        let std::task::Poll::Ready(result) = std::future::Future::poll(
            future.as_mut(),
            &mut std::task::Context::from_waker(std::task::Waker::noop()),
        ) else {
            panic!("unexpected host request")
        };
        let result = serde_json::to_value(result.unwrap()).unwrap();
        assert_eq!(result["answer"], json!({"kind":"choice","ids":["b"]}));
    }
}
