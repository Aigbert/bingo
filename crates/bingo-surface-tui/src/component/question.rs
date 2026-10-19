use bingo_sdk::{Interaction, InteractionKind, component::*};

pub fn props(interaction: &Interaction) -> Option<Props> {
    let questions = match &interaction.kind {
        InteractionKind::Question(question) => vec![question.clone()],
        InteractionKind::Form { questions, .. } => questions.clone(),
        _ => return None,
    };
    Some(Props::Question(QuestionProps {
        interaction_id: Some(interaction.id.clone()),
        form: matches!(interaction.kind, InteractionKind::Form { .. }),
        tool: String::new(),
        questions,
        metadata_source: match &interaction.kind {
            InteractionKind::Form { title, .. } => title.clone(),
            _ => None,
        },
    }))
}

type Rows = Vec<(ratatui::text::Line<'static>, Option<usize>)>;

pub fn rows(
    site: &super::sealed::Site<'_>,
    pulls: &mut Vec<crate::pulled::Key>,
) -> super::sealed::Drawing {
    let original = own(site, site.interaction, site.width);
    let Some(props) = props(site.interaction) else {
        return super::sealed::Drawing::plain(original);
    };
    let Some(key) = super::key(
        site.state,
        site.interaction.id.as_str(),
        &props,
        site.width as u16,
        site.room.saturating_sub(1) as u16,
    ) else {
        return super::sealed::Drawing::plain(original);
    };
    pulls.push(key.clone());
    let Some(answer) = site.ui.placed.pulled.display(&key) else {
        return super::sealed::Drawing::plain(original);
    };
    let options = std::cell::RefCell::new(Vec::new());
    let drawing = super::draw_marked(
        &props,
        Some(answer),
        site.width,
        &crate::views::Marks::at(site.now),
        |props, width| {
            let interaction = rewritten(site.interaction, props);
            let rows = own(site, &interaction, width);
            options
                .borrow_mut()
                .extend(rows.iter().filter_map(|(line, option)| {
                    option.map(|option| (line.to_string().trim().to_owned(), option))
                }));
            rows.into_iter().map(|(line, _)| line).collect::<Vec<_>>()
        },
    );
    mark_options(drawing, &options.borrow())
}

fn own(site: &super::sealed::Site<'_>, interaction: &Interaction, width: usize) -> Rows {
    crate::dialog::rows(
        &site.ui.dialog,
        interaction,
        site.agent,
        width,
        site.room.saturating_sub(1),
    )
}

fn rewritten(interaction: &Interaction, props: &Props) -> Interaction {
    let mut interaction = interaction.clone();
    if let Props::Question(props) = props {
        match &mut interaction.kind {
            InteractionKind::Question(question) => {
                if let Some(first) = props.questions.first() {
                    *question = first.clone();
                }
            }
            InteractionKind::Form { questions, .. } => *questions = props.questions.clone(),
            _ => {}
        }
    }
    interaction
}

fn mark_options(
    drawing: crate::views::Drawing,
    options: &[(String, usize)],
) -> super::sealed::Drawing {
    let rows = drawing
        .lines
        .into_iter()
        .map(|line| {
            let text = line.to_string();
            let option = options
                .iter()
                .find(|(label, _)| label == text.trim())
                .map(|(_, option)| *option);
            (line, option)
        })
        .collect();
    super::sealed::Drawing {
        next: drawing.next,
        ..super::sealed::Drawing::plain(rows)
    }
}
