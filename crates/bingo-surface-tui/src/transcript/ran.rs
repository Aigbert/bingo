//! A shell line is its command band and its complete output well.
use super::Rows;
use crate::{layout::grid, theme};
use bingo_sdk::{Item, ItemBody, SessionState};
use ratatui::text::{Line, Span};

pub(super) fn lines(item: &Item, rows: &Rows<'_>) -> crate::views::Drawing {
    let ItemBody::Shell {
        command,
        output,
        exit,
        ..
    } = &item.body
    else {
        return Default::default();
    };
    if !item.is_terminal() {
        return running(item, command, rows);
    }
    let mut out = typed(item, command, *exit, rows);
    out.lines.extend(well(whole(output, *exit), rows.width));
    out
}

fn running(item: &Item, command: &str, rows: &Rows<'_>) -> crate::views::Drawing {
    use bingo_sdk::component::{Step, StepState, Tier};
    let step = Step {
        mark: '$',
        reads: Some(bingo_sdk::Reads::Command),
        tier: Tier::Change,
        verb: "run".into(),
        target: command.into(),
        state: StepState::Running,
        upshot: None,
        answer: None,
        why: None,
        landed_at: None,
        teammate: None,
        elapsed_ms: Some(
            rows.now
                .wall
                .duration_since(item.started_at)
                .unsigned_abs()
                .as_millis() as u64,
        ),
    };
    super::steps::step_block(item, &step, false, rows).into_text_drawing()
}

pub(crate) fn latest(state: &SessionState) -> Option<&Item> {
    let last = state.transcript.items().iter().rfind(|item| {
        matches!(item.body, ItemBody::Shell { .. }) || super::band::owns(item, state)
    })?;
    matches!(last.body, ItemBody::Shell { .. }).then_some(last)
}

fn typed(item: &Item, command: &str, exit: Option<i32>, rows: &Rows<'_>) -> crate::views::Drawing {
    let mut line = Line::from(vec![
        Span::raw("   "),
        Span::styled("!", theme::glow().patch(theme::bold())),
        Span::raw(" "),
        Span::styled(
            command.to_owned(),
            theme::foreground("code").unwrap_or_default(),
        ),
    ]);
    let newest = latest(rows.state).is_some_and(|last| last.id == item.id);
    let light = if newest {
        light(item, exit, rows)
    } else {
        Default::default()
    };
    if let Some(mark) = light.lines.first() {
        line.spans[0] = Span::raw("  ");
        line.spans.splice(0..0, mark.spans.iter().cloned());
    }
    crate::views::Drawing {
        lines: vec![grid::pad_to(line, rows.width).style(theme::ground("you").unwrap_or_default())],
        next: light.next,
    }
}

fn light(item: &Item, exit: Option<i32>, rows: &Rows<'_>) -> crate::views::Drawing {
    use bingo_sdk::view::{FadeCurve, TextChild, TextMotion, TextStyle};
    let failed = exit.is_some_and(|code| code != 0);
    let motion = item.completed_at.filter(|_| !failed).map(|end| {
        Box::new(TextMotion::Fade {
            curve: Some(FadeCurve::EaseOut),
            from: "glow".into(),
            at_ms: end.as_millisecond(),
            ms: 320,
        })
    });
    let view = bingo_sdk::View::Text {
        children: vec![TextChild::String(theme::glyphs::get("side").into())],
        style: TextStyle {
            color: Some(if failed { "bad" } else { "good" }.into()),
            motion,
            ..Default::default()
        },
    };
    crate::views::marked(&view, 1, &crate::views::Marks::at(rows.now))
}

fn well(lines: Vec<Line<'static>>, width: usize) -> Vec<Line<'static>> {
    if lines.is_empty() {
        return Vec::new();
    }
    crate::well::frame(lines, width)
}

pub fn whole(output: &str, exit: Option<i32>) -> Vec<Line<'static>> {
    let failed = exit.is_some_and(|code| code != 0);
    let ink = if failed { theme::bad() } else { theme::ash() };
    let mut lines: Vec<_> = output
        .lines()
        .map(|line| Line::from(Span::styled(line.to_owned(), ink)))
        .collect();
    if let Some(code) = exit.filter(|_| failed) {
        lines.push(Line::from(Span::styled(
            format!("exit {code}"),
            theme::bad(),
        )));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shell_output_never_folds_and_failure_keeps_its_exit() {
        let output = (1..=20)
            .map(|n| format!("line {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        let lines = whole(&output, Some(2));
        assert_eq!(lines.len(), 21);
        assert_eq!(lines.last().unwrap().to_string(), "exit 2");
        assert!(whole("", Some(0)).is_empty());
        assert_eq!(whole("one\n[interrupted]", None).len(), 2);
    }
}

#[cfg(test)]
mod live_tests {
    use crate::test_support::*;
    use bingo_sdk::{Event, ItemBody, ItemStatus};
    #[test]
    fn a_live_shell_draws_a_running_step_instead_of_a_settled_well() {
        let mut shell = item(
            "shell",
            ItemStatus::Running,
            ItemBody::Shell {
                command: "echo hello".into(),
                output: "hello".into(),
                exit: None,
                cwd: "/work".into(),
            },
        );
        shell.turn = None;
        let state = folded(vec![frame(1, Event::ItemStarted { item: shell })]);
        let (mut ui, now) = scene();
        let drawn = draw_tree(80, 24, &solo(&state), &mut ui, now);
        assert!(drawn.contains("run     echo hello"), "{drawn}");
        assert!(!drawn.contains("! echo hello"), "{drawn}");
    }
}
