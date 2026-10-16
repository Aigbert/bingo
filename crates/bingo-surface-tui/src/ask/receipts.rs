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
