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
pub use validate::{answer_ok, published_ok};
mod style;
pub use style::*;
#[cfg(test)]
mod contract;
#[cfg(test)]
mod tests;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", remote = "Self")]
pub enum View {
    #[serde(rename = "Text")]
    Text {
        children: Vec<TextChild>,
        #[serde(flatten)]
        style: TextStyle,
    },
    #[serde(rename = "Markdown")]
    Markdown {
        text: String,
        #[serde(default, rename = "dimColor", skip_serializing_if = "is_false")]
        dim_color: bool,
    },
    #[serde(rename = "Code")]
    Code {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
        source: String,
    },
    Diff {
        unified: String,
    },
    List {
        items: Vec<String>,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    KeyValue {
        rows: Vec<(String, String)>,
    },
    Progress {
        value: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        total: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
    Badge {
        text: String,
        #[serde(default)]
        tone: Tone,
    },
    Tree {
        nodes: Vec<TreeNode>,
    },
    #[serde(rename = "Box")]
    Box {
        children: Vec<View>,
        #[serde(flatten)]
        layout: BoxStyle,
    },
    Panel {
        title: String,
        child: Box<View>,
    },
    Actions {
        items: Vec<ActionItem>,
    },
    #[serde(rename = "Button")]
    Button {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        children: Vec<TextChild>,
        #[serde(flatten)]
        button: ButtonProps,
    },
    #[serde(rename = "Input")]
    Input {
        #[serde(flatten)]
        input: InputProps,
    },
    #[serde(rename = "Select")]
    Select {
        #[serde(flatten)]
        select: SelectProps,
    },
    #[serde(rename = "Link")]
    Link {
        href: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
    #[serde(rename = "Image")]
    Image {
        source: String,
        alt: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        columns: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rows: Option<u16>,
    },
    #[serde(rename = "Raster")]
    Raster {
        columns: u16,
        rows: u16,
        cells: Vec<String>,
    },
    #[serde(rename = "Svg")]
    Svg {
        source: String,
        alt: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        width: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        height: Option<u16>,
    },
    #[serde(rename = "Engine")]
    Engine {
        component: crate::render::Component,
        props: Value,
    },
    Custom {
        #[serde(rename = "customKind")]
        kind: String,
        data: Value,
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
pub const READ_ONLY: &[&str] = &["stack", "columns"];
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
    pub label: String,
    pub action: Action,
    /// A single-key hint; the surface may use another key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<char>,
}
