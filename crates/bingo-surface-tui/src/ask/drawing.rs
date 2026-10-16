use crate::{
    component::sealed::{Drawing, Site},
    layout::Layout,
    theme,
    tree::{self, Tree},
    ui::{Card, Ui},
};
use bingo_sdk::InteractionKind;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
};

pub fn inline(tree: &Tree, ui: &Ui) -> bool {
    tree.open_interaction()
        .is_some_and(|(_, open)| !matches!(open.kind, InteractionKind::Login { .. }))
        && !ui.focus.showing()
}

pub fn splice(
    tree: &Tree,
    ui: &Ui,
    frame: &mut Layout,
    area: Rect,
    now: crate::clock::Now,
) -> Option<(usize, usize, Drawing)> {
    if !inline(tree, ui) {
        return None;
    }
    let (owner, interaction) = tree.open_interaction()?;
    let agent = (owner.summary.id != *tree.view()).then(|| tree::name(owner));
    let mut drawing = Site {
        state: owner,
        ui,
        interaction,
        agent: agent.as_deref(),
        width: area.width as usize,
        room: area.height as usize,
    }
    .rows(&mut frame.pulls, false);
    let span = (owner.summary.id == *tree.view())
        .then(|| {
            interaction
                .item
                .as_ref()
                .and_then(|item| frame.blocks.span(item))
        })
        .flatten();
    let (at, removed) = span
        .map(|(from, to)| (from, to - from))
        .unwrap_or((frame.height, 0));
    if !matches!(interaction.kind, InteractionKind::Permission { .. }) {
        drawing.rows.insert(
            0,
            (
                Line::from(Span::styled("   ◆ asks you · needs you", theme::presence())),
                None,
            ),
        );
    }
    drawing.rows = fit(
        drawing.rows,
        ui.dialog.focus,
        area.width as usize,
        area.height as usize,
    );
    if interaction
        .guard_until
        .is_some_and(|until| now.wall < until)
    {
        for (line, _) in &mut drawing.rows {
            for span in &mut line.spans {
                span.style = span.style.patch(theme::dim());
            }
        }
    }
    frame.inline = Some((at, removed, drawing.rows.len()));
    Some((at, removed, drawing))
}

pub fn place(frame: &mut Layout, mut drawing: Drawing, at: usize, area: Rect) {
    let Some(row) = at.checked_sub(frame.top) else {
        return;
    };
    let y = area.y + row as u16;
    let height = (drawing.rows.len() as u16).min(area.bottom().saturating_sub(y));
    let rect = Rect::new(area.x, y, area.width, height);
    drawing.place(rect);
    frame.card_actions = drawing.actions;
    frame.card = Some(Card {
        area: rect,
        options: drawing.rows.iter().map(|(_, index)| *index).collect(),
        sealed_options: if drawing.options.is_empty() {
            drawing
                .rows
                .iter()
                .enumerate()
                .filter_map(|(row, (_, option))| {
                    option.map(|option| {
                        (
                            Rect::new(rect.x, rect.y + row as u16, rect.width, 1),
                            option,
                        )
                    })
                })
                .collect()
        } else {
            drawing.options
        },
    });
}

pub fn activity(tree: &Tree, ui: &Ui) -> Option<Line<'static>> {
    let (owner, open) = tree.open_interaction()?;
    let name = match &open.kind {
        InteractionKind::Permission { summary, .. } => summary.clone(),
        InteractionKind::Form { title, .. } => title.clone().unwrap_or_else(|| "form".into()),
        _ => "a question".into(),
    };
    let waiting = if ui.dialog.aside {
        format!("{name} is waiting for you")
    } else {
        "waiting for you".into()
    };
    let mut asks: Vec<_> = tree
        .sessions()
        .flat_map(|s| s.interactions.iter().map(move |i| (s, i)))
        .collect();
    asks.sort_by_key(|(_, open)| (open.opened_at, &open.id));
    let queue = if asks.len() > 1 {
        let rest = asks
            .iter()
            .filter(|(_, i)| i.id != open.id)
            .map(|(s, _)| tree::name(s))
            .collect::<Vec<_>>()
            .join(", ");
        format!(" · 1/{} ◆ {} · then {rest}", asks.len(), tree::name(owner))
    } else {
        String::new()
    };
    Some(Line::from(Span::styled(
        format!(" ▍ {waiting}{queue}"),
        theme::presence(),
    )))
}

fn fit(
    rows: Vec<(Line<'static>, Option<usize>)>,
    focus: usize,
    width: usize,
    height: usize,
) -> Vec<(Line<'static>, Option<usize>)> {
    let rows = crate::view::wrapped_card_rows(rows, width);
    let prefix = rows
        .iter()
        .position(|(_, option)| option.is_some())
        .unwrap_or(0);
    let mut rows = crate::view::fitted_answers(rows, focus, height.saturating_sub(prefix));
    if rows.len() > height && height > 1 {
        let remove = rows.len() - height;
        rows.drain(1..1 + remove);
    }
    rows
}
