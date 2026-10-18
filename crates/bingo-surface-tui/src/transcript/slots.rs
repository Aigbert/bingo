use bingo_sdk::{Item, ItemBody};
use ratatui::text::{Line, Span};

use super::{Block, Cue, Landing, Rows, live_bullet, returns, signature};
use crate::theme;

/// Apply only the fields a producer replaces; absent fields keep the item's drawing.
pub(super) fn placed_row(
    item: &Item,
    slot: &bingo_sdk::slot::Slot,
    rows: &Rows<'_>,
    cue: Cue,
    mut block: Block,
) -> Block {
    let bingo_sdk::slot::Slot::Row { body, actions, .. } = slot else {
        return block;
    };
    let quiet = quiet(item);
    if let Some(body) = body {
        block.lines.truncate(usize::from(!quiet));
        block.pictures.clear();
        block.wanted.clear();
        let drawn = placed_body(body, rows);
        block.next = block.next.into_iter().chain(drawn.next).min();
        block.lines.extend(drawn.lines);
    }
    if let Some(heading) = heading(item, slot, rows, cue) {
        block.next = block.next.into_iter().chain(heading.next).min();
        block
            .lines
            .splice(..block.lines.len().min(1), heading.lines);
    }
    if !quiet && !matches!(item.body, ItemBody::ToolCall { .. }) {
        decorate_heading(block.lines.first_mut(), slot);
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

fn heading(
    item: &Item,
    slot: &bingo_sdk::slot::Slot,
    rows: &Rows<'_>,
    cue: Cue,
) -> Option<crate::views::Drawing> {
    let bingo_sdk::slot::Slot::Row {
        mark,
        title,
        badge,
        caption,
        ..
    } = slot
    else {
        return None;
    };
    if quiet(item) {
        return None;
    }
    if matches!(item.body, ItemBody::ToolCall { .. }) {
        return step_heading(
            item,
            *mark,
            title.as_deref(),
            badge.as_deref(),
            caption.as_deref(),
            rows,
        );
    }
    title
        .as_ref()
        .map(|title| replacement_heading(item, title, badge.as_deref(), rows, cue))
}

fn quiet(item: &Item) -> bool {
    matches!(&item.body, ItemBody::ToolCall { quiet: true, output, .. }
        if !item.status.failed(output.as_deref()))
}

fn decorate_heading(heading: Option<&mut Line<'static>>, slot: &bingo_sdk::slot::Slot) {
    let bingo_sdk::slot::Slot::Row {
        mark,
        title,
        badge,
        caption,
        ..
    } = slot
    else {
        return;
    };
    if let Some(heading) = heading {
        placed_heading(heading, *mark, badge.as_deref().filter(|_| title.is_none()));
        if let Some(caption) = caption.as_deref().filter(|caption| !caption.is_empty()) {
            heading
                .spans
                .push(Span::styled(format!(" {caption}"), theme::text()));
        }
    }
}

fn step_heading(
    item: &Item,
    mark: Option<char>,
    title: Option<&str>,
    badge: Option<&str>,
    caption: Option<&str>,
    rows: &Rows<'_>,
) -> Option<crate::views::Drawing> {
    let mut step = crate::turns::step(item, rows)?;
    if let Some(mark) = mark {
        step.mark = bingo_sdk::look::mark(Some(mark), theme::bullet().is_ascii());
    }
    if let Some(title) = title {
        step.verb = title.to_owned();
    }
    if let Some(badge) = badge {
        step.target = badge.to_owned();
    }
    if let Some(caption) = caption {
        step.target.push(' ');
        step.target.push_str(caption);
    }
    Some(crate::views::marked(
        &step.drawing(&crate::turns::marks(), rows.width),
        rows.width,
        &crate::views::Marks::at(rows.now),
    ))
}

fn replacement_heading(
    item: &Item,
    title: &str,
    badge: Option<&str>,
    rows: &Rows<'_>,
    cue: Cue,
) -> crate::views::Drawing {
    let failed = match &item.body {
        ItemBody::ToolCall { output, .. } => item.status.failed(output.as_deref()),
        _ => false,
    };
    let landing = match item.body {
        ItemBody::User { .. } => Landing::Settled,
        _ => Landing::of(cue, failed, rows),
    };
    live_bullet(item.status, failed, rows).heading(
        vec![signature(title, badge.unwrap_or_default(), rows, landing)],
        rows,
    )
}

fn placed_heading(heading: &mut Line<'static>, mark: Option<char>, badge: Option<&str>) {
    if let Some(mark) = mark
        && let Some(bullet) = heading.spans.first_mut()
    {
        bullet.content = format!(
            " {} ",
            bingo_sdk::look::mark(Some(mark), theme::bullet().is_ascii())
        )
        .into();
    }
    if let Some(badge) = badge {
        heading
            .spans
            .push(Span::styled(format!(" {badge}"), theme::dim()));
    }
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
        if state
            .transcript
            .get(&ItemId::from_raw(item))
            .is_some_and(|item| matches!(item.body, ItemBody::ToolCall { .. }))
        {
            panic!("a typed skill row cannot replace a tool call");
        }
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

    #[test]
    #[should_panic(expected = "a typed skill row cannot replace a tool call")]
    fn a_skill_tool_cannot_use_a_typed_commands_row() {
        let state = folded(vec![frame(
            1,
            bingo_sdk::Event::ItemStarted {
                item: running_tool("call", "Skill", "guide"),
            },
        )]);
        with_skill_row(state, "call", "guide", "");
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
                            meaning: None,
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
                            body: Some(bingo_sdk::View::text("placed body")),
                            actions: vec![bingo_sdk::ActionItem {
                                meaning: None,
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
        assert!(screen.contains("Review  sample details"), "{screen}");
        assert!(screen.contains("open"), "{screen}");
        assert!(screen.contains("placed body"), "{screen}");
    }
}
