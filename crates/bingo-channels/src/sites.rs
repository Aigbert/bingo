//! The places this chat actually draws; unmatched sites cost no host request.
use crate::deliver::Op;
use bingo_sdk::{Event, Frame, ItemBody, SessionState, component::*, render::*};

pub fn request(
    frame: &Frame,
    state: &SessionState,
    op: &Op,
    input: Option<&bingo_sdk::Input>,
) -> Option<RenderRequest> {
    let (id, props) = props(frame, state, op, input)?;
    let request = RenderRequest {
        component: props.component(),
        request_id: id,
        surface: SurfaceName::Chat,
        props: props.value(),
        viewport: Viewport {
            columns: 0,
            rows: 0,
            place: None,
        },
    };
    matched(&state.config)
        .asks(request.component, &request)
        .then_some(request)
}

fn props(
    frame: &Frame,
    state: &SessionState,
    op: &Op,
    input: Option<&bingo_sdk::Input>,
) -> Option<(String, Props)> {
    match (&frame.event, op) {
        (Event::TurnCompleted { turn, .. }, Op::Finalize { question: None, .. }) => Some((
            format!("answer:{turn}"),
            Props::AssistantMessage(AssistantMessageProps {
                text: state
                    .transcript
                    .items()
                    .iter()
                    .filter(|item| item.turn.as_ref() == Some(turn))
                    .filter_map(|item| match &item.body {
                        ItemBody::Assistant { text } if !text.trim().is_empty() => {
                            Some(text.as_str())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n"),
                is_first_of_reply: true,
            }),
        )),
        (Event::ItemCompleted { item }, Op::Status { .. }) => shell(item),
        (Event::IntentAck { intent, outcome }, Op::Card { text }) => {
            let (command, args) = input.map(command).unwrap_or_default();
            Some((
                format!("command:{intent}"),
                Props::CommandOutput(CommandOutputProps {
                    command,
                    args,
                    text: text.clone(),
                    is_errored: matches!(outcome, bingo_sdk::IntentOutcome::Rejected { .. }),
                }),
            ))
        }
        (_, Op::Status { text }) => Some((
            format!("notice:{}", frame.seq),
            Props::InfoNotice(InfoNoticeProps {
                text: text.clone(),
                command: None,
            }),
        )),
        _ => None,
    }
}

fn shell(item: &bingo_sdk::Item) -> Option<(String, Props)> {
    let ItemBody::Shell {
        command,
        output,
        exit,
        cwd,
        ..
    } = &item.body
    else {
        return None;
    };
    Some((
        item.id.to_string(),
        Props::ShellRun(ShellRunProps {
            command: command.clone(),
            output: output.clone(),
            exit: *exit,
            cwd: cwd.clone(),
            status: item.status,
        }),
    ))
}

fn command(input: &bingo_sdk::Input) -> (String, String) {
    match input {
        bingo_sdk::Input::Action { action } => (
            action.name.clone(),
            action
                .args
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| action.args.to_string()),
        ),
        bingo_sdk::Input::Text { text, .. } => {
            let text = text.trim().trim_start_matches('/');
            let (name, args) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
            (name.into(), args.trim_start().into())
        }
    }
}

pub fn action_notice(
    frame: &Frame,
    state: &SessionState,
    input: Option<&bingo_sdk::Input>,
) -> Option<Op> {
    if !matches!(input, Some(bingo_sdk::Input::Action { .. })) {
        return None;
    }
    let Event::IntentAck {
        outcome: bingo_sdk::IntentOutcome::Applied { result },
        ..
    } = &frame.event
    else {
        return None;
    };
    let id = bingo_sdk::ItemId::from_raw(result["item"].as_str()?);
    let ItemBody::Notice { text, .. } = &state.item(&id)?.body else {
        return None;
    };
    Some(Op::Status { text: text.clone() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use bingo_sdk::{Input, IntentId, IntentOutcome, Level, Origin};
    use serde_json::json;
    #[test]
    fn each_site_carries_its_typed_facts_and_chat_prefilters_apply() {
        let mut state = fixtures::state();
        state.config.kernel["render"] =
            json!({"CommandOutput":[{"surface":"chat"}],"InfoNotice":[{}],"ShellRun":[{}]});
        let input = Input::text("/model fake", Origin::surface("channels"));
        let frame = fixtures::frame(
            1,
            Event::IntentAck {
                intent: IntentId::from_raw("i"),
                outcome: IntentOutcome::Applied {
                    result: json!({"message":"Selected"}),
                },
            },
        );
        let req = request(
            &frame,
            &state,
            &Op::Card {
                text: "Selected".into(),
            },
            Some(&input),
        )
        .unwrap();
        assert_eq!(
            req.props,
            json!({"command":"model","args":"fake","text":"Selected","isErrored":false})
        );
        let notice = fixtures::frame(
            2,
            Event::Notice {
                level: Level::Warn,
                code: "test".into(),
                text: "Notice".into(),
            },
        );
        let req = request(
            &notice,
            &state,
            &Op::Status {
                text: "Notice".into(),
            },
            None,
        )
        .unwrap();
        assert_eq!(req.component, Component::InfoNotice);
        assert!(Props::parse(req.component, req.props).is_ok());
        state.config.kernel["render"] = json!({"CommandOutput":[{"surface":"terminal"}]});
        assert!(
            request(
                &frame,
                &state,
                &Op::Card {
                    text: "Selected".into()
                },
                Some(&input)
            )
            .is_none()
        );
    }
}
