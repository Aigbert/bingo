//! Typed substitutions for transcript blocks and their tool-call children.
use super::*;

/// The words of one item's block — every row of it but the picture its own
/// parts carry. An answer's words may name a picture themselves, and that one
/// is drawn among them ([`pictured::in_the_words`]), so this answers with a
/// block rather than lines.
pub(super) fn item_lines(
    item: &Item,
    place: &Place<'_>,
    agents: &Agents<'_>,
    rows: &Rows<'_>,
    cue: Cue,
    fold: Fold,
) -> Block {
    let mut block = component_item(item, place, agents, rows, cue, fold);
    if let Some(props) = crate::component::items::group(item, fold == Fold::Open) {
        block.lines = crate::component::resolve(
            rows.state,
            rows.pulled,
            &crate::component::items::id(item),
            &props,
            rows.width,
            |changed, width| {
                if changed == &props && width == rows.width {
                    return block.lines.clone();
                }
                let bingo_sdk::component::Props::ToolGroup(group) = changed else {
                    return Vec::new();
                };
                let rows = Rows {
                    width,
                    ..rows.clone()
                };
                group
                    .calls
                    .iter()
                    .flat_map(|call| {
                        let item = crate::component::items::changed(
                            item,
                            &bingo_sdk::component::Props::ToolUse(call.clone()),
                        );
                        component_item(
                            &item,
                            place,
                            agents,
                            &rows,
                            cue,
                            if group.is_expanded { Fold::Open } else { fold },
                        )
                        .lines
                    })
                    .collect()
            },
        );
    }
    match rows.placed_on(&item.id).next_back() {
        Some(slot) => slots::placed_row(item, slot, rows, cue, block),
        None => block,
    }
}

fn component_item(
    item: &Item,
    place: &Place<'_>,
    agents: &Agents<'_>,
    rows: &Rows<'_>,
    cue: Cue,
    fold: Fold,
) -> Block {
    let mut block = default_item_lines(item, place, agents, rows, cue, fold);
    if let Some(props) = crate::component::items::props(item, fold == Fold::Open)
        && let Some(key) = crate::component::key(
            rows.state,
            &crate::component::items::id(item),
            &props,
            rows.width as u16,
            0,
        )
        && let Some(answer) = rows.pulled.and_then(|memo| memo.get(&key))
    {
        block.lines = crate::component::draw(&props, Some(answer), rows.width, |changed, width| {
            let rows = &Rows {
                width,
                ..rows.clone()
            };
            if changed == &props && width == key.request.viewport.columns as usize {
                return block.lines.clone();
            }
            let changed_item = crate::component::items::changed(item, changed);
            let fold = match changed {
                bingo_sdk::component::Props::UserMessage(props) if props.is_expanded => Fold::Open,
                _ => fold,
            };
            default_item_lines(&changed_item, place, agents, rows, cue, fold).lines
        });
    }
    block
}

pub(super) fn component_call(
    item: &Item,
    call: Call<'_>,
    rows: &Rows<'_>,
    cue: Cue,
) -> Vec<Line<'static>> {
    let mut head = call.clone();
    head.output = None;
    head.progress = None;
    let mut out = tool_call(head, rows, cue);
    let Some(props) = crate::component::items::tail(item) else {
        return out;
    };
    out.extend(crate::component::resolve(
        rows.state,
        rows.pulled,
        &crate::component::items::id(item),
        &props,
        rows.width,
        |changed, width| {
            let rows = Rows {
                width,
                ..rows.clone()
            };
            match changed {
                bingo_sdk::component::Props::ToolResult(props) => {
                    let output = crate::component::items::output_of(&props.output);
                    let call = Call {
                        output: Some(&output),
                        status: if props.is_errored {
                            ItemStatus::Failed
                        } else {
                            ItemStatus::Completed
                        },
                        ..call.clone()
                    };
                    result(&call, &rows)
                }
                bingo_sdk::component::Props::ToolProgress(props) => {
                    returns(tail(&props.hint, TAIL_ROWS, rows.result_width()), &rows)
                }
                _ => Vec::new(),
            }
        },
    ));
    out
}
