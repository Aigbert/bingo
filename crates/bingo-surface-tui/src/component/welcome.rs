use crate::transcript::Rows;
use bingo_sdk::component::*;

pub fn props(rows: &Rows<'_>) -> Props {
    Props::Welcome(Box::new(WelcomeProps {
        session: rows.state.summary.clone(),
        effort: rows.state.summary.effort,
        modes: bingo_sdk::slot::modes(rows.state).collect(),
        update: rows.update.map(str::to_owned),
        resume: rows.entrance.resume.cloned(),
        started_at_ms: rows.entrance.started_at_ms,
        out_at_ms: rows
            .state
            .transcript
            .items()
            .iter()
            .find_map(|item| bingo_sdk::asked(item).map(|_| item.started_at.as_millisecond())),
        recent: None,
        selected: rows.entrance.selected,
        branch: rows.slots.iter().find_map(|placed| {
            if let bingo_sdk::slot::Slot::Location { branch, .. } = &placed.slot {
                branch.clone()
            } else {
                None
            }
        }),
        changed: rows.slots.iter().find_map(|placed| {
            if let bingo_sdk::slot::Slot::Location { changed, .. } = &placed.slot {
                *changed
            } else {
                None
            }
        }),
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
        |props, width| {
            let mut props = props.clone();
            if let Props::Welcome(props) = &mut props {
                props.selected = rows.entrance.selected;
                props.out_at_ms = rows.state.transcript.items().iter().find_map(|item| {
                    bingo_sdk::asked(item).map(|_| item.started_at.as_millisecond())
                });
            }
            crate::welcome::draw(&props, width, rows.now)
        },
    )
}

pub fn request(rows: &Rows<'_>) -> Option<crate::pulled::Key> {
    super::key(rows.state, "Welcome", &props(rows), rows.width as u16, 1)
}
