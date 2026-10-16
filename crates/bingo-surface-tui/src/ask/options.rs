use bingo_sdk::{Answer, AnswerSpec, Interaction, InteractionKind, LoginFlow, Question, Rung};

/// What one row of the dialog does when it is chosen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Choice {
    /// A complete answer, sent as it is.
    Send(Answer),
    /// Opens the row where the person says what to do instead.
    Words,
    Change,
    /// One member of a multiple-choice set.
    Toggle(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Opt {
    pub label: String,
    pub description: Option<String>,
    pub choice: Choice,
}

/// The rows this interaction offers, in the order they are shown: the rungs
/// the kernel will accept ([`Interaction::rungs`]), each in this card's own
/// words, and the row that takes words of one's own where it takes any.
pub fn options(interaction: &Interaction) -> Vec<Opt> {
    let rungs = interaction
        .rungs()
        .into_iter()
        .map(|rung| row(&interaction.kind, rung));
    match &interaction.kind {
        // A paste flow's credential goes above the way out; a browser or
        // device flow finishes on its own and asks for nothing.
        InteractionKind::Login { flow, .. } => {
            paste(interaction, flow).into_iter().chain(rungs).collect()
        }
        // The form has a card of its own (M53); the dialog answers no key of
        // it, so `esc` is the only way out of one drawn here.
        InteractionKind::Form { .. } => Vec::new(),
        // Words of one's own are the last row, under the options they stand
        // as an alternative to.
        _ => rungs
            .chain(other(interaction))
            .chain(change(interaction))
            .collect(),
    }
}

/// One rung as this card draws it. The kernel says which answers there are;
/// the words they are put in, and the gesture each row takes, are the
/// surface's own.
fn row(kind: &InteractionKind, rung: Rung) -> Opt {
    Opt {
        label: label(&rung),
        choice: gesture(kind, &rung.answer),
        description: rung.description,
    }
}

/// What this card calls a rung: a permission is a plain yes or no in the
/// second person, and a question wears the words the asker wrote.
fn label(rung: &Rung) -> String {
    match &rung.answer {
        Answer::AllowOnce => "Yes".into(),
        Answer::AllowSession { .. } => "Yes for this session".into(),
        Answer::Deny { .. } => "No, and tell it why".into(),
        Answer::Confirm => "Confirm".into(),
        Answer::Cancel => "Cancel".into(),
        _ => rung.label.clone(),
    }
}

/// What choosing a row does: a refusal wants a reason, so it opens the words
/// row rather than sending as it stands, and a member of a multiple-choice
/// set is ticked rather than sent.
fn gesture(kind: &InteractionKind, answer: &Answer) -> Choice {
    match answer {
        Answer::Deny { .. } => Choice::Words,
        Answer::Choice { ids, .. } if ticks(kind) => match ids.first() {
            Some(id) => Choice::Toggle(id.clone()),
            None => Choice::Send(answer.clone()),
        },
        _ => Choice::Send(answer.clone()),
    }
}

/// Whether this question is answered by ticking a set rather than by picking
/// one of a list.
fn ticks(kind: &InteractionKind) -> bool {
    matches!(
        kind,
        InteractionKind::Question(Question { multi: true, .. })
    )
}

/// The row for words of one's own, where the interaction takes them.
fn other(interaction: &Interaction) -> Option<Opt> {
    interaction
        .answers
        .contains(&AnswerSpec::Text)
        .then(|| plain("something else…", Choice::Words))
}

/// The row a paste flow's credential goes in.
fn paste(interaction: &Interaction, flow: &LoginFlow) -> Option<Opt> {
    (matches!(flow, LoginFlow::Paste) && interaction.answers.contains(&AnswerSpec::Text))
        .then(|| plain("Paste it here", Choice::Words))
}

fn plain(label: &str, choice: Choice) -> Opt {
    Opt {
        label: label.to_string(),
        description: None,
        choice,
    }
}

pub fn initial(interaction: &Interaction) -> usize {
    if matches!(
        interaction.kind,
        InteractionKind::Permission {
            caution: Some(_),
            ..
        }
    ) {
        options(interaction)
            .iter()
            .position(|opt| opt.choice == Choice::Words)
            .unwrap_or(0)
    } else {
        0
    }
}
fn change(interaction: &Interaction) -> Option<Opt> {
    interaction
        .answers
        .contains(&AnswerSpec::AllowChanged)
        .then(|| plain("Change it first", Choice::Change))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn permission_rows_follow_accepted_specs_and_danger_starts_on_no() {
        let mut open = crate::test_support::permission(Some("Bash(cargo:*)"), None);
        open.answers.push(AnswerSpec::AllowChanged);
        assert_eq!(
            options(&open)
                .iter()
                .map(|opt| opt.label.as_str())
                .collect::<Vec<_>>(),
            [
                "Yes",
                "Yes for this session",
                "No, and tell it why",
                "Change it first"
            ]
        );
        assert_eq!(initial(&open), 0);
        if let InteractionKind::Permission {
            caution,
            session_scope,
            ..
        } = &mut open.kind
        {
            *caution = Some("destructive".into());
            *session_scope = None;
        }
        assert_eq!(initial(&open), 1);
        assert_eq!(options(&open).len(), 3);
        open.answers = vec![AnswerSpec::Deny, AnswerSpec::AllowChanged];
        assert_eq!(initial(&open), 0);
        assert_eq!(options(&open).len(), 2);
    }
}
