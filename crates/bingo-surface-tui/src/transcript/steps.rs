//! One Ledger request per step; details remain attached to that step.
use super::*;
use crate::turns::Role;
use bingo_sdk::component::{Props, Step, StepState};
const HANG: usize = 5;

pub(super) fn part(item: &Item, role: &Role, rows: &Rows<'_>) -> Option<Block> {
    match role {
        Role::Own => None,
        Role::Hidden => Some(Block::default()),
        Role::Step { step, opened } => Some(step_block(item, step, *opened, rows)),
        Role::Thought { opened } => Some(super::thinking::detail(item, *opened, rows.width).into()),
        Role::Addition => Some(super::band::addition(item, rows).into()),
        Role::Narration => Some(narration(item, rows).into()),
    }
}

pub(super) fn step_block(item: &Item, step: &Step, opened: bool, rows: &Rows<'_>) -> Block {
    let (id, props) = crate::turns::ledger(item, step, rows.now);
    let mut out = crate::component::resolve(
        rows.state,
        rows.pulled,
        &id,
        &props,
        rows.width,
        &crate::views::Marks::at(rows.now),
        |changed, width| ledger(changed, width, rows.now),
    );
    if step.state == StepState::Running {
        out.next = out
            .next
            .into_iter()
            .chain(
                bingo_sdk::transcript::next_second(
                    rows.now.wall,
                    crate::turns::running_for(item, rows.now),
                )
                .map(|at| rows.now.deadline(at)),
            )
            .min();
        out.append(progress(item, step.target_column(), rows));
    }
    if opened {
        out.append(result(item, step, rows));
    } else if fails_turn(item, step, rows) {
        out.lines.extend(fail_tail(item, rows));
    }
    Block::from(out)
}

pub(crate) fn asked(step: &Step, width: usize) -> Line<'static> {
    crate::views::marked(
        &step.drawing(&crate::turns::marks(), width),
        width,
        &Default::default(),
    )
    .lines
    .into_iter()
    .next()
    .unwrap_or_default()
}

fn ledger(props: &Props, width: usize, now: Now) -> crate::views::Drawing {
    let Props::Ledger(ledger) = props else {
        return Default::default();
    };
    ledger
        .steps
        .iter()
        .map(|step| {
            crate::views::marked(
                &step.drawing(&ledger.marks, width),
                width,
                &crate::views::Marks::at(now),
            )
        })
        .collect()
}

fn narration(item: &Item, rows: &Rows<'_>) -> crate::views::Drawing {
    let ItemBody::Assistant { text } = &item.body else {
        return Default::default();
    };
    let lines = wrap::wrap_all(
        &[Line::from(Span::styled(
            text.clone(),
            theme::ash().patch(theme::italic()),
        ))],
        rows.measured().saturating_sub(5).min(100),
    );
    crate::views::Drawing {
        lines: under(Span::raw("   "), lines, 3, rows.width),
        next: None,
    }
}

fn fails_turn(item: &Item, step: &Step, rows: &Rows<'_>) -> bool {
    let state = rows.state;
    step.state == StepState::Failed
        && state
            .turn
            .as_ref()
            .is_none_or(|t| Some(&t.id) != item.turn.as_ref())
        && !rows.followed.contains(&item.id)
}

fn fail_tail(item: &Item, rows: &Rows<'_>) -> Vec<Line<'static>> {
    let ItemBody::ToolCall {
        output, progress, ..
    } = &item.body
    else {
        return Vec::new();
    };
    let text = output
        .as_ref()
        .map(|output| {
            output
                .parts
                .iter()
                .filter_map(bingo_sdk::ContentPart::as_text)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|| progress.clone().unwrap_or_default());
    let lines: Vec<_> = text.lines().collect();
    if lines.is_empty() {
        return Vec::new();
    }
    let ground = theme::ground("well").unwrap_or_default();
    let width = rows.width.saturating_sub(HANG + 2);
    let blank = Line::from(Span::styled(" ".repeat(width), ground));
    let mut out = vec![blank.clone()];
    for text in lines.iter().skip(lines.len().saturating_sub(4)) {
        let bad = ["error", "failed", "panicked"]
            .iter()
            .any(|word| text.to_lowercase().contains(word));
        let ink = if bad { theme::bad() } else { theme::ash() };
        let line = crate::layout::grid::fit(
            Line::from(Span::styled((*text).to_owned(), ink)),
            width.saturating_sub(4),
        );
        let mut spans = vec![Span::raw("  ")];
        spans.extend(line.spans);
        out.push(crate::layout::grid::pad_to(Line::from(spans), width).style(ground));
    }
    out.push(blank);
    out.into_iter()
        .map(|mut line| {
            for span in &mut line.spans {
                span.style = ground.patch(span.style);
            }
            line.style = Style::default();
            line.spans.insert(0, Span::raw(" ".repeat(HANG)));
            line
        })
        .collect()
}

fn progress(item: &Item, ax: usize, rows: &Rows<'_>) -> crate::views::Drawing {
    let Some(props @ Props::ToolProgress(_)) = crate::component::items::tail(item) else {
        return Default::default();
    };
    crate::component::resolve(
        rows.state,
        rows.pulled,
        &crate::component::items::id(item),
        &props,
        rows.width,
        &crate::views::Marks::at(rows.now),
        |props, width| {
            let Props::ToolProgress(props) = props else {
                return Vec::new();
            };
            let last = props.hint.lines().last().unwrap_or_default();
            let line = crate::layout::grid::fit(
                Line::from(Span::styled(last.trim_start().to_owned(), theme::dim())),
                width.saturating_sub(ax + 2),
            );
            under(Span::raw(" ".repeat(ax)), vec![line], ax, width)
        },
    )
}

fn result(item: &Item, step: &Step, rows: &Rows<'_>) -> crate::views::Drawing {
    let width = crate::well::inner(rows.width);
    let skipped = step.state == StepState::Skipped;
    let drawing = if let Some((id, props)) = crate::turns::result(item) {
        crate::component::resolve(
            rows.state,
            rows.pulled,
            &id,
            &props,
            width,
            &crate::views::Marks::at(rows.now),
            |props, width| {
                let Props::ToolResult(result) = props else {
                    return Default::default();
                };
                let output = crate::component::items::output_of(&result.output);
                if rows.opened.browse_hit == Some(&item.id) {
                    super::output::whole(&output, width).into()
                } else {
                    crate::well::content(Some(&output), step.tier, skipped, width, rows.now)
                }
            },
        )
    } else {
        crate::well::content(None, step.tier, skipped, width, rows.now)
    };
    drawing.map_lines(|lines| {
        if rows.opened.browse_hit == Some(&item.id) {
            super::step_body(lines, rows)
        } else {
            crate::well::frame(lines, rows.width)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{folded, frame, scene, started, still, tool};
    use bingo_sdk::Event;

    #[test]
    fn a_finished_step_hangs_nothing_until_opened() {
        let mut call = tool(
            "step",
            "Bash",
            serde_json::json!({"command":"cat note.txt"}),
            None,
            ItemStatus::Completed,
        );
        if let ItemBody::ToolCall { output, .. } = &mut call.body {
            let mut value = bingo_sdk::ToolOutput::text("one\ntwo\nthree");
            value.display = Some(bingo_sdk::View::text("one\ntwo\nthree"));
            *output = Some(Box::new(value));
        }
        let state = folded(vec![
            frame(1, started("trn_1")),
            frame(2, Event::ItemCompleted { item: call.clone() }),
        ]);
        let folds = Default::default();
        let pictures = Default::default();
        let linked = Default::default();
        let rows = &Rows::of(&state, 80, &folds, &pictures, &linked, still(scene().1));
        {
            let step = crate::turns::step(&call, rows).unwrap();
            let closed = step_block(&call, &step, false, rows);
            assert_eq!(closed.lines.len(), 1);
            let open = step_block(&call, &step, true, rows);
            assert!(
                open.lines
                    .iter()
                    .any(|line| line.to_string().contains("three"))
            );
        }
    }
}
