//! Pane declarations use terminal-independent lengths.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(try_from = "LengthWire", into = "LengthWire")]
pub enum Length {
    Cells(u16),
    Percent(u8),
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
enum LengthWire {
    Cells(u16),
    Percent(String),
}
impl TryFrom<LengthWire> for Length {
    type Error = String;
    fn try_from(value: LengthWire) -> Result<Self, Self::Error> {
        match value {
            LengthWire::Cells(cells) => Ok(Self::Cells(cells)),
            LengthWire::Percent(text) => text
                .strip_suffix('%')
                .and_then(|text| text.parse().ok())
                .map(Self::Percent)
                .ok_or_else(|| "a length is cells or a percentage".into()),
        }
    }
}
impl From<Length> for LengthWire {
    fn from(value: Length) -> Self {
        match value {
            Length::Cells(cells) => Self::Cells(cells),
            Length::Percent(percent) => Self::Percent(format!("{percent}%")),
        }
    }
}
impl Default for Length {
    fn default() -> Self {
        Self::Cells(0)
    }
}
impl Length {
    pub fn cells(self, area: u16) -> u16 {
        match self {
            Self::Cells(cells) => cells,
            Self::Percent(percent) => {
                (u32::from(area) * u32::from(percent) / 100).min(u32::from(u16::MAX)) as u16
            }
        }
    }
    pub fn published_ok(self) -> bool {
        !matches!(self, Self::Percent(percent) if percent > 100)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Span {
    pub min: Length,
    pub ideal: Length,
    pub max: Length,
}
impl Span {
    pub fn published_ok(self) -> bool {
        [self.min, self.ideal, self.max]
            .into_iter()
            .all(Length::published_ok)
            && match (self.min, self.max) {
                (Length::Cells(min), Length::Cells(max)) => min <= max,
                (Length::Percent(min), Length::Percent(max)) => min <= max,
                _ => true,
            }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct PaneSize {
    pub rows: Span,
    pub columns: Span,
}
impl Default for PaneSize {
    fn default() -> Self {
        Self::from_rows(8)
    }
}
impl PaneSize {
    pub fn from_rows(rows: u16) -> Self {
        Self {
            rows: Span {
                min: Length::Cells(2),
                ideal: Length::Cells(rows.max(2)),
                max: Length::Percent(100),
            },
            columns: Span {
                min: Length::Cells(1),
                ideal: Length::Cells(64),
                max: Length::Percent(100),
            },
        }
    }
    pub fn published_ok(self) -> bool {
        self.rows.published_ok() && self.columns.published_ok()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Anchor {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Rect {
    #[serde(default, skip_serializing_if = "is_top_left")]
    pub anchor: Anchor,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub x: Length,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub y: Length,
    pub width: Length,
    pub height: Length,
}
fn is_top_left(anchor: &Anchor) -> bool {
    *anchor == Anchor::TopLeft
}
fn is_zero(length: &Length) -> bool {
    *length == Length::Cells(0)
}
impl Rect {
    pub fn published_ok(self) -> bool {
        [self.x, self.y, self.width, self.height]
            .into_iter()
            .all(Length::published_ok)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(rename = "PanePlace")]
pub enum Place {
    #[default]
    Above,
    Side,
    Overlay,
    Rect(Rect),
}
impl Place {
    pub fn published_ok(self) -> bool {
        match self {
            Self::Rect(rect) => rect.published_ok(),
            _ => true,
        }
    }
}
pub fn above() -> Vec<Place> {
    vec![Place::Above]
}
