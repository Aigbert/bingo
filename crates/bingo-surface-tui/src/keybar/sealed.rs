//! An open interaction never delegates its KeyBar to a mod.
use crate::ui::Ui;
use bingo_sdk::{
    Interaction, InteractionKind, Preview,
    component::{KeyPair as Pair, QuestionFocus},
};

pub fn pairs(ui: &Ui, ask: &Interaction, selection: Option<&QuestionFocus>) -> Vec<Pair> {
    if ui.dialog.aside {
        return vec![
            Pair::new("esc", "back to the ask"),
            Pair::new("⏎", "queue this message"),
        ];
    }
    if let InteractionKind::Login { flow, .. } = &ask.kind {
        let mut pairs = Vec::new();
        if matches!(flow, bingo_sdk::LoginFlow::Paste) {
            pairs.push(Pair::new("⏎", "sign in"));
        }
        pairs.push(Pair::new("esc", "cancel"));
        return pairs;
    }
    if ui.dialog.changing.is_some() {
        return editing("make this edit");
    }
    if selection.is_some_and(|q| q.writing) {
        return vec![
            Pair::new("⏎", "answer"),
            Pair::new("esc", "skip, it decides"),
        ];
    }
    if ui.dialog.words.is_some() {
        return editing("send the reason");
    }
    match &ask.kind {
        InteractionKind::Form { questions, .. } => form(ui, questions, selection),
        InteractionKind::Question(question) => question_pairs(ui, question, selection),
        _ => permission(ui, ask),
    }
}

fn editing(action: &str) -> Vec<Pair> {
    vec![
        Pair::new("⏎", action),
        Pair::new("ctrl+g", "back"),
        Pair::new("esc", "no, stop the turn"),
    ]
}
fn form(
    ui: &Ui,
    questions: &[bingo_sdk::Question],
    selection: Option<&QuestionFocus>,
) -> Vec<Pair> {
    let toggle = questions
        .get(selection.map_or(ui.dialog.form.tab, |q| q.field))
        .is_some_and(|q| q.multi && q.options.len() == 1);
    vec![
        Pair::new("tab", "next"),
        if toggle {
            Pair::new("space", "toggle")
        } else {
            Pair::new("←→", "choose")
        },
        Pair::new("⏎", "send"),
        Pair::new("esc", "decline"),
    ]
}
fn question_pairs(
    ui: &Ui,
    question: &bingo_sdk::Question,
    selection: Option<&QuestionFocus>,
) -> Vec<Pair> {
    let mut pairs = vec![Pair::new("↑↓", "pick")];
    if question.multi {
        pairs.push(Pair::new("space", "tick"));
    }
    let answer = if question.multi {
        format!(
            "answer with {}",
            selection.map_or(ui.dialog.chosen.len(), |q| q.checked)
        )
    } else {
        "answer".into()
    };
    pairs.extend([Pair::new("⏎", answer), Pair::new("esc", "skip, it decides")]);
    pairs
}
fn permission(ui: &Ui, ask: &Interaction) -> Vec<Pair> {
    let mut pairs = vec![Pair::new("↑↓", "pick"), Pair::new("⏎", "answer")];
    if let InteractionKind::Permission { preview, .. } = &ask.kind
        && matches!(preview.as_deref(), Some(Preview::Diff { .. }))
    {
        pairs.push(Pair::new(
            "ctrl+o",
            if ui.dialog.expanded {
                "less"
            } else {
                "whole diff"
            },
        ));
    }
    pairs.push(Pair::new("esc", "no, stop the turn"));
    pairs
}
