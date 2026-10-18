use super::*;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TextStyle {
    /// An Ember ground, filled to the available width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ground: Option<String>,
    /// A surface-clock animation; the author never needs to redraw it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motion: Option<Box<TextMotion>>,
    /// A colour name or `#rrggbb`; the surface may adjust it to its theme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Draw the text dimmed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dim_color: Option<bool>,
    /// Draw the text bold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    /// Draw the text in italics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    /// Underline the text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    /// Strike the text through.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strikethrough: Option<bool>,
    /// Swap the foreground and background colours.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inverse: Option<bool>,
    /// `wrap` breaks long lines at the width; `truncate` cuts them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BoxStyle {
    /// An Ember ground, filled across the box including padding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ground: Option<String>,
    /// A question owner's editing state, on the root of a drawn question;
    /// the choices' labels and the answer stay the host's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<crate::component::QuestionFocus>,
    /// The focused site declares chrome separately from its body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hints: Option<crate::component::SiteHints>,
    /// A handler for a key pressed while this box's pane has focus; return `{handled: false}` to give the key back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_key: Option<Action>,
    /// A stable identity among siblings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// `row` or `column` (the default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flex_direction: Option<Direction>,
    /// How much of the spare room this box takes; `0` takes none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flex_grow: Option<u16>,
    /// `wrap` lets children flow onto another line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flex_wrap: Option<Wrap>,
    /// Cross-axis alignment: `flex-start`, `center`, `flex-end` or `stretch`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align_items: Option<Align>,
    /// Main-axis distribution, such as `flex-start` or `space-between`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub justify_content: Option<Justify>,
    /// Cells between children.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap: Option<u16>,
    /// The width in cells.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u16>,
    /// The height in rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u16>,
    #[serde(flatten)]
    pub spacing: Spacing,
    /// Any value but `none` draws a border.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_style: Option<String>,
    /// The border colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_color: Option<String>,
    /// `none` hides the box and its controls.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ButtonProps {
    /// A stable identity among siblings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// The button's text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// One letter: pressed with Alt in a band, or alone in a focused pane, it presses the button.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<char>,
    /// Draw the button dimmed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dim_color: Option<bool>,
    /// `primary` draws it bold, `danger` in the alarm colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    /// The handler run when the button is pressed: a mod passes a function.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_press: Option<Action>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InputProps {
    /// A stable identity among siblings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Text before the field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Text shown while the field is empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    /// The text in the field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// The handler run as the text changes: a mod passes a function.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_input: Option<Action>,
    /// The handler run when the person presses Enter: a mod passes a function.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_submit: Option<Action>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SelectProps {
    /// A stable identity among siblings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Text before the choice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The chosen option's `value`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// The choices, each `{label, value}`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<SelectOption>>,
    /// The handler run with the chosen value: a mod passes a function.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_select: Option<Action>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SelectOption {
    /// The text shown.
    pub label: String,
    /// The value chosen.
    pub value: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Direction {
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Wrap {
    Wrap,
    Nowrap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Align {
    FlexStart,
    Center,
    FlexEnd,
    Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Justify {
    FlexStart,
    Center,
    FlexEnd,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Spacing {
    /// Cells outside the box on every side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin: Option<u16>,
    /// Cells outside the box on the left and right.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_x: Option<u16>,
    /// Rows outside the box above and below.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_y: Option<u16>,
    /// Cells inside the box on every side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding: Option<u16>,
    /// Cells inside the box on the left and right.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_x: Option<u16>,
    /// Rows inside the box above and below.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_y: Option<u16>,
}

/// Declarative motion on the surface clock, with a reduced-motion still form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TextMotion {
    Breathe {
        with: String,
        #[serde(rename = "periodMs")]
        period_ms: u64,
        depth: f64,
    },
    Spin,
    Fade {
        from: String,
        #[serde(rename = "atMs")]
        at_ms: i64,
        ms: u64,
    },
}
