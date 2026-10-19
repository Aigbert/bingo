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
        let Some(press) = press(&args) else {
            return Ok(handled(false));
        };
        let props: component::QuestionProps =
            serde_json::from_value(args["props"].clone()).map_err(error)?;
        let (used, answer) = answer(&self.0, &session, id, &props, &press)?;
        cx.host
            .invalidate(
                &session,
                vec![args["request"].as_str().unwrap_or_default().into()],
            )
            .await?;
        Ok(answer.map_or_else(|| handled(used), |answer| CommandOutcome::Answer { answer }))
    }
}
/// A key pressed or a choice clicked; `ctrl+c` is the surface's own.
fn press(args: &Value) -> Option<sheet::Press> {
    if args["event"] != "press" {
        return None;
    }
    if let Some(id) = args["value"]["pick"].as_str() {
        return Some(sheet::Press::Pick(id.to_owned()));
    }
    let key = args["value"]["key"].as_str().unwrap_or_default();
    (key != "ctrl+c").then(|| sheet::Press::Key(key.to_owned()))
}

/// A press on the question; a form's answers go back together, a single
/// question's on its own.
fn answer(
    states: &States,
    session: &SessionId,
    id: &str,
    props: &component::QuestionProps,
    press: &sheet::Press,
) -> Result<(bool, Option<Answer>), KernelError> {
    let mut states = states.lock().map_err(error)?;
    let Some(state) = states.get_mut(&(session.clone(), id.into())) else {
        return Ok((false, None));
    };
    let (used, answers) = match press {
        sheet::Press::Key(key) => state.key(&props.questions, key),
        sheet::Press::Pick(id) => state.pick(&props.questions, id),
    };
    let answer = answers.and_then(|answers| match props.form {
        true => Some(Answer::Form { answers }),
        false => answers.into_iter().next(),
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
        let args = json!({"handler":"q","session":"s","event":"press","request":"q", "value":{"key":"enter"},
            "props":{"tool":"", "questions":[{"question":"Route?", "options":[{"id":"a","label":"Alpha"},{"id":"b","label":"Beta"}],"multi":false,"freeText":false}]}});
        let handler = Handler(states);
        let args = args.to_string();
        let cx = CommandContext {
            surface: None,
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
        assert_eq!(result["answer"], json!({"kind":"choice","ids":["a"]}));
    }
}
