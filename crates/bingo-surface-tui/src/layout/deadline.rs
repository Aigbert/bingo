//! Each cue contributes its own next change; the frame takes the earliest.

use super::Layout;
use crate::clock::{FRAME, Now};
use crate::tree::Tree;
use crate::ui::Ui;
use std::time::Instant;

pub(super) fn next(tree: &Tree, ui: &Ui, layout: &Layout, now: Now) -> Option<Instant> {
    [
        crate::activity::next_change(tree, ui, now),
        ui.scroll.next_change(now),
        ui.layer.next_change(now),
        ui.notice().and_then(|notice| notice.next_change(now)),
        ui.next_change(now),
        layout.blocks.next_change(now),
        crate::status::next_change(tree, now),
        interaction(tree, now),
        layout.next_frame,
        ui.block
            .as_ref()
            .and_then(|view| crate::views::next_change(view, now)),
    ]
    .into_iter()
    .flatten()
    .min()
}

fn interaction(tree: &Tree, now: Now) -> Option<Instant> {
    let (_, interaction) = tree.open_interaction()?;
    let guard = interaction
        .guard_until
        .filter(|at| *at > now.wall)
        .map(|at| now.instant + at.duration_since(now.wall).unsigned_abs());
    let reveal = crate::view::opening(interaction, now);
    let arrival = reveal.moving().then(|| {
        let elapsed = now.past(interaction.opened_at);
        now.instant + FRAME
            - std::time::Duration::from_nanos((elapsed.as_nanos() % FRAME.as_nanos()) as u64)
    });
    let pulse = now
        .motion
        .then(|| crate::clock::next_tick(now, crate::theme::PULSE));
    let motion = arrival.into_iter().chain(pulse).min();
    guard.into_iter().chain(motion).min()
}
