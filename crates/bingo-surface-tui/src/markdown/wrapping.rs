use ratatui::text::{Line, Span};

pub(super) fn wrap(
    line: Line<'static>,
    width: usize,
    margin: &[Span<'static>],
) -> Vec<Line<'static>> {
    let indent: usize = margin.iter().map(Span::width).sum();
    if indent == 0 {
        return crate::wrap::wrap_all(&[line], width);
    }
    let mut spans = line.spans.into_iter();
    let mut prefix = Vec::new();
    let mut used = 0;
    while used < indent {
        let Some(span) = spans.next() else {
            break;
        };
        used += span.width();
        prefix.push(span);
    }
    crate::wrap::wrap_all(
        &[Line::from(spans.collect::<Vec<_>>())],
        width.saturating_sub(indent).max(1),
    )
    .into_iter()
    .enumerate()
    .map(|(at, mut line)| {
        let lead = if at == 0 {
            prefix.clone()
        } else {
            margin.to_vec()
        };
        line.spans.splice(0..0, lead);
        line
    })
    .collect()
}
