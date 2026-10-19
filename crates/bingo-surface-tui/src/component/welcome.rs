use crate::transcript::Rows;
use bingo_sdk::component::*;

pub fn props(rows: &Rows<'_>) -> Props {
    Props::Welcome(Box::new(WelcomeProps {
        session: rows.state.summary.clone(),
        effort: rows.state.summary.effort,
        modes: bingo_sdk::slot::modes(rows.state).collect(),
        update: rows.update.map(str::to_owned),
        resume: rows.entrance.resume.cloned(),
        started_at_ms: None,
        out_at_ms: None,
        recent: None,
        selected: None,
        branch: None,
        changed: None,
    }))
}

pub fn draw(rows: &Rows<'_>) -> crate::views::Drawing {
    let props = props(rows);
    super::resolve(
        rows.state,
        rows.pulled,
        "Welcome",
        &props,
        rows.width,
        &crate::views::Marks::at(rows.now),
        |_, _| Vec::new(),
    )
}

pub fn request(rows: &Rows<'_>) -> Option<crate::pulled::Key> {
    super::key(rows.state, "Welcome", &props(rows), rows.width as u16, 1)
}
