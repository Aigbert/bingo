//! An open interaction never delegates its KeyBar to a mod.
use crate::ui::Ui;
use bingo_sdk::{
    Interaction, InteractionKind,
    component::{KeyPair as Pair, QuestionFocus},
};

/// `away` is a teammate's session on screen, where esc only goes home.
pub fn pairs(
    ui: &Ui,
    ask: &Interaction,
    selection: Option<&QuestionFocus>,
    away: bool,
) -> Vec<Pair> {
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
    let stop = if away { "back" } else { "stop" };
    if ui.dialog.words.is_some() || selection.is_some_and(|q| q.writing) {
        return vec![Pair::new("⏎", "send"), Pair::new("esc", "back")];
    }
    let mut pairs = Vec::new();
    match &ask.kind {
        InteractionKind::Form { questions, .. } => {
            let at = selection.map_or(ui.dialog.form.tab, |q| q.field);
            if questions.len() > 1 {
                pairs.push(Pair::new("←→", "questions"));
            }
            if questions.get(at).is_some_and(|q| q.multi) {
                pairs.push(Pair::new("space", "tick"));
            }
            pairs.push(Pair::new("⏎", "choose"));
        }
        InteractionKind::Question(question) => {
            if question.multi {
                pairs.push(Pair::new("space", "tick"));
            }
            pairs.push(Pair::new("⏎", "choose"));
        }
        _ => pairs.extend([Pair::new("↑↓", "choose"), Pair::new("⏎", "confirm")]),
    }
    pairs.push(Pair::new("esc", stop));
    pairs
}
