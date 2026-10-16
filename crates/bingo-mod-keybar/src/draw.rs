use bingo_sdk::{
    View,
    component::KeyBarProps,
    view::{TextChild, TextStyle},
};
use unicode_width::UnicodeWidthStr;

/// Keep whole pairs from the right. A toast owns the row until its moment ends.
pub fn tree(props: &KeyBarProps, width: usize) -> View {
    let room = width.saturating_sub(3);
    if let Some(toast) = &props.toast {
        let color = match toast.level {
            bingo_sdk::Level::Error => "red",
            bingo_sdk::Level::Warn => "yellow",
            _ => "default",
        };
        let mut children = vec![node(
            &format!(" {}", toast.text.lines().collect::<Vec<_>>().join(" · ")),
            Some(color),
            false,
            false,
        )];
        if let Some(about) = &toast.about {
            children.push(node(&format!(" · {about}"), None, false, true));
        }
        return text(children, None, false, false);
    }
    let (pairs, used) = props.fitting(room, UnicodeWidthStr::width);
    let mut children = vec![TextChild::String(
        " ".repeat(width.saturating_sub(used + 2)),
    )];
    for (index, pair) in pairs.iter().enumerate() {
        if index > 0 {
            children.push(node(" · ", Some("ash"), false, false));
        }
        children.push(node(&pair.keys, Some("glow"), true, false));
        children.push(node(
            &format!(
                "{}{}",
                if pair.keys.is_empty() { "" } else { " " },
                pair.action
            ),
            None,
            pair.emphasis,
            true,
        ));
    }
    text(children, None, false, false)
}
fn node(value: &str, color: Option<&str>, bold: bool, dim: bool) -> TextChild {
    TextChild::Node(Box::new(text(
        vec![TextChild::String(value.into())],
        color,
        bold,
        dim,
    )))
}
fn text(children: Vec<TextChild>, color: Option<&str>, bold: bool, dim: bool) -> View {
    View::Text {
        children,
        style: TextStyle {
            color: color.map(str::to_owned),
            bold: Some(bold),
            dim_color: Some(dim),
            wrap: Some("truncate".into()),
            ..Default::default()
        },
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use bingo_sdk::component::KeyPair;
    #[test]
    fn narrow_widths_drop_whole_pairs_from_the_left_and_toast_wins() {
        let mut props = KeyBarProps {
            pairs: vec![KeyPair::new("↑↓", "pick"), KeyPair::new("esc", "close")],
            toast: None,
        };
        assert_eq!(tree(&props, 12).fold().trim(), "esc close");
        assert!(tree(&props, 3).fold().trim().is_empty());
        props.toast = Some(bingo_sdk::component::KeyToast {
            about: None,
            text: "saved".into(),
            level: bingo_sdk::Level::Info,
        });
        assert_eq!(tree(&props, 80).fold().trim(), "saved");
    }
}
