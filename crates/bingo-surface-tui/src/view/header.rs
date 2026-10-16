use bingo_sdk::{SessionState, SessionSummary};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
};
use unicode_width::UnicodeWidthStr;

use crate::{layout::Layout, paths, status, theme, ui::Ui};

pub(super) fn render(
    state: &SessionState,
    ui: &Ui,
    frame: &mut Layout,
    area: Rect,
    now: crate::clock::Now,
) {
    if area.height == 0 {
        return;
    }
    let rows = crate::transcript::Rows::of(
        state,
        usize::from(area.width),
        &ui.folds,
        &ui.catalogs.commands,
        &frame.measurements,
        &ui.linked,
        now,
    )
    .components(&ui.placed.pulled)
    .saying(ui.welcome.update.as_deref())
    .entrance(area.height, ui.welcome.resume.as_ref())
    .opening(ui.focus.intro().map(|intro| intro.seconds(now)));
    if let Some(key) = crate::component::welcome::request(&rows) {
        frame.pulls.push(key);
    }
    let lines = crate::component::welcome::draw(&rows);
    let lines = match rows.opening {
        Some(t) => crate::opening::frame(t, &lines),
        None => lines,
    };
    frame.render_widget(Paragraph::new(lines), Rect { height: 1, ..area });
}

pub(crate) fn welcome(state: &SessionState, width: usize) -> Line<'static> {
    line(&state.summary, branch(state), width)
}

fn branch(state: &SessionState) -> Option<&str> {
    let location = state.extensions.get("_bingo.context")?.get("location")?;
    (location["cwd"].as_str()? == state.summary.cwd)
        .then(|| location["branch"].as_str())
        .flatten()
}

fn line(summary: &SessionSummary, branch: Option<&str>, width: usize) -> Line<'static> {
    let cwd = paths::short(&summary.cwd, "", paths::home());
    let id = summary
        .id
        .as_str()
        .strip_prefix("ses_")
        .unwrap_or(summary.id.as_str());
    let session = format!("session {}  ", id.chars().take(4).collect::<String>());
    let title = summary
        .title
        .as_ref()
        .map(|title| format!("{title} · "))
        .unwrap_or_default();
    let branch = branch.map(|name| format!(" · {name}")).unwrap_or_default();
    let left = format!(" bingo{} {cwd}", theme::mark());
    let room = width.saturating_sub(left.width() + session.width() + 2);
    let title = if title.width() + branch.width() <= room {
        title
    } else {
        String::new()
    };
    let branch = if branch.width() <= room {
        branch
    } else {
        String::new()
    };
    let mut spans = vec![
        Span::styled(" bingo", theme::presence().patch(theme::bold())),
        Span::styled(theme::mark(), theme::presence()),
        Span::styled(format!(" {cwd}{branch}"), theme::dim()),
    ];
    let left_width: usize = spans.iter().map(Span::width).sum();
    let remaining = width.saturating_sub(session.width() + title.width() + 1);
    spans = status::clip(spans, remaining);
    spans.push(Span::raw(" ".repeat(width.saturating_sub(
        left_width.min(remaining) + title.width() + session.width(),
    ))));
    spans.push(Span::styled(title, theme::ash()));
    spans.push(Span::styled(session, theme::dim()));
    Line::from(status::clip(spans, width))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_narrow_header_drops_title_then_branch() {
        let mut state = crate::test_support::state();
        state.summary.cwd = "/work".into();
        state.summary.title = Some("label the total".into());
        let wide = line(&state.summary, Some("main"), 108).to_string();
        assert!(wide.contains("/work · main"));
        assert!(wide.contains("label the total · session"));
        let narrow = line(&state.summary, Some("main"), 38).to_string();
        assert!(narrow.contains("main"));
        assert!(!narrow.contains("label the total"));
        let tiny = line(&state.summary, Some("main"), 30).to_string();
        assert!(!tiny.contains("main"));
        assert!(tiny.contains("session"));
    }
}
