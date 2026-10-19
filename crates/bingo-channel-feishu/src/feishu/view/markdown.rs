use bingo_sdk::{
    View,
    view::{TextChild, TextStyle},
};
use serde_json::{Value, json};

pub(super) fn element(content: &str) -> Value {
    json!({"tag":"markdown","content":content})
}

pub(super) fn escape(text: &str) -> String {
    let mut escaped = String::new();
    for ch in text.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '\\' | '`' | '*' | '_' | '~' | '[' | ']' | '(' | ')' | '#' | '!' | '|' | '-' | '+' => {
                escaped.push('\\');
                escaped.push(ch);
            }
            _ => escaped.push(ch),
        }
    }
    escaped
}

pub(super) fn view(view: &View) -> String {
    match view {
        View::Text { children, style } => styled(
            children
                .iter()
                .map(|child| match child {
                    TextChild::String(text) => escape(text),
                    TextChild::Node(node) => self::view(node),
                })
                .collect(),
            style,
        ),
        View::Code {
            source, language, ..
        } => code(source, language.as_deref()),
        View::Link { href, label } => link(href, label.as_deref()),
        _ => escape(&view.fold()),
    }
}

fn styled(mut text: String, style: &TextStyle) -> String {
    for (on, marker) in [
        (style.bold, "**"),
        (style.italic, "*"),
        (style.strikethrough, "~~"),
    ] {
        if on == Some(true) {
            text = format!("{marker}{text}{marker}");
        }
    }
    if let Some(color) = style.color.as_deref().filter(|color| {
        ["red", "blue", "green", "grey", "orange", "purple", "yellow"].contains(color)
    }) {
        text = format!("<font color='{color}'>{text}</font>");
    }
    text
}

fn code(source: &str, language: Option<&str>) -> String {
    let longest = source
        .split(|c| c != '`')
        .map(str::len)
        .max()
        .unwrap_or_default();
    let fence = "`".repeat(longest.max(2) + 1);
    let language = language
        .filter(|s| s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
        .unwrap_or("");
    let source = source
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    format!("{fence}{language}\n{source}\n{fence}")
}

fn link(href: &str, label: Option<&str>) -> String {
    let label = escape(label.unwrap_or(href));
    if !href.starts_with("https://") && !href.starts_with("http://") {
        return label;
    }
    let href = href
        .chars()
        .map(|ch| match ch {
            '(' => "%28".into(),
            ')' => "%29".into(),
            '[' => "%5B".into(),
            ']' => "%5D".into(),
            '<' => "%3C".into(),
            '>' => "%3E".into(),
            '\\' => "%5C".into(),
            '"' => "%22".into(),
            c if c.is_whitespace() || c.is_control() => format!("%{:02X}", c as u32),
            c => c.to_string(),
        })
        .collect::<String>();
    format!("[{label}]({href})")
}
