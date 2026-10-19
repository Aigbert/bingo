//! A pane's first fitting place, without drawing or state mutation.
use super::Pane;
use bingo_sdk::slot::Place;
use ratatui::layout::Rect;
use std::collections::BTreeMap;

pub const TRANSCRIPT_MIN_COLUMNS: u16 = 60;
pub const TRANSCRIPT_MIN_ROWS: u16 = 8;
pub const FLOATING_MAX: usize = 3;

#[derive(Clone, Debug)]
pub struct Seated {
    pub key: String,
    pub place: Place,
    pub area: Rect,
    pub body_rows: Option<u16>,
}
impl Seated {
    pub fn docked(&self) -> bool {
        self.place == Place::Dock
    }
}
#[derive(Debug, Default)]
pub struct Seating {
    pub area: Rect,
    pub transcript: Rect,
    pub panes: Vec<Seated>,
    pub collapsed: Vec<String>,
}
pub fn seat(area: Rect, wanted: &[Pane], moved: &BTreeMap<String, Place>) -> Seating {
    let mut seating = Seating {
        area,
        transcript: area,
        ..Seating::default()
    };
    for pane in wanted {
        let places = moved.get(&pane.key).into_iter().chain(&pane.place);
        let choice = places.copied().find_map(|place| {
            fits(&seating, pane, place, wanted).map(|area| Seated {
                body_rows: None,
                key: pane.key.clone(),
                place,
                area,
            })
        });
        match choice {
            Some(pane) => reserve(&mut seating, pane),
            None => seating.collapsed.push(pane.key.clone()),
        }
    }
    seating
}

fn reserve(seating: &mut Seating, pane: Seated) {
    match pane.place {
        Place::Above => {
            seating.transcript.height -= pane.area.height;
            for other in &mut seating.panes {
                if other.place == Place::Side {
                    other.area.height = seating.transcript.height;
                }
            }
        }
        Place::Side => seating.transcript.width -= pane.area.width,
        _ => {}
    }
    seating.panes.push(pane);
}

fn fits(seating: &Seating, pane: &Pane, place: Place, wanted: &[Pane]) -> Option<Rect> {
    let full = seating.area;
    let rest = seating.transcript;
    let rows = bounds(pane.size.rows, full.height)?;
    let columns = bounds(pane.size.columns, full.width)?;
    match place {
        Place::Above if !occupied(seating, Place::Above) => {
            if rest.width < TRANSCRIPT_MIN_COLUMNS || full.width < columns.0 {
                return None;
            }
            let side_min = seating
                .panes
                .iter()
                .filter(|seat| seat.place == Place::Side)
                .filter_map(|seat| wanted.iter().find(|pane| pane.key == seat.key))
                .map(|pane| pane.size.rows.min.cells(full.height))
                .max()
                .unwrap_or_default();
            let height = dimension(
                rows,
                rest.height
                    .saturating_sub(TRANSCRIPT_MIN_ROWS.max(side_min)),
            )?;
            // An above pane spans the whole area, including beneath a side pane.
            Some(Rect::new(
                full.x,
                rest.bottom() - height,
                full.width,
                height,
            ))
        }
        Place::Side if !occupied(seating, Place::Side) => {
            if rest.height < TRANSCRIPT_MIN_ROWS || rest.height < rows.0 {
                return None;
            }
            let width = dimension(columns, rest.width.saturating_sub(TRANSCRIPT_MIN_COLUMNS))?;
            Some(Rect::new(rest.right() - width, rest.y, width, rest.height))
        }
        Place::Dock if !occupied(seating, Place::Dock) => Some(full),
        Place::Overlay | Place::Rect(_) if floating(seating) < FLOATING_MAX => {
            let rect = match place {
                Place::Rect(rect) => floating_rect(full, rect),
                _ => full,
            };
            (rect.width >= columns.0 && rect.height >= rows.0).then_some(rect)
        }
        _ => None,
    }
}
fn occupied(seating: &Seating, place: Place) -> bool {
    seating.panes.iter().any(|pane| pane.place == place)
}
fn floating(seating: &Seating) -> usize {
    seating
        .panes
        .iter()
        .filter(|pane| matches!(pane.place, Place::Overlay | Place::Rect(_)))
        .count()
}
fn bounds(span: bingo_sdk::slot::Span, area: u16) -> Option<(u16, u16, u16)> {
    let (min, ideal, max) = (
        span.min.cells(area),
        span.ideal.cells(area),
        span.max.cells(area),
    );
    (span.published_ok() && min <= max).then_some((min, ideal, max))
}
fn dimension((min, ideal, max): (u16, u16, u16), available: u16) -> Option<u16> {
    (min <= available).then(|| ideal.clamp(min, max).min(available))
}
fn floating_rect(area: Rect, rect: bingo_sdk::slot::Rect) -> Rect {
    use bingo_sdk::slot::Anchor;
    let width = rect.width.cells(area.width);
    let height = rect.height.cells(area.height);
    let x = rect.x.cells(area.width);
    let y = rect.y.cells(area.height);
    let left = match rect.anchor {
        Anchor::TopLeft | Anchor::BottomLeft => x,
        Anchor::TopRight | Anchor::BottomRight => {
            area.width.saturating_sub(width).saturating_sub(x)
        }
        Anchor::Center => (area.width.saturating_sub(width) / 2).saturating_add(x),
    };
    let top = match rect.anchor {
        Anchor::TopLeft | Anchor::TopRight => y,
        Anchor::BottomLeft | Anchor::BottomRight => {
            area.height.saturating_sub(height).saturating_sub(y)
        }
        Anchor::Center => (area.height.saturating_sub(height) / 2).saturating_add(y),
    };
    Rect::new(
        area.x + left.min(area.width.saturating_sub(width)),
        area.y + top.min(area.height.saturating_sub(height)),
        width.min(area.width),
        height.min(area.height),
    )
}

#[cfg(test)]
mod tests;
