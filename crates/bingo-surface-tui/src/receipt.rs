//! The three shell rows are a pure projection of the home session's facts.
use crate::{theme, views};
use bingo_sdk::{
    ItemBody, ItemStatus, SessionState, View,
    component::{Props, ReceiptFacts},
};
use ratatui::text::{Line, Span};

pub fn facts(
    state: &SessionState,
    now: jiff::Timestamp,
    tools: &crate::tool_rows::Looks,
) -> ReceiptFacts {
    let turns = bingo_sdk::transcript::turns(&state.transcript);
    let start = state
        .transcript
        .items()
        .iter()
        .find(|item| item.turn.is_some())
        .map(|item| item.started_at);
    let files: std::collections::BTreeSet<_> = state
        .transcript
        .items()
        .iter()
        .filter_map(|item| {
            let ItemBody::ToolCall {
                name,
                input,
                output: Some(output),
                ..
            } = &item.body
            else {
                return None;
            };
            if item.status != ItemStatus::Completed
                || output.is_error
                || !matches!(output.outcome(), Some(bingo_sdk::Upshot::Diff { .. }))
            {
                return None;
            }
            let row = crate::tool_rows::resolve(name, input, tools, false);
            (row.reads == Some(bingo_sdk::Reads::Path)).then_some(row.target)
        })
        .collect();
    ReceiptFacts {
        id: state.summary.id.clone(),
        title: state
            .summary
            .title
            .clone()
            .unwrap_or_else(|| "this session".into()),
        turns: turns.len(),
        duration_ms: start
            .map(|start| {
                now.duration_since(start)
                    .max(jiff::SignedDuration::ZERO)
                    .as_millis() as u64
            })
            .unwrap_or_default(),
        edited_files: files.len(),
    }
}

pub fn lines(facts: &ReceiptFacts) -> Vec<Line<'static>> {
    if facts.turns == 0 {
        return vec![];
    }
    let span =
        |text: String, ink: &str| Span::styled(text, theme::foreground(ink).unwrap_or_default());
    let plural = |n, word: &str| format!("{n} {word}{}", if n == 1 { "" } else { "s" });
    let mut details = vec![
        plural(facts.turns, "turn"),
        bingo_sdk::component::row::fmt_dur(facts.duration_ms),
    ];
    if facts.edited_files > 0 {
        details.push(format!("edited {}", plural(facts.edited_files, "file")));
    }
    vec![
        Line::from(vec![
            span(theme::glyphs::get("lamp").into(), "good"),
            Span::styled(" bingo", theme::bold()),
            span(theme::glyphs::get("sep").into(), "faint"),
            span(facts.title.clone(), "muted"),
        ]),
        Line::from(span(
            format!("  {}", details.join(theme::glyphs::get("sep"))),
            "dim",
        )),
        Line::from(vec![
            span("  resume with ".into(), "dim"),
            span(format!("bingo --resume {}", facts.id), "code"),
        ]),
    ]
}

pub fn props(state: &SessionState, facts: ReceiptFacts) -> Props {
    Props::SessionStatus(bingo_sdk::component::SessionStatusProps {
        receipt: Some(facts),
        model_scope: state.summary.model_scope,
        title: state.summary.title.clone(),
        cwd: state.summary.cwd.clone(),
        provider: state.summary.provider.clone(),
        model: state.summary.model.clone(),
        effort: state.summary.effort,
        context: state.context,
        usage: state.summary.usage,
    })
}

pub fn draw(props: &Props, answer: Option<&View>, width: usize) -> Vec<Line<'static>> {
    crate::component::draw_marked(
        props,
        answer,
        width,
        &views::Marks::default(),
        |props, _| {
            let Props::SessionStatus(props) = props else {
                return vec![];
            };
            props.receipt.as_ref().map(lines).unwrap_or_default()
        },
    )
    .lines
}

pub fn print(lines: &[Line<'static>]) -> std::io::Result<()> {
    use crossterm::{
        queue,
        style::{Attribute, Print, ResetColor, SetAttribute, SetStyle},
    };
    use ratatui::backend::IntoCrossterm;
    use std::io::Write;
    let mut stdout = std::io::stdout().lock();
    for line in lines {
        for span in &line.spans {
            queue!(stdout, ResetColor, SetAttribute(Attribute::Reset))?;
            queue!(stdout, SetStyle(span.style.into_crossterm()))?;
            queue!(stdout, Print(&span.content))?;
        }
        queue!(
            stdout,
            ResetColor,
            SetAttribute(Attribute::Reset),
            Print("\n")
        )?;
    }
    stdout.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    #[test]
    fn receipt_rows_are_exact_and_empty_sessions_leave_nothing() {
        let facts = ReceiptFacts {
            id: bingo_sdk::SessionId::from_raw("ses_1"),
            title: "hello".into(),
            turns: 2,
            duration_ms: 4900,
            edited_files: 1,
        };
        assert_eq!(
            lines(&facts)
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            [
                "● bingo · hello",
                "  2 turns · 4s · edited 1 file",
                "  resume with bingo --resume ses_1"
            ]
        );
        assert!(lines(&ReceiptFacts { turns: 0, ..facts }).is_empty());
        let (state, _) = crate::test_support::ember::story::scene("done");
        let facts = super::facts(
            &state,
            scene().1.wall,
            &crate::tool_rows::Looks::of(&crate::test_support::ember::story::catalogue()),
        );
        assert_eq!(facts.turns, 2);
        assert_eq!(facts.edited_files, 2);
    }
}

#[cfg(test)]
mod override_tests {
    use super::*;
    use crate::test_support::*;

    #[test]
    fn receipt_facts_and_drawing_are_replaceable() {
        let state = state();
        let facts = ReceiptFacts {
            id: state.summary.id.clone(),
            title: "home".into(),
            turns: 1,
            duration_ms: 1000,
            edited_files: 0,
        };
        let props = super::props(&state, facts);
        let replaced = draw(&props, Some(&View::text("custom receipt")), 80);
        assert_eq!(replaced[0].to_string(), "custom receipt");
        let mut changed = props.clone();
        if let Props::SessionStatus(props) = &mut changed {
            props.receipt.as_mut().unwrap().title = "redrawn".into();
        }
        let engine = View::Engine {
            component: changed.component(),
            props: changed.value(),
        };
        assert_eq!(
            draw(&props, Some(&engine), 80)[0].to_string(),
            "● bingo · redrawn"
        );
    }
}
