use bingo_sdk::{Interaction, InteractionKind, SessionState, component::*};
use ratatui::text::Line;

pub fn props(interaction: &Interaction) -> Option<Props> {
    let questions = match &interaction.kind {
        InteractionKind::Question(question) => vec![question.clone()],
        InteractionKind::Form { questions, .. } => questions.clone(),
        _ => return None,
    };
    Some(Props::Question(QuestionProps {
        tool: String::new(),
        questions,
        metadata_source: None,
    }))
}

pub fn rows(
    state: &SessionState,
    ui: &crate::ui::Ui,
    interaction: &Interaction,
    agent: Option<&str>,
    width: usize,
    room: usize,
    pulls: &mut Vec<crate::pulled::Key>,
) -> Vec<(Line<'static>, Option<usize>)> {
    let own = |interaction: &Interaction, width| {
        crate::dialog::rows(
            &ui.dialog,
            interaction,
            agent,
            &state.summary.cwd,
            width,
            room,
        )
    };
    let original = own(interaction, width);
    let Some(props) = props(interaction) else {
        return original;
    };
    let Some(key) = super::key(
        state,
        interaction.id.as_str(),
        &props,
        width as u16,
        room as u16,
    ) else {
        return original;
    };
    pulls.push(key.clone());
    let Some(answer) = ui.placed.pulled.get(&key) else {
        return original;
    };
    let lines = super::draw(&props, Some(answer), width, |changed, width| {
        let mut changed_interaction = interaction.clone();
        if let Props::Question(props) = changed {
            match &mut changed_interaction.kind {
                InteractionKind::Question(question) => {
                    if let Some(first) = props.questions.first() {
                        *question = first.clone();
                    }
                }
                InteractionKind::Form { questions, .. } => *questions = props.questions.clone(),
                _ => {}
            }
        }
        own(&changed_interaction, width)
            .into_iter()
            .map(|(line, _)| line)
            .collect()
    });
    // Replacement trees have no engine option geometry; keyboard answers remain the interaction's.
    lines.into_iter().map(|line| (line, None)).collect()
}
