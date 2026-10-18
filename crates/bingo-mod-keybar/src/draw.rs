use bingo_sdk::{
    View,
    component::KeyBarProps,
    view::{TextChild, TextStyle},
};
use unicode_width::UnicodeWidthStr;

/// Keep whole pairs from the right; the host draws toasts beside them.
pub fn tree(props: &KeyBarProps, width: usize) -> View {
    let (pairs, _) = props.fitting(width, UnicodeWidthStr::width);
    let mut children = Vec::new();
    for (index, pair) in pairs.iter().enumerate() {
        if index > 0 {
            children.push(node("   ", None, false, false));
        }
        children.push(node(&pair.keys, Some("muted"), false, false));
        children.push(node(
            &format!(
                "{}{}",
                if pair.keys.is_empty() { "" } else { " " },
                pair.action
            ),
            Some("dim"),
            pair.emphasis,
            false,
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
    fn narrow_widths_drop_whole_pairs_from_the_left() {
        let props = KeyBarProps {
            pairs: vec![KeyPair::new("↑↓", "pick"), KeyPair::new("esc", "close")],
        };
        assert_eq!(tree(&props, 12).fold().trim(), "esc close");
        assert!(tree(&props, 3).fold().trim().is_empty());
    }
}
