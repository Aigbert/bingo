//! Each step retains its row throughout the turn (ADR-0097).
use crate::ask::receipts::Answers;
use crate::fold::Fold;
use crate::paths;
use crate::tree::Agents;
use bingo_sdk::component::{LedgerProps, Props, Step, StepState, ledger::Marks};
use bingo_sdk::{DecisionKind, Item, ItemBody, ItemId, ItemStatus, SessionState, Upshot};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Role {
    #[default]
    Own,
    Hidden,
    Step {
        step: Step,
        opened: bool,
    },
    Narration,
}

#[derive(Debug, Default)]
pub struct Roles(BTreeMap<ItemId, Role>);

impl Roles {
    pub fn of(agents: &Agents<'_>, rows: &crate::transcript::Rows<'_>) -> Self {
        let state = rows.state;
        let replaced = rowed(&rows.slots);
        let mut roles = BTreeMap::new();
        for item in state.transcript.items() {
            if item.turn.is_none() && !matches!(item.body, ItemBody::ToolCall { .. }) {
                continue;
            }
            let role = match &item.body {
                ItemBody::ToolCall { quiet, output, .. }
                    if (!*quiet || item.status.failed(output.as_ref()))
                        && !agents.contains_key(&item.id)
                        && !replaced.contains(&item.id) =>
                {
                    Role::Step {
                        step: step(item, rows),
                        opened: rows.folds.get(&item.id) == Some(&Fold::Open),
                    }
                }
                ItemBody::Interruption { .. } | ItemBody::Reasoning { .. } => Role::Hidden,
                ItemBody::Assistant { .. } if rows.followed.contains(&item.id) => Role::Narration,
                _ => continue,
            };
            roles.insert(item.id.clone(), role);
        }
        Self(roles)
    }

    pub fn of_item(&self, id: &ItemId) -> &Role {
        self.0.get(id).unwrap_or(&Role::Own)
    }
}

/// Items before the last call are followed by a step, including narration.
pub fn followed<'a>(items: &[&'a Item]) -> impl Iterator<Item = &'a ItemId> {
    items
        .iter()
        .rev()
        .skip_while(|item| !matches!(item.body, ItemBody::ToolCall { .. }))
        .skip(1)
        .map(|item| &item.id)
}

pub fn requests(
    item: &Item,
    role: &Role,
    rows: &crate::transcript::Rows<'_>,
) -> Vec<(String, Props)> {
    match role {
        Role::Step { step, opened } => std::iter::once(ledger(item, step, &rows.answers, rows.now))
            .chain(
                crate::component::items::tail(item)
                    .filter(|props| matches!(props, Props::ToolProgress(_)))
                    .map(|props| (crate::component::items::id(item), props)),
            )
            .chain((*opened).then(|| result(item)).flatten())
            .collect(),
        _ => Vec::new(),
    }
}

pub fn result(item: &Item) -> Option<(String, Props)> {
    match crate::component::items::tail(item)? {
        props @ Props::ToolResult(_) => Some((crate::component::items::id(item), props)),
        _ => None,
    }
}

pub fn ledger(
    item: &Item,
    step: &Step,
    answers: &Answers<'_>,
    now: crate::clock::Now,
) -> (String, Props) {
    let mut step = step.clone();
    if step.state == StepState::Running {
        step.elapsed_ms = Some(running_for(item, answers, now).as_millis() as u64 / 1000 * 1000);
    }
    let props = Props::Ledger(LedgerProps {
        steps: vec![step],
        marks: marks(),
    });
    (crate::component::items::id(item), props)
}

/// M202 will move this start to the call's Running frame.
pub fn running_for(
    item: &Item,
    answers: &Answers<'_>,
    now: crate::clock::Now,
) -> jiff::SignedDuration {
    let start = answers
        .to(&item.id)
        .iter()
        .filter(|receipt| matches!(receipt.body, ItemBody::PermissionReceipt { .. }))
        .map(|receipt| receipt.started_at)
        .max()
        .unwrap_or(item.started_at);
    now.wall
        .duration_since(start)
        .max(jiff::SignedDuration::ZERO)
}

pub fn marks() -> Marks {
    Marks {
        asked: crate::theme::glyphs().chrome.steps[1].into(),
    }
}

fn rowed(slots: &[bingo_sdk::slot::Placed]) -> Vec<ItemId> {
    slots
        .iter()
        .filter_map(|placed| match &placed.slot {
            bingo_sdk::slot::Slot::Row {
                item,
                mark,
                title,
                badge,
                caption,
                body,
                ..
            } => {
                let drawn = mark.is_some()
                    || title.is_some()
                    || badge.is_some()
                    || caption.is_some()
                    || body.is_some();
                drawn.then(|| item.clone())
            }
            _ => None,
        })
        .collect()
}

/// Whether the person said no to a step.
fn refused(answers: &[&Item]) -> bool {
    answers.iter().any(|receipt| {
        matches!(
            receipt.body,
            ItemBody::PermissionReceipt {
                decision: DecisionKind::Deny,
                ..
            }
        )
    })
}

/// One call, in the words its row shows.
pub fn step(item: &Item, rows: &crate::transcript::Rows<'_>) -> Step {
    let state = rows.state;
    let answers = &rows.answers;
    let tools = &rows.tools;
    let ItemBody::ToolCall {
        name,
        input,
        output,
        ..
    } = &item.body
    else {
        return crate::tool_rows::resolve(
            "",
            &serde_json::Value::Null,
            tools,
            crate::theme::bullet().is_ascii(),
        );
    };
    let mut step = crate::tool_rows::resolve(name, input, tools, crate::theme::bullet().is_ascii());
    step.target = paths::shorten_in(
        step.target.lines().next().unwrap_or_default(),
        &state.summary.cwd,
        paths::home(),
    );
    step.state = if refused(answers.to(&item.id)) {
        StepState::Skipped
    } else {
        step_state(item, state)
    };
    step.landed_at = item.completed_at.map(|time| time.as_millisecond());
    step.upshot = output.as_ref().and_then(bingo_sdk::ToolOutput::outcome);
    step.answer = answer(answers.to(&item.id), input);
    step.why = why(item, &rows.mode_labels);
    step
}

fn why(item: &Item, modes: &BTreeMap<String, String>) -> Option<String> {
    let ItemBody::ToolCall { allowed_by, .. } = &item.body else {
        return None;
    };
    match allowed_by.as_deref()? {
        bingo_sdk::Reason::Mode { mode } => Some(modes.get(mode).unwrap_or(mode).clone()),
        bingo_sdk::Reason::Rule {
            source: Some(bingo_sdk::policy::RuleSource::Session),
            ..
        } => Some("session rule".into()),
        _ => None,
    }
}

fn step_state(item: &Item, state: &SessionState) -> StepState {
    let ItemBody::ToolCall { output, .. } = &item.body else {
        return StepState::Done;
    };
    let asked = || {
        state
            .interactions
            .iter()
            .any(|open| open.item.as_ref() == Some(&item.id))
    };
    match item.status {
        ItemStatus::Pending | ItemStatus::Running if asked() => StepState::Asked,
        ItemStatus::Pending | ItemStatus::Running => StepState::Running,
        ItemStatus::Interrupted => StepState::Stopped,
        status if status.failed(output.as_ref()) => StepState::Failed,
        _ => StepState::Done,
    }
}

fn answer(receipts: &[&Item], input: &serde_json::Value) -> Option<Upshot> {
    let words: Vec<_> = receipts
        .iter()
        .filter_map(|item| match &item.body {
            ItemBody::PermissionReceipt {
                decision: DecisionKind::AllowChanged | DecisionKind::AllowChangedSession,
                ..
            } => Some("changed by you".to_owned()),
            ItemBody::QuestionAnswer { answer, .. } => Some(question_result(answer, input)),
            _ => None,
        })
        .collect();
    (!words.is_empty()).then(|| Upshot::plain(words.join(crate::theme::glyphs::get("sep"))))
}

#[cfg(test)]
mod tests;

// M197 replaces this receipt-string contract with the typed Answer.
fn question_result(answer: &str, input: &serde_json::Value) -> String {
    let questions = input.get("questions").and_then(serde_json::Value::as_array);
    if let Some(questions) = questions {
        if questions.len() > 1 && !answer.starts_with("skipped") {
            return format!("{} answers", questions.len());
        }
        if let Some(question) = questions.first()
            && let Some(label) = question
                .get("header")
                .or_else(|| question.get("question"))
                .and_then(serde_json::Value::as_str)
            && let Some(words) = answer.strip_prefix(&format!("{label}: "))
        {
            return words.to_owned();
        }
    }
    answer.to_owned()
}
