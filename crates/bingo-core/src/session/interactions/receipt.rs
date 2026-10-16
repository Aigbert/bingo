use bingo_sdk::*;

pub(super) fn answered(open: &Interaction, answer: &Answer) -> Option<ItemBody> {
    // Escape can interrupt the waiting caller before it records its result.
    if let (InteractionKind::Permission { tool, .. }, Answer::Deny { feedback }) =
        (&open.kind, answer)
    {
        return Some(ItemBody::PermissionReceipt {
            item: open.item.clone(),
            interaction: open.id.clone(),
            tool: tool.clone(),
            decision: DecisionKind::Deny,
            feedback: feedback.clone(),
        });
    }
    let (question, answer) = match (&open.kind, answer) {
        (InteractionKind::Question(question), answer) => {
            (question.question.clone(), words(question, answer))
        }
        (InteractionKind::Form { title, questions }, Answer::Form { answers }) => (
            title.clone().unwrap_or_else(|| "Form".into()),
            questions
                .iter()
                .zip(answers)
                .map(|(q, a)| {
                    format!(
                        "{}: {}",
                        q.header.as_deref().unwrap_or(&q.question),
                        words(q, a)
                    )
                })
                .collect::<Vec<_>>()
                .join("; "),
        ),
        (InteractionKind::Form { title, .. }, Answer::Cancel) => (
            title.clone().unwrap_or_else(|| "Form".into()),
            "skipped · it decides".into(),
        ),
        (InteractionKind::Form { title, .. }, _) => (
            title.clone().unwrap_or_else(|| "Form".into()),
            "declined".into(),
        ),
        _ => return None,
    };
    Some(ItemBody::QuestionAnswer {
        item: open.item.clone(),
        interaction: open.id.clone(),
        question,
        answer,
    })
}
fn words(question: &Question, answer: &Answer) -> String {
    match answer {
        Answer::Text { text } => text.clone(),
        Answer::Choice { ids, other } => ids
            .iter()
            .filter_map(|id| {
                question
                    .options
                    .iter()
                    .find(|o| &o.id == id)
                    .map(|o| o.label.clone())
            })
            .chain(other.iter().cloned())
            .collect::<Vec<_>>()
            .join(", "),
        Answer::Cancel => "skipped · it decides".into(),
        _ => "skipped".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_questions_and_forms_record_the_same_skip() {
        for kind in [
            serde_json::json!({"kind":"question","question":"Choose?","options":[],"freeText":true,"multi":false}),
            serde_json::json!({"kind":"form","questions":[],"title":"Options"}),
        ] {
            let open: Interaction = serde_json::from_value(serde_json::json!({
                "id":"q","session":"s","openedAt":"2026-10-16T00:00:00Z",
                "kind":kind,"answers":["cancel"]
            }))
            .unwrap();
            let Some(ItemBody::QuestionAnswer { answer, .. }) = answered(&open, &Answer::Cancel)
            else {
                panic!("missing answer receipt")
            };
            assert_eq!(answer, "skipped · it decides");
        }
    }
}
