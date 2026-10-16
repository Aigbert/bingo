use crate::{theme, transcript::Rows};
use bingo_sdk::{DecisionKind, Item, ItemBody, ItemId, SessionState};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

pub fn source(item: &Item) -> Option<&ItemId> {
    match &item.body {
        ItemBody::PermissionReceipt { item, .. } | ItemBody::QuestionAnswer { item, .. } => {
            item.as_ref()
        }
        _ => None,
    }
}
pub fn linked<'a>(state: &'a SessionState, id: &'a ItemId) -> impl Iterator<Item = &'a Item> {
    state
        .transcript
        .items()
        .iter()
        .map(AsRef::as_ref)
        .filter(move |item| source(item) == Some(id))
}
pub fn label(body: &ItemBody) -> Option<String> {
    match body {
        ItemBody::PermissionReceipt {
            decision, feedback, ..
        } => Some(match decision {
            DecisionKind::Allow => "you: yes".into(),
            DecisionKind::AllowSession => "you: yes for this session".into(),
            DecisionKind::AllowChangedSession => {
                "you changed it first; yes for this session".into()
            }
            DecisionKind::AllowChanged => "you changed it first".into(),
            DecisionKind::Deny => feedback
                .as_ref()
                .map(|why| format!("you: no, {why}"))
                .unwrap_or_else(|| "you: no".into()),
        }),
        ItemBody::QuestionAnswer { answer, .. } => Some(format!("you: {answer}")),
        _ => None,
    }
}
pub fn append(item: &Item, lines: &mut [Line<'static>], rows: &Rows<'_>) {
    let Some(first) = lines.first_mut() else {
        return;
    };
    let words = linked(rows.state, &item.id)
        .flat_map(|receipt| {
            let own = label(&receipt.body).unwrap_or_default();
            let Some(props) = crate::component::items::props(receipt, false) else {
                return vec![Line::from(own)];
            };
            crate::component::resolve(
                rows.state,
                rows.pulled,
                &crate::component::items::id(receipt),
                &props,
                rows.width,
                |_, _| vec![Line::from(own.clone())],
            )
        })
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join(" · ");
    if words.is_empty() {
        return;
    }
    let gap = rows
        .width
        .saturating_sub(first.width() + words.width())
        .max(2);
    first.spans.push(Span::raw(" ".repeat(gap)));
    first.spans.push(Span::styled(words, theme::dim()));
}
pub fn folded(state: &SessionState) -> Option<String> {
    let turn = state.last_turn.as_ref()?;
    let items: Vec<_> = state
        .transcript
        .items()
        .iter()
        .map(AsRef::as_ref)
        .filter(|item| item.turn.as_ref() == Some(&turn.id))
        .collect();
    let receipts: Vec<_> = items.iter().filter_map(|item| label(&item.body)).collect();
    let stopped = matches!(turn.status, bingo_sdk::TurnStatus::Interrupted { .. });
    if receipts.is_empty() && !stopped {
        return None;
    }
    let steps: Vec<_> = items
        .iter()
        .filter(|item| matches!(item.body, ItemBody::ToolCall { .. }))
        .collect();
    let ran = ran(state, &steps);
    let noun = if steps.len() == 1 { "step" } else { "steps" };
    let ending = if stopped {
        format!(" · stopped · {ran} of {} {noun} had run", steps.len())
    } else {
        String::new()
    };
    let answers = if receipts.is_empty() {
        String::new()
    } else {
        format!(" · {}", receipts.join(" · "))
    };
    Some(format!(
        "{} {} {noun}{ending}{answers}",
        theme::glyphs().point,
        steps.len()
    ))
}

fn ran(state: &SessionState, steps: &[&&Item]) -> usize {
    steps
        .iter()
        .filter(|item| {
            !linked(state, &item.id).any(|receipt| {
                matches!(
                    receipt.body,
                    ItemBody::PermissionReceipt {
                        decision: DecisionKind::Deny,
                        ..
                    }
                )
            }) && matches!(
                item.status,
                bingo_sdk::ItemStatus::Completed | bingo_sdk::ItemStatus::Failed
            )
        })
        .count()
}
