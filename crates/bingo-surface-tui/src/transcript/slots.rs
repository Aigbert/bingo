use bingo_sdk::{Item, ItemBody};
use ratatui::text::{Line, Span};

use super::{Block, Rows, returns, speaks};
use crate::theme;

/// Apply only the fields a producer replaces; absent fields keep the item's drawing.
pub(super) fn placed_row(
    item: &Item,
    slot: &bingo_sdk::slot::Slot,
    rows: &Rows<'_>,
    mut block: Block,
) -> Block {
    let bingo_sdk::slot::Slot::Row {
        mark,
        title,
        badge,
        body,
        actions,
        ..
    } = slot
    else {
        return block;
    };
    let quiet = matches!(&item.body, ItemBody::ToolCall { quiet: true, output, .. }
        if !item.status.failed(output.as_ref()));
    if let Some(body) = body {
        block.lines.truncate(usize::from(!quiet));
        block.pictures.clear();
        block.wanted.clear();
        block.lines.extend(placed_body(body, rows));
    }
    if !quiet {
        if let Some(title) = title {
            let heading = speaks(
                theme::text(),
                vec![Line::from(Span::styled(title.clone(), theme::bold()))],
                rows,
            );
            block.lines.splice(..block.lines.len().min(1), heading);
        }
        if let Some(heading) = block.lines.first_mut() {
            placed_heading(heading, *mark, badge.as_deref());
        }
    }
    if !actions.is_empty() {
        block.lines.extend(placed_body(
            &bingo_sdk::View::Actions {
                items: actions.clone(),
            },
            rows,
        ));
    }
    block
}

fn placed_heading(heading: &mut Line<'static>, mark: Option<char>, badge: Option<&str>) {
    use unicode_width::UnicodeWidthChar;
    if let Some(mark) = mark.filter(|mark| mark.width() == Some(1))
        && let Some(bullet) = heading.spans.first_mut()
    {
        bullet.content = format!("{mark} ").into();
    }
    if let Some(badge) = badge {
        heading
            .spans
            .push(Span::styled(format!(" {badge}"), theme::dim()));
    }
}

fn placed_body(view: &bingo_sdk::View, rows: &Rows<'_>) -> Vec<Line<'static>> {
    returns(
        crate::views::marked(
            view,
            rows.result_width(),
            &crate::views::Marks::at(rows.now),
        ),
        rows,
    )
}

impl Rows<'_> {
    pub fn placed_on(
        &self,
        item: &bingo_sdk::ItemId,
    ) -> impl DoubleEndedIterator<Item = &bingo_sdk::slot::Slot> {
        self.slots
            .iter()
            .filter_map(move |placed| match &placed.slot {
                bingo_sdk::slot::Slot::Row { item: id, .. } if id == item => Some(&placed.slot),
                _ => None,
            })
    }
}
