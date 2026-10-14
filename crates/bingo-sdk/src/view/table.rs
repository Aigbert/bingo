//! The element subsets a surface promises to draw.
use super::*;
use crate::render::SurfaceName;
#[derive(Clone, Copy, Debug, Serialize)]
pub struct Element {
    pub kind: &'static str,
    pub props: &'static [&'static str],
}
pub const ELEMENTS: &[Element] = &[
    Element {
        kind: "Box",
        props: &[
            "key",
            "flexDirection",
            "flexGrow",
            "flexWrap",
            "alignItems",
            "justifyContent",
            "gap",
            "width",
            "height",
            "margin",
            "marginX",
            "marginY",
            "padding",
            "paddingX",
            "paddingY",
            "borderStyle",
            "borderColor",
            "display",
            "children",
        ],
    },
    Element {
        kind: "Text",
        props: &[
            "children",
            "color",
            "dimColor",
            "bold",
            "italic",
            "underline",
            "strikethrough",
            "inverse",
            "wrap",
        ],
    },
    Element {
        kind: "Button",
        props: &[
            "children", "key", "label", "hotkey", "dimColor", "variant", "onPress",
        ],
    },
    Element {
        kind: "Input",
        props: &[
            "key",
            "label",
            "placeholder",
            "value",
            "onInput",
            "onSubmit",
        ],
    },
    Element {
        kind: "Select",
        props: &["key", "label", "options", "value", "onSelect"],
    },
    Element {
        kind: "Link",
        props: &["href", "label"],
    },
    Element {
        kind: "Image",
        props: &["source", "alt", "columns", "rows"],
    },
    Element {
        kind: "Raster",
        props: &["columns", "rows", "cells"],
    },
    Element {
        kind: "Svg",
        props: &["source", "alt", "width", "height"],
    },
    Element {
        kind: "Code",
        props: &["source", "language"],
    },
    Element {
        kind: "Markdown",
        props: &["text", "dimColor"],
    },
    Element {
        kind: "Engine",
        props: &["component", "props"],
    },
    Element {
        kind: "diff",
        props: &["unified"],
    },
    Element {
        kind: "list",
        props: &["items"],
    },
    Element {
        kind: "table",
        props: &["headers", "rows"],
    },
    Element {
        kind: "keyValue",
        props: &["rows"],
    },
    Element {
        kind: "progress",
        props: &["value", "total", "label"],
    },
    Element {
        kind: "badge",
        props: &["text", "tone"],
    },
    Element {
        kind: "tree",
        props: &["nodes"],
    },
    Element {
        kind: "panel",
        props: &["title", "child"],
    },
    Element {
        kind: "actions",
        props: &["items"],
    },
    Element {
        kind: "custom",
        props: &["customKind", "data", "fold"],
    },
];
pub const TERMINAL: SurfaceName = SurfaceName::Terminal;
pub const DESKTOP: SurfaceName = SurfaceName::Desktop;
pub const CHAT: SurfaceName = SurfaceName::Chat;
pub const TEXT: SurfaceName = SurfaceName::Text;
pub fn elements(surface: SurfaceName) -> Vec<Element> {
    ELEMENTS
        .iter()
        .copied()
        .filter(|element| match surface {
            SurfaceName::Terminal => element.kind != "Svg",
            SurfaceName::Desktop => element.kind != "Raster",
            SurfaceName::Text => false,
            SurfaceName::Chat => matches!(
                element.kind,
                "Box"
                    | "Text"
                    | "Code"
                    | "Link"
                    | "Image"
                    | "Button"
                    | "Select"
                    | "Input"
                    | "Engine"
            ),
        })
        .collect()
}
pub fn outside(surface: SurfaceName, view: &View) -> Vec<String> {
    let elements = elements(surface);
    super::walk::nodes(view)
        .into_iter()
        .filter_map(|(path, view)| {
            (!elements.iter().any(|element| element.kind == kind(view))).then_some(path)
        })
        .collect()
}
pub fn kind(view: &View) -> &'static str {
    match view {
        View::Box { .. } => "Box",
        View::Text { .. } => "Text",
        View::Button { .. } => "Button",
        View::Input { .. } => "Input",
        View::Select { .. } => "Select",
        View::Link { .. } => "Link",
        View::Image { .. } => "Image",
        View::Raster { .. } => "Raster",
        View::Svg { .. } => "Svg",
        View::Code { .. } => "Code",
        View::Markdown { .. } => "Markdown",
        View::Engine { .. } => "Engine",
        View::Diff { .. } => "diff",
        View::List { .. } => "list",
        View::Table { .. } => "table",
        View::KeyValue { .. } => "keyValue",
        View::Progress { .. } => "progress",
        View::Badge { .. } => "badge",
        View::Tree { .. } => "tree",
        View::Panel { .. } => "panel",
        View::Actions { .. } => "actions",
        View::Custom { .. } => "custom",
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chat_names_every_native_element_and_folds_the_rest() {
        let native = [
            "Box", "Text", "Code", "Link", "Image", "Button", "Select", "Input", "Engine",
        ];
        for element in ELEMENTS {
            assert_eq!(
                elements(CHAT).iter().any(|e| e.kind == element.kind),
                native.contains(&element.kind),
                "{}",
                element.kind
            );
        }
        assert!(elements(TEXT).is_empty());
        assert_eq!(serde_json::to_value(CHAT).unwrap(), "chat");
    }
    #[test]
    fn tables_pin_props_and_report_nested_folds() {
        let svg = View::Svg {
            source: "<svg/>".into(),
            alt: "circle".into(),
            width: None,
            height: None,
        };
        let view = View::column(vec![svg]);
        assert_eq!(outside(TERMINAL, &view), ["/children/0"]);
        assert!(outside(DESKTOP, &view).is_empty());
        assert_eq!(outside(TEXT, &view), ["", "/children/0"]);
        assert!(!elements(TERMINAL)[0].props.contains(&"backgroundColor"));
        let schema = serde_json::to_value(schemars::schema_for!(View)).unwrap();
        for variant in schema["oneOf"].as_array().unwrap() {
            let kind = variant["properties"]["kind"]["const"].as_str().unwrap();
            assert!(
                ELEMENTS.iter().any(|element| element.kind == kind),
                "{kind}"
            );
        }
    }
}
