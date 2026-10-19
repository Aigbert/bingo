//! Sealed cards use facts and gestures from the open interaction alone.
use crate::{dialog, views, wrap};
use bingo_sdk::{Interaction, InteractionKind, SessionState, View, component::*};
use ratatui::{layout::Rect, text::Line};

type Rows = Vec<(Line<'static>, Option<usize>)>;

pub fn props(_state: &SessionState, interaction: &Interaction) -> Option<Props> {
    match &interaction.kind {
        InteractionKind::Permission { tool, input, .. } => {
            Some(Props::Permission(PermissionProps {
                interaction_id: interaction.id.clone(),
                tool: tool.clone(),
                input: input.clone(),
                answers: interaction.answers.clone(),
            }))
        }
        InteractionKind::Login { provider, flow } => Some(Props::SignIn(SignInProps {
            interaction_id: interaction.id.clone(),
            provider: provider.clone(),
            flow: flow.clone(),
        })),
        _ => None,
    }
}

pub struct Site<'a> {
    pub state: &'a SessionState,
    pub ui: &'a crate::ui::Ui,
    pub interaction: &'a Interaction,
    pub agent: Option<&'a str>,
    pub width: usize,
    pub room: usize,
    pub now: crate::clock::Now,
}

pub struct Drawing {
    pub next: Option<std::time::Instant>,
    pub rows: Rows,
    pub options: Vec<(Rect, usize)>,
    pub actions: Vec<views::targets::Target>,
}

impl Drawing {
    pub(super) fn plain(rows: Rows) -> Self {
        Self {
            rows,
            next: None,
            options: Vec::new(),
            actions: Vec::new(),
        }
    }
}

impl Site<'_> {
    pub fn rows(&self, pulls: &mut Vec<crate::pulled::Key>, moving: bool) -> Drawing {
        if !moving && let Some(drawing) = self.draw(pulls) {
            return drawing;
        }
        super::question::rows(self, pulls)
    }

    pub fn draw(&self, pulls: &mut Vec<crate::pulled::Key>) -> Option<Drawing> {
        let props = props(self.state, self.interaction)?;
        let key = super::key(
            self.state,
            self.interaction.id.as_str(),
            &props,
            self.width as u16,
            self.room as u16,
        )?;
        pulls.push(key.clone());
        // A stale answer may describe a different input, even with the same interaction id.
        let answer = self.ui.placed.pulled.get(&key)?;
        if *answer == key.request.engine() {
            return None;
        }
        Some(
            self.checked(&props, answer)
                .unwrap_or_else(|| self.fallback()),
        )
    }

    fn checked(&self, props: &Props, answer: &View) -> Option<Drawing> {
        bingo_sdk::view::sealed_ok(props.component(), &props.value(), answer).ok()?;
        let engine_options = std::cell::RefCell::new(Vec::new());
        let own = |_, _: &serde_json::Value, width| {
            let rows = self.own(width);
            *engine_options.borrow_mut() = rows.iter().map(|(_, option)| *option).collect();
            views::Drawing::still(rows.into_iter().map(|(line, _)| line).collect())
        };
        let expected = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let marks = views::Marks {
            sealed: Some(expected.clone()),
            ..Default::default()
        };
        let mask = views::marked_with(answer, self.width, &marks, &own);
        let areas = views::sealed::whole(&mask.lines, &expected.borrow(), self.width, self.room)?;
        let lines = views::marked_with(answer, self.width, &views::Marks::at(self.now), &own);
        if lines.lines.len() > self.room {
            return None;
        }
        let options = engine_options
            .borrow()
            .iter()
            .zip(areas)
            .filter_map(|(option, area)| option.map(|option| (area, option)))
            .collect();
        let targets = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let marks = views::Marks {
            targets: Some(targets.clone()),
            ..Default::default()
        };
        let mask = views::marked_with(answer, self.width, &marks, &own);
        let actions = views::targets::cells(
            &mask.lines,
            &targets.borrow(),
            Rect::new(0, 0, self.width as u16, self.room as u16),
        );
        Some(Drawing {
            next: lines.next,
            rows: lines.lines.into_iter().map(|line| (line, None)).collect(),
            options,
            actions,
        })
    }

    fn own(&self, width: usize) -> Rows {
        // A docked sign-in seals the sheet a person would otherwise see.
        if let Some(rows) = self.sheet(width, self.room) {
            return rows;
        }
        let rows = dialog::rows(
            &self.ui.dialog,
            self.interaction,
            self.agent,
            width,
            self.room,
        );
        rows.into_iter()
            .flat_map(|(line, option)| {
                wrap::wrap_all(&[line], width)
                    .into_iter()
                    .map(move |line| (line, option))
            })
            .collect()
    }

    /// The device code's sheet, the rows a docked sign-in shows.
    fn sheet(&self, width: usize, room: usize) -> Option<Rows> {
        let view = crate::sign_in::sheet(&self.interaction.kind, width as u16, room as u16)?;
        let drawn = views::marked(&view, width, &views::Marks::at(self.now));
        Some(drawn.lines.into_iter().map(|line| (line, None)).collect())
    }

    fn fallback(&self) -> Drawing {
        let room = self.room.saturating_sub(1);
        let mut rows = self.sheet(self.width, room).unwrap_or_else(|| {
            dialog::rows(
                &self.ui.dialog,
                self.interaction,
                self.agent,
                self.width,
                self.room,
            )
        });
        rows.push((
            Line::from("mod drawing cannot show the sealed card whole; showing host card"),
            None,
        ));
        Drawing {
            next: None,
            rows,
            options: Vec::new(),
            actions: Vec::new(),
        }
    }
}

impl Drawing {
    pub fn place(&mut self, area: Rect) {
        for (rect, _) in &mut self.options {
            rect.x += area.x;
            rect.y += area.y;
        }
        for target in &mut self.actions {
            target.area.x += area.x;
            target.area.y += area.y;
        }
    }
}

#[cfg(test)]
mod tests;
