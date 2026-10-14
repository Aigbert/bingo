//! Fact-only inputs for transcript sites.
use bingo_sdk::{Item, ItemBody, component::*};

pub fn props(item: &Item) -> Option<Props> {
    match &item.body {
        ItemBody::Reasoning { text, .. } => Some(Props::Reasoning(ReasoningProps {
            text: text.clone(),
            status: item.status,
        })),
        ItemBody::Compaction {
            summary,
            replaced,
            before,
            after,
            duration_ms,
            ..
        } => Some(Props::Compaction(CompactionProps {
            summary: summary.clone(),
            replaced: *replaced,
            before: *before,
            after: *after,
            duration_ms: *duration_ms,
        })),
        ItemBody::Rewind {
            to_turn, dropped, ..
        } => Some(Props::Rewind(RewindProps {
            to_turn: to_turn.clone(),
            dropped: *dropped,
        })),
        ItemBody::Interruption { marker, .. } => Some(Props::Interruption(InterruptionProps {
            marker: marker.clone(),
        })),
        _ => receipt_or_operation(item),
    }
}

fn receipt_or_operation(item: &Item) -> Option<Props> {
    match &item.body {
        ItemBody::QuestionAnswer {
            interaction,
            question,
            answer,
            ..
        } => Some(Props::QuestionAnswer(QuestionAnswerProps {
            interaction: interaction.clone(),
            question: question.clone(),
            answer: answer.clone(),
        })),
        ItemBody::PermissionReceipt {
            interaction,
            tool,
            decision,
            feedback,
            ..
        } => Some(Props::PermissionReceipt(PermissionReceiptProps {
            interaction: interaction.clone(),
            tool: tool.clone(),
            decision: *decision,
            feedback: feedback.clone(),
        })),
        ItemBody::Asset { asset, label, .. } => Some(Props::Asset(AssetProps {
            asset: asset.clone(),
            label: label.clone(),
        })),
        ItemBody::Shell {
            command,
            output,
            exit,
            cwd,
            ..
        } => Some(Props::ShellRun(ShellRunProps {
            command: command.clone(),
            output: output.clone(),
            exit: *exit,
            cwd: cwd.clone(),
            status: item.status,
        })),
        ItemBody::Action {
            name, args, result, ..
        } => Some(Props::Action(ActionProps {
            name: name.clone(),
            args: args.clone(),
            result: result.clone(),
            status: item.status,
        })),
        _ => None,
    }
}

pub fn change(item: &mut Item, props: &Props) {
    match props {
        Props::Reasoning(props) => {
            item.status = props.status;
            if let ItemBody::Reasoning { text, .. } = &mut item.body {
                *text = props.text.clone();
            }
        }
        Props::Compaction(props) => {
            item.body = ItemBody::Compaction {
                summary: props.summary.clone(),
                replaced: props.replaced,
                before: props.before,
                after: props.after,
                duration_ms: props.duration_ms,
            };
        }
        Props::Rewind(props) => {
            item.body = ItemBody::Rewind {
                to_turn: props.to_turn.clone(),
                dropped: props.dropped,
            };
        }
        Props::Interruption(props) => {
            item.body = ItemBody::Interruption {
                marker: props.marker.clone(),
            };
        }
        _ => change_receipt_or_operation(item, props),
    }
}

fn change_receipt_or_operation(item: &mut Item, props: &Props) {
    match props {
        Props::QuestionAnswer(props) => {
            item.body = ItemBody::QuestionAnswer {
                interaction: props.interaction.clone(),
                question: props.question.clone(),
                answer: props.answer.clone(),
            };
        }
        Props::PermissionReceipt(props) => {
            item.body = ItemBody::PermissionReceipt {
                interaction: props.interaction.clone(),
                tool: props.tool.clone(),
                decision: props.decision,
                feedback: props.feedback.clone(),
            };
        }
        Props::Asset(props) => {
            item.body = ItemBody::Asset {
                asset: props.asset.clone(),
                label: props.label.clone(),
            };
        }
        Props::ShellRun(props) => {
            item.status = props.status;
            item.body = ItemBody::Shell {
                command: props.command.clone(),
                output: props.output.clone(),
                exit: props.exit,
                cwd: props.cwd.clone(),
            };
        }
        Props::Action(props) => {
            item.status = props.status;
            item.body = ItemBody::Action {
                name: props.name.clone(),
                args: props.args.clone(),
                result: props.result.clone(),
            };
        }
        _ => {}
    }
}
