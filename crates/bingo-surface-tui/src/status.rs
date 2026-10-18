//! Shared clipping and session facts for component sites.
use crate::theme;
use bingo_sdk::SessionState;
use ratatui::text::Span;
use unicode_width::UnicodeWidthStr;

pub(crate) fn session_facts(state: &SessionState) -> bingo_sdk::component::SessionStatusProps {
    bingo_sdk::component::SessionStatusProps {
        title: state.summary.title.clone(),
        cwd: state.summary.cwd.clone(),
        provider: state.summary.provider.clone(),
        model: state.summary.model.clone(),
        model_scope: state.summary.model_scope,
        effort: state.summary.effort,
        context: state.context,
        usage: state.summary.usage,
    }
}

pub fn clip(spans: Vec<Span<'static>>, max: usize) -> Vec<Span<'static>> {
    if cells(&spans) <= max {
        return spans;
    }
    if max <= 1 {
        return Vec::new();
    }
    let mut out: Vec<Span<'static>> = Vec::new();
    let mut used = 0;
    for span in spans {
        let room = max - 1 - used;
        let width = span.content.width();
        if width <= room {
            used += width;
            out.push(span);
            continue;
        }
        out.push(cut(&span, room));
        break;
    }
    out.push(Span::styled("…", theme::dim()));
    out
}

fn cut(span: &Span<'static>, room: usize) -> Span<'static> {
    let mut text = String::new();
    let mut used = 0;
    for c in span.content.chars() {
        let width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if used + width > room {
            break;
        }
        text.push(c);
        used += width;
    }
    Span::styled(text, span.style)
}

fn cells(spans: &[Span<'static>]) -> usize {
    spans.iter().map(Span::width).sum()
}
#[cfg(test)]
pub fn line(
    tree: &crate::tree::Tree,
    ui: &crate::ui::Ui,
    width: usize,
    now: crate::clock::Now,
) -> ratatui::text::Line<'static> {
    let mut frame = crate::layout::Layout::default();
    crate::footer::draw(tree, ui, &mut frame, width, now)
        .lines
        .into_iter()
        .next()
        .unwrap_or_default()
}
