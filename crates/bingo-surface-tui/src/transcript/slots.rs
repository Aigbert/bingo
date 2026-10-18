use bingo_sdk::{Item, ItemBody};
use ratatui::text::{Line, Span};

use super::{Block, Cue, Landing, Rows, live_bullet, returns, signature, speaks};
use crate::theme;

/// Apply only the fields a producer replaces; absent fields keep the item's drawing.
pub(super) fn placed_row(
    item: &Item,
    slot: &bingo_sdk::slot::Slot,
    rows: &Rows<'_>,
    cue: Cue,
    mut block: Block,
) -> Block {
    let bingo_sdk::slot::Slot::Row {
        mark,
        title,
        badge,
        caption,
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
        let drawn = placed_body(body, rows);
        block.next = drawn.next;
        block.lines.extend(drawn.lines);
    }
    if !quiet {
        if let Some(title) = title {
            let heading = replacement_heading(item, title, badge.as_deref(), rows, cue);
            block.lines.splice(..block.lines.len().min(1), heading);
        }
        if let Some(heading) = block.lines.first_mut() {
            placed_heading(heading, *mark, badge.as_deref().filter(|_| title.is_none()));
            if let Some(caption) = caption.as_deref().filter(|caption| !caption.is_empty()) {
                heading
                    .spans
                    .push(Span::styled(format!(" {caption}"), theme::text()));
            }
        }
    }
    if !actions.is_empty() {
        let drawn = placed_body(
            &bingo_sdk::View::Actions {
                items: actions.clone(),
            },
            rows,
        );
        block.next = block.next.into_iter().chain(drawn.next).min();
        block.lines.extend(drawn.lines);
    }
    block
}

fn replacement_heading(
    item: &Item,
    title: &str,
    badge: Option<&str>,
    rows: &Rows<'_>,
    cue: Cue,
) -> Vec<Line<'static>> {
    let failed = match &item.body {
        ItemBody::ToolCall { output, .. } => item.status.failed(output.as_ref()),
        _ => false,
    };
    let landing = match item.body {
        ItemBody::User { .. } => Landing::Settled,
        _ => Landing::of(cue, failed, rows),
    };
    speaks(
        live_bullet(item.status, failed, rows),
        vec![signature(title, badge.unwrap_or_default(), rows, landing)],
        rows,
    )
}

fn placed_heading(heading: &mut Line<'static>, mark: Option<char>, badge: Option<&str>) {
    if let Some(mark) = mark
        && let Some(bullet) = heading.spans.first_mut()
    {
        bullet.content = format!(" {} ", theme::row_mark(mark)).into();
    }
    if let Some(badge) = badge {
        heading
            .spans
            .push(Span::styled(format!(" {badge}"), theme::dim()));
    }
}

/// The actions a producer offers on a step, under the step's words: the
/// last row placed on it decides, as it does for a row of its own.
pub(super) fn actions_under(item: &Item, indent: usize, rows: &Rows<'_>) -> Vec<Line<'static>> {
    let Some(bingo_sdk::slot::Slot::Row { actions, .. }) = rows.placed_on(&item.id).next_back()
    else {
        return Vec::new();
    };
    if actions.is_empty() {
        return Vec::new();
    }
    let view = bingo_sdk::View::Actions {
        items: actions.clone(),
    };
    let width = rows.width.saturating_sub(indent).max(1);
    let body = crate::views::marked(&view, width, &crate::views::Marks::at(rows.now));
    super::under(
        Span::raw(" ".repeat(indent)),
        body.lines,
        indent,
        rows.width,
    )
}

fn placed_body(view: &bingo_sdk::View, rows: &Rows<'_>) -> crate::views::Drawing {
    crate::views::marked(
        view,
        rows.result_width(),
        &crate::views::Marks::at(rows.now),
    )
    .map_lines(|lines| returns(lines, rows))
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

#[cfg(test)]
pub(crate) mod fixtures {
    use crate::test_support::*;
    use bingo_sdk::{ItemBody, ItemId, SessionState};
    use serde_json::json;

    /// A producer's journaled replacement for a transcript row.
    pub fn with_row(mut state: SessionState, slot: bingo_sdk::slot::Slot) -> SessionState {
        let bingo_sdk::slot::Slot::Row { item, .. } = &slot else {
            panic!("row fixture")
        };
        let kind = format!("row:{item}");
        state.apply(&frame(
            state.seq.0 + 1,
            extended(
                "fixture.rows",
                &kind,
                bingo_sdk::slot::with(serde_json::json!({}), [slot]),
            ),
        ));
        state
    }

    pub fn with_skill_row(state: SessionState, item: &str, name: &str, args: &str) -> SessionState {
        with_row(
            state,
            bingo_sdk::slot::Slot::Row {
                item: ItemId::from_raw(item),
                mark: Some('❖'),
                title: Some("Skill".into()),
                badge: Some(name.into()),
                caption: (!args.is_empty()).then(|| args.into()),
                body: None,
                actions: vec![],
            },
        )
    }

    pub fn with_skill_tool_rows(mut state: SessionState) -> SessionState {
        let rows: Vec<_> = state
            .transcript
            .items()
            .iter()
            .filter_map(|item| match &item.body {
                ItemBody::ToolCall { name, input, .. } if name == "Skill" => Some((
                    item.id.to_string(),
                    input["name"].as_str()?.to_string(),
                    input["arguments"].as_str().unwrap_or_default().to_string(),
                )),
                _ => None,
            })
            .collect();
        for (id, name, args) in rows {
            state = with_skill_row(state, &id, &name, &args);
        }
        state
    }

    pub fn with_promotions(mut state: SessionState) -> SessionState {
        let rows: Vec<_> = state
            .transcript
            .items()
            .iter()
            .filter_map(|item| match &item.body {
                ItemBody::ToolCall { name, call_id, .. } if name == "Bash" => {
                    Some(bingo_sdk::slot::Slot::Row {
                        item: item.id.clone(),
                        mark: None,
                        title: None,
                        badge: None,
                        caption: None,
                        body: None,
                        actions: vec![bingo_sdk::ActionItem {
                            label: "background".into(),
                            action: bingo_sdk::Action {
                                name: "bash.promote".into(),
                                args: serde_json::json!(call_id),
                            },
                            key: Some('b'),
                        }],
                    })
                }
                _ => None,
            })
            .collect();
        for row in rows {
            state = with_row(state, row);
        }
        state
    }
    #[test]
    fn a_stranger_places_a_row_with_a_title_badge_and_action() {
        let state = folded(vec![
            frame(
                1,
                bingo_sdk::Event::ItemStarted {
                    item: running_tool("itm_1", "Inspect", "checking"),
                },
            ),
            frame(
                2,
                extended(
                    "stranger",
                    "row:itm_1",
                    bingo_sdk::slot::with(
                        json!({}),
                        [bingo_sdk::slot::Slot::Row {
                            caption: Some("details".into()),
                            item: bingo_sdk::ItemId::from_raw("itm_1"),
                            mark: Some('❖'),
                            title: Some("Review".into()),
                            badge: Some("sample".into()),
                            body: None,
                            actions: vec![bingo_sdk::ActionItem {
                                label: "open".into(),
                                action: bingo_sdk::Action {
                                    name: "stranger.open".into(),
                                    args: json!("sample"),
                                },
                                key: Some('b'),
                            }],
                        }],
                    ),
                ),
            ),
        ]);
        let (mut ui, now) = scene();
        let screen = draw_tree(80, 24, &solo(&state), &mut ui, now);
        assert!(screen.contains("❖ Review(sample) details"), "{screen}");
        assert!(screen.contains("open"), "{screen}");
    }
}
