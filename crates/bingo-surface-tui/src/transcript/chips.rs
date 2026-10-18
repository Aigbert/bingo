//! Chips are a view of the image parts that travelled with the person's words.
use crate::{pictures, theme};
use bingo_sdk::{ContentPart, Item, ItemBody};
use ratatui::text::{Line, Span};

pub(super) fn text(item: &Item, text: &str) -> String {
    let count = count(item);
    let mut out = String::new();
    let mut from = 0;
    for (range, _) in pictures::spans(text).into_iter().take(count) {
        out.push_str(&text[from..range.start]);
        from = range.end;
    }
    out.push_str(&text[from..]);
    out.trim_start().to_owned()
}

pub(super) fn prepend(item: &Item, line: &mut Line<'static>) {
    let mut spans = Vec::new();
    let numbers = match &item.body {
        ItemBody::User { parts, .. } => pictures::tokens(&super::said::said(parts)),
        _ => Vec::new(),
    };
    for index in 0..count(item) {
        let n = numbers.get(index).copied().unwrap_or(index as u32 + 1);
        spans.push(Span::styled(
            pictures::placeholder(n),
            theme::foreground("plan").unwrap_or_default(),
        ));
        spans.push(Span::raw(" "));
    }
    spans.append(&mut line.spans);
    line.spans = spans;
}

fn count(item: &Item) -> usize {
    match &item.body {
        ItemBody::User { parts, .. } => parts
            .iter()
            .filter(|part| matches!(part, ContentPart::Image { .. }))
            .count(),
        _ => 0,
    }
}
