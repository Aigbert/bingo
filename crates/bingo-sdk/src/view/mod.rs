//! UI as data (ADR-0013): the one vocabulary a plugin describes a screen
//! with. A surface draws each node once for everyone; what it cannot draw
//! it shows as `text()`, the one fold every node ships with.
//!
//! A word this vocabulary does not have is [`View::Custom`] (ADR-0038): a
//! plugin puts up an element the sdk has no name for, and an unknown `kind`
//! off the wire lands there too, so a newer speaker never breaks an older
//! reader — it is read as its fold until the reader learns the word.

use crate::host::Action;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
mod fold;
pub use fold::text_children;
mod read;
pub mod table;
mod validate;
mod walk;
pub use validate::{answer_ok, published_ok, sealed_ok};
mod style;
pub use style::*;
#[cfg(test)]
mod contract;
#[cfg(test)]
mod tests;

/// What a person is told when a gesture names a control its drawing no longer
/// has: the host, a surface and a channel all say it in these words.
pub const STALE_CONTROL: &str = "This control belongs to an earlier drawing; try the current one.";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", remote = "Self")]
pub enum View {
    /// A run of text: strings and nested `Text` nodes, which keep their own style.
    #[serde(rename = "Text")]
    Text {
        /// Strings and nested `Text` nodes, drawn in order.
        children: Vec<TextChild>,
        #[serde(flatten)]
        style: TextStyle,
    },
    /// Markdown text the surface renders.
    #[serde(rename = "Markdown")]
    Markdown {
        /// The Markdown source.
        text: String,
        /// Draw it dimmed.
        #[serde(default, rename = "dimColor", skip_serializing_if = "is_false")]
        dim_color: bool,
    },
    /// A block of source code.
    #[serde(rename = "Code")]
    Code {
        /// The language name for highlighting, such as `rust`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
        /// The code itself.
        source: String,
    },
    /// A diff drawn with added and removed lines marked.
    Diff {
        /// The diff in unified format.
        unified: String,
    },
    /// A bulleted list of lines.
    List {
        /// One string per item.
        items: Vec<String>,
    },
    /// A table of strings.
    Table {
        /// The column headings.
        headers: Vec<String>,
        /// One array of cells per row, in column order.
        rows: Vec<Vec<String>>,
    },
    /// Aligned label and value pairs.
    KeyValue {
        /// `[label, value]` pairs, in order.
        rows: Vec<(String, String)>,
    },
    /// A progress bar, or a count when there is no total.
    Progress {
        /// How much is done.
        value: u64,
        /// The whole; without it the surface shows a count.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        total: Option<u64>,
        /// Text drawn beside the bar.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
    /// A short label coloured by its tone.
    Badge {
        /// The label.
        text: String,
        /// `neutral`, `good`, `bad` or `attention`; the surface picks the colour.
        #[serde(default)]
        tone: Tone,
    },
    /// An indented tree of labelled nodes.
    Tree {
        /// The top-level nodes.
        nodes: Vec<TreeNode>,
    },
    /// A flex container: lays its children out in a row or a column.
    #[serde(rename = "Box")]
    Box {
        /// The views inside, in order.
        children: Vec<View>,
        #[serde(flatten)]
        layout: Box<BoxStyle>,
    },
    /// One view under a title.
    Panel {
        /// The heading.
        title: String,
        /// The view inside.
        child: Box<View>,
    },
    /// A row of buttons that each run a named action.
    Actions {
        /// The buttons, in order.
        items: Vec<ActionItem>,
    },
    /// A pressable control; a mod's `onPress` runs when it is pressed.
    #[serde(rename = "Button")]
    Button {
        /// The button's text, when `label` is not given.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        children: Vec<TextChild>,
        #[serde(flatten)]
        button: Box<ButtonProps>,
    },
    /// A one-line text field.
    #[serde(rename = "Input")]
    Input {
        #[serde(flatten)]
        input: Box<InputProps>,
    },
    /// A choice among options.
    #[serde(rename = "Select")]
    Select {
        #[serde(flatten)]
        select: Box<SelectProps>,
    },
    /// A link the person can open.
    #[serde(rename = "Link")]
    Link {
        /// The URL.
        href: String,
        /// The text shown; the URL when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
    /// A picture; surfaces that cannot draw it show `alt`.
    #[serde(rename = "Image")]
    Image {
        /// A path or a `data:` URL.
        source: String,
        /// Text shown where the image cannot be.
        alt: String,
        /// The width in terminal columns.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        columns: Option<u16>,
        /// The height in terminal rows.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rows: Option<u16>,
    },
    /// A grid of character cells, drawn by terminals only.
    #[serde(rename = "Raster")]
    Raster {
        /// The width in cells.
        columns: u16,
        /// The height in cells.
        rows: u16,
        /// The cells, row by row.
        cells: Vec<String>,
    },
    /// An SVG drawing, for surfaces that draw vectors; others show `alt`.
    #[serde(rename = "Svg")]
    Svg {
        /// The SVG markup.
        source: String,
        /// Text shown where the drawing cannot be.
        alt: String,
        /// The width in pixels.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<u16>,
        /// The height in pixels.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<u16>,
    },
    /// bingo's own drawing of a component, placed inside a mod's view.
    #[serde(rename = "Engine")]
    Engine {
        /// The component bingo draws, such as `ToolUse`.
        component: crate::render::Component,
        /// That component's props, as its `ui.render` event carried them.
        props: Value,
    },
    /// An element bingo has no name for; surfaces that do not know it show `fold`.
    Custom {
        /// The element's own name.
        #[serde(rename = "customKind")]
        kind: String,
        /// Whatever the element needs.
        data: Value,
        /// The plain text drawn where the element is unknown.
        fold: String,
    },
}

/// A text child preserves the style of its nested Text node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum TextChild {
    String(String),
    Node(Box<View>),
}

pub const NARROW: usize = 60;
pub const COLUMN_GAP: usize = 2;
const CUSTOM: &str = "custom";
const KNOWN: &[&str] = &[
    "Text", "Markdown", "Code", "diff", "list", "table", "keyValue", "progress", "badge", "tree",
    "Box", "panel", "actions", "Button", "Input", "Select", "Link", "Image", "Raster", "Svg",
    "Engine", CUSTOM,
];
fn is_false(value: &bool) -> bool {
    !value
}

/// The one styling hook a plugin has; the surface owns the colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Tone {
    #[default]
    Neutral,
    Good,
    Bad,
    /// Wants a person: a surface makes it move.
    Attention,
}

impl Tone {
    pub fn is_neutral(&self) -> bool {
        *self == Self::Neutral
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TreeNode {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub badge: Option<String>,
    #[serde(default)]
    pub tone: Tone,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<TreeNode>,
}

/// A button: the surface fires `Input::Action` with `action` (ADR-0008 §1).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActionItem {
    /// A one-line explanation supplied by the action owner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meaning: Option<String>,
    pub label: String,
    pub action: Action,
    /// A single-key hint; the surface may use another key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<char>,
}
