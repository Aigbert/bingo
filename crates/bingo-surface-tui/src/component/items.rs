//! Item props are borrowed facts turned into one temporary render input.
use bingo_sdk::{Item, ItemBody, ItemStatus, ToolOutput, component::*};
use serde_json::Value;

pub fn props(item: &Item, expanded: bool) -> Option<Props> {
    match &item.body {
        ItemBody::User { parts, origin } => Some(Props::UserMessage(UserMessageProps {
            text: parts
                .iter()
                .filter_map(|part| part.as_text())
                .collect::<Vec<_>>()
                .join("\n"),
            origin: origin.clone(),
            is_expanded: expanded,
        })),
        ItemBody::Assistant { text } => Some(Props::AssistantMessage(AssistantMessageProps {
            text: text.clone(),
            is_first_of_reply: true,
        })),
        ItemBody::ToolCall {
            call_id,
            name,
            input,
            output,
            ..
        } => Some(Props::ToolUse(ToolUseProps {
            tool_use_id: call_id.to_string(),
            tool: name.clone(),
            input: input.clone(),
            is_running: !item.status.is_terminal(),
            is_errored: item.status.failed(output.as_ref()),
            is_interrupted: item.status == ItemStatus::Interrupted,
            output: output
                .as_ref()
                .and_then(|output| serde_json::to_value(output).ok()),
        })),
        ItemBody::Action {
            name,
            args,
            result: Some(result),
        } => Some(Props::CommandOutput(CommandOutputProps {
            command: name.clone(),
            args: text_of(args),
            text: text_of(result),
            is_errored: item.status == ItemStatus::Failed,
        })),
        ItemBody::Notice { text, .. } => Some(Props::InfoNotice(InfoNoticeProps {
            text: text.clone(),
            command: None,
        })),
        _ => super::facts::props(item),
    }
}
pub fn id(item: &Item) -> String {
    match &item.body {
        ItemBody::ToolCall { call_id, .. } => call_id.to_string(),
        _ => item.id.to_string(),
    }
}
pub fn changed(item: &Item, props: &Props) -> Item {
    let mut item = item.clone();
    match (&mut item.body, props) {
        (ItemBody::User { parts, origin }, Props::UserMessage(props)) => {
            *parts = vec![bingo_sdk::ContentPart::text(&props.text)];
            *origin = props.origin.clone();
        }
        (ItemBody::Assistant { text }, Props::AssistantMessage(props)) => {
            *text = props.text.clone()
        }
        (
            ItemBody::ToolCall {
                name,
                input,
                output,
                ..
            },
            Props::ToolUse(props),
        ) => {
            *name = props.tool.clone();
            *input = props.input.clone();
            *output = props.output.as_ref().map(output_of);
            item.status = status(props);
        }
        (
            ItemBody::Action {
                name,
                args,
                result: Some(result),
            },
            Props::CommandOutput(props),
        ) => {
            *name = props.command.clone();
            *args = Value::String(props.args.clone());
            *result = Value::String(props.text.clone());
            item.status = if props.is_errored {
                ItemStatus::Failed
            } else {
                ItemStatus::Completed
            };
        }
        (ItemBody::Notice { text, .. }, Props::InfoNotice(props)) => {
            *text = props.text.clone();
            if let Some(command) = &props.command {
                text.push_str(&format!(" /{command}"));
            }
        }
        _ => super::facts::change(&mut item, props),
    }
    item
}
fn status(props: &ToolUseProps) -> ItemStatus {
    if props.is_interrupted {
        ItemStatus::Interrupted
    } else if props.is_running {
        ItemStatus::Running
    } else if props.is_errored {
        ItemStatus::Failed
    } else {
        ItemStatus::Completed
    }
}
pub fn output_of(value: &Value) -> ToolOutput {
    serde_json::from_value(value.clone()).unwrap_or_else(|_| {
        ToolOutput::text(
            value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string()),
        )
    })
}

fn text_of(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string())
}

pub fn tail(item: &Item) -> Option<Props> {
    let ItemBody::ToolCall {
        call_id,
        name,
        progress,
        output,
        ..
    } = &item.body
    else {
        return None;
    };
    if item.status == ItemStatus::Running {
        return progress.as_ref().map(|hint| {
            Props::ToolProgress(ToolProgressProps {
                tool_use_id: call_id.to_string(),
                kind: "text".into(),
                hint: hint.clone(),
            })
        });
    }
    output.as_ref().map(|output| {
        Props::ToolResult(ToolResultProps {
            tool_use_id: call_id.to_string(),
            tool: name.clone(),
            output: serde_json::to_value(output).unwrap_or_default(),
            is_errored: item.status.failed(Some(output)),
        })
    })
}

pub fn all(item: &Item, expanded: bool) -> Vec<Props> {
    group(item, expanded)
        .into_iter()
        .chain(props(item, expanded))
        .chain(tail(item))
        .collect()
}

pub fn group(item: &Item, expanded: bool) -> Option<Props> {
    let Props::ToolUse(call) = props(item, expanded)? else {
        return None;
    };
    Some(Props::ToolGroup(ToolGroupProps {
        is_active: call.is_running,
        is_expanded: expanded,
        calls: vec![call],
    }))
}
