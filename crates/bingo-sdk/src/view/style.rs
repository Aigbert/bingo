use super::*;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TextStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dim_color: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strikethrough: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inverse: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrap: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BoxStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flex_direction: Option<Direction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flex_grow: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flex_wrap: Option<Wrap>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align_items: Option<Align>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub justify_content: Option<Justify>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u16>,
    #[serde(flatten)]
    pub spacing: Spacing,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ButtonProps {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<char>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dim_color: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_press: Option<Action>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InputProps {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_input: Option<Action>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_submit: Option<Action>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SelectProps {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<SelectOption>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_select: Option<Action>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SelectOption {
    pub label: String,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_x: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub margin_y: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_x: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding_y: Option<u16>,
}
