use super::*;
use crate::{
    clock::{self, Now},
    theme,
    views::Drawing,
};
use bingo_sdk::component::Props;
use ratatui::text::{Line, Span};
pub fn draw(props: &Props, width: usize, now: Now) -> Drawing {
    let Props::Welcome(props) = props else {
        return Drawing::default();
    };
    let Some(at) = props.started_at_ms else {
        return Drawing::default();
    };
    let k = clock::prog(now, at, 900);
    let x = width.saturating_sub(52) / 2;
    let mut lines = mark(x.max(3), k, now);
    let ink = |s: String, ink| Span::styled(s, theme::foreground(ink).unwrap_or_default());
    let row = |spans: Vec<Span<'static>>| {
        let mut line = Line::raw(" ".repeat(x.max(3)));
        line.spans.extend(spans);
        line
    };
    lines.push(Line::default());
    lines.push(row(model(props)));
    lines.push(row(location(props)));
    lines.push(Line::default());
    if let Some(recent) = &props.recent
        && !recent.is_empty()
    {
        lines.push(row(vec![ink("recent".into(), "dim")]));
        lines.extend(
            recent
                .iter()
                .take(3)
                .enumerate()
                .map(|(i, session)| recent_row(session, props.selected == Some(i), x.max(3), now)),
        );
    }
    let moving = now.motion
        && props
            .out_at_ms
            .is_none_or(|at| clock::prog(now, at, 520) < 1.0);
    Drawing {
        lines,
        next: moving.then_some(now.instant + clock::FRAME),
    }
}

fn ink(text: String, ink: &str) -> Span<'static> {
    Span::styled(text, theme::foreground(ink).unwrap_or_default())
}

fn model(props: &bingo_sdk::component::WelcomeProps) -> Vec<Span<'static>> {
    let mut model = vec![ink(props.session.model.clone().unwrap_or_default(), "fg")];
    if let Some(effort) = props.effort {
        model.extend([
            ink(theme::glyphs::get("sep").into(), "dim"),
            ink(effort.name().into(), "muted"),
        ]);
    }
    let mode = props.modes.first().map(String::as_str).unwrap_or("default");
    model.extend([
        ink(theme::glyphs::get("sep").into(), "dim"),
        ink(
            mode.replace("acceptEdits", "accept edits"),
            match mode {
                "plan" => "plan",
                "bypass" => "bad",
                "acceptEdits" => "auto",
                _ => "muted",
            },
        ),
    ]);
    model
}

fn location(props: &bingo_sdk::component::WelcomeProps) -> Vec<Span<'static>> {
    let mut location = vec![ink(
        crate::paths::short(&props.session.cwd, "", crate::paths::home()),
        "muted",
    )];
    if let Some(branch) = &props.branch {
        location.extend([ink(" on ".into(), "dim"), ink(branch.clone(), "muted")]);
    }
    if let Some(changed) = props.changed {
        location.extend([
            ink(theme::glyphs::get("sep").into(), "dim"),
            ink(format!("{changed} changed"), "muted"),
        ]);
    }
    location
}

fn mark(x: usize, k: f64, now: Now) -> Vec<Line<'static>> {
    let rows = [
        "█      ▀                     ",
        "█▀▀▀▀█ █ █▀▀▀▀█ █▀▀▀▀█ █▀▀▀▀█",
        "█▄▄▄▄█ █ █    █ █▄▄▄▄█ █▄▄▄▄█",
        "                ▄▄▄▄▄█       ",
    ];
    let count = ((1.0 - (1.0 - k).powi(3)) * 29.0).round() as usize;
    rows.iter()
        .enumerate()
        .map(|(y, row)| {
            let mut line = Line::raw(" ".repeat(x));
            for (col, ch) in row.chars().enumerate() {
                let ch = if col >= count {
                    ' '
                } else if theme::glyphs::get("caret").is_ascii() {
                    match ch {
                        '█' => '#',
                        '▀' => '"',
                        '▄' => '_',
                        _ => ch,
                    }
                } else {
                    ch
                };
                let distance = count.saturating_sub(col);
                let color = if k < 1.0 && distance < 6 {
                    theme::ink::mix(
                        theme::palette().glow(),
                        theme::palette().fg(),
                        distance as f64 / 6.0,
                    )
                } else {
                    theme::palette().fg()
                };
                line.spans
                    .push(Span::styled(ch.to_string(), theme::literal(color)));
            }
            if y >= 1 {
                line.spans.truncate(1 + count);
                line.spans.push(Span::raw(" "));
                line.spans.push(Span::styled(
                    theme::glyphs::get("caret"),
                    theme::literal(theme::ink::mix(
                        theme::palette().glow(),
                        theme::palette().bg(),
                        if k < 1.0 { 0.0 } else { clock::caret_fade(now) },
                    )),
                ));
            }
            line
        })
        .collect()
}

fn recent_row(session: &SessionSummary, selected: bool, x: usize, now: Now) -> Line<'static> {
    let title = session.title.as_deref().unwrap_or("this session");
    let age = now
        .wall
        .duration_since(session.updated_at)
        .max(jiff::SignedDuration::ZERO)
        .unsigned_abs();
    let when = if age.as_secs() >= 86400 && age.as_secs() < 172800 {
        "yesterday".into()
    } else {
        clock::span(age)
    };
    let style = if selected {
        theme::text().patch(theme::bold())
    } else {
        theme::ash()
    };
    let mut line = Line::from(vec![
        Span::raw(" ".repeat(x)),
        Span::styled(title.to_owned(), style),
    ]);
    line = crate::layout::grid::pad_to(line, x + 52 - when.len());
    line.spans.push(Span::styled(when, theme::dim()));

    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    fn props(now: Now) -> Props {
        Props::Welcome(Box::new(bingo_sdk::component::WelcomeProps {
            session: summary(),
            effort: None,
            modes: vec![],
            update: None,
            resume: None,
            started_at_ms: Some(now.wall.as_millisecond()),
            out_at_ms: None,
            recent: None,
            selected: None,
            branch: None,
            changed: None,
        }))
    }

    #[test]
    fn welcome_writes_on_the_frame_clock_and_reduced_motion_is_settled() {
        let (_, now) = scene();
        let props = props(now);
        let fresh = draw(&props, 100, now);
        assert!(!fresh.lines[1].to_string().contains('█'));
        assert!(
            fresh.lines[1]
                .to_string()
                .contains(crate::theme::glyphs::get("caret"))
        );
        let middle = draw(&props, 100, later(now, 150));
        assert_ne!(fresh.lines, middle.lines);
        assert!(!middle.lines[1].to_string().contains('…'));
        assert_eq!(fresh.next, Some(now.instant + clock::FRAME));
        assert_eq!(draw(&props, 0, now).next, fresh.next);
        let settled = draw(&props, 100, still(now));
        assert!(settled.lines[1].to_string().contains("█▀▀▀▀█"));
        assert!(settled.next.is_none());
    }

    #[test]
    fn dissolve_has_a_deadline_until_its_last_frame() {
        let (_, now) = scene();
        let mut props = props(now);
        if let Props::Welcome(props) = &mut props {
            props.out_at_ms = Some(now.wall.as_millisecond());
        }
        assert!(draw(&props, 0, later(now, 519)).next.is_some());
        assert!(draw(&props, 0, later(now, 520)).next.is_none());
    }
}
