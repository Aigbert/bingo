//! Capture contracts shared by M192–M200. Coordinates are terminal cells.
#![allow(dead_code)]
use crate::{clock::Now, theme};
use ratatui::{buffer::Buffer, layout::Rect};
use serde::Deserialize;
use std::path::PathBuf;
pub mod acting;
mod compare;
mod parse;
pub mod story;

/// The private clock origin in prototype/p8-page.js (snapshot and capture).
pub const CLOCK_ORIGIN_MS: i64 = 10_000_000;

#[derive(Clone, Copy, Debug)]
pub enum Band {
    Stream,
    Dock,
    Footer,
    Frame,
}

#[derive(Debug, Deserialize)]
pub struct Entry {
    pub file: String,
    pub scene: String,
    #[serde(default, rename = "grp")]
    pub group: String,
    #[serde(rename = "t")]
    pub moment_ms: i64,
    pub size: String,
    #[serde(default)]
    pub flags: serde_json::Value,
    #[serde(default)]
    pub after: Vec<serde_json::Value>,
}

#[derive(Debug)]
struct Run {
    start: u16,
    end: u16,
    token: String,
}
#[derive(Debug, Default)]
struct Row {
    cells: Vec<String>,
    fg: Vec<Run>,
    bg: Vec<Run>,
}

pub struct Capture {
    pub entry: Entry,
    rows: Vec<Row>,
    spinner_cells: Vec<(u16, u16)>,
}

impl Capture {
    pub fn load(name: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/design/ember/scenes");
        let entries: Vec<Entry> =
            serde_json::from_str(&std::fs::read_to_string(root.join("index.json")).unwrap())
                .unwrap();
        let entry = entries
            .into_iter()
            .find(|entry| entry.file == name)
            .expect("capture in index.json");
        let text = std::fs::read_to_string(root.join(format!("{name}.txt"))).unwrap();
        let inks = std::fs::read_to_string(root.join(format!("{name}.ink"))).unwrap();
        let rows = parse::rows(&text, &inks);
        let capture = Self {
            entry,
            rows,
            spinner_cells: Vec::new(),
        };
        let (width, height) = capture.entry.size.split_once('x').expect("capture size");
        assert_eq!(
            capture.size(),
            Rect::new(0, 0, width.parse().unwrap(), height.parse().unwrap())
        );
        capture
    }

    /// Spinner locations in capture coordinates: (row, column).
    pub fn spinners(mut self, cells: &[(u16, u16)]) -> Self {
        self.spinner_cells = cells.to_vec();
        self
    }

    pub fn size(&self) -> Rect {
        let width = self.rows.first().map_or(0, |row| row.cells.len());
        assert!(
            self.rows.iter().all(|row| row.cells.len() == width),
            "capture rows have one width"
        );
        Rect::new(0, 0, width as u16, self.rows.len() as u16)
    }

    pub fn area(&self, band: Band) -> Rect {
        let size = self.size();
        let footer = size.height.saturating_sub(1);
        let mut dock = footer;
        while dock > 0 && parse::full_ground(&self.rows[usize::from(dock - 1)], size.width) {
            dock -= 1;
        }
        let (y, height) = match band {
            Band::Stream => (0, dock),
            Band::Dock => (dock, footer - dock),
            Band::Footer => (footer, 1),
            Band::Frame => (0, size.height),
        };
        Rect::new(0, y, size.width, height)
    }

    pub fn now(&self) -> Now {
        Now {
            wall: jiff::Timestamp::from_millisecond(CLOCK_ORIGIN_MS + self.entry.moment_ms)
                .unwrap(),
            motion: self.entry.flags["reduced"] != true,
            ..super::scene().1
        }
    }

    pub fn render<R>(&self, draw: impl FnOnce() -> R) -> R {
        let look = theme::detect_palette(
            theme::Ask {
                truecolor: true,
                ascii: self.entry.flags["ascii"] == true,
                ..Default::default()
            },
            Some((0x3a3a, 0x3434, 0x2e2e)),
            Some((0xf9f9, 0xf4f4, 0xf2f2)),
        );
        crate::render::Render {
            look,
            motion: self.now().motion,
            ..Default::default()
        }
        .scope(draw)
    }
}

/// Standalone bands render at the capture band's size, independent of other milestones.
pub fn assert_band(capture: &Capture, band: Band, buffer: &Buffer) {
    assert_area(capture, band, buffer, buffer.area);
}

/// A full frame uses its own layout regions, never the capture's geometry to crop itself.
pub fn assert_regions(
    capture: &Capture,
    band: Band,
    buffer: &Buffer,
    regions: &crate::frame::Regions,
) {
    let area = match band {
        Band::Frame => buffer.area,
        Band::Stream => regions.above(),
        Band::Footer => regions.status,
        Band::Dock => [regions.composer, regions.palette]
            .into_iter()
            .filter(|rect| rect.height > 0)
            .reduce(|a, b| a.union(b))
            .unwrap_or_default(),
    };
    assert_area(capture, band, buffer, area);
}

pub fn assert_area(capture: &Capture, band: Band, buffer: &Buffer, area: Rect) {
    if let Err(report) = compare::compare(capture, band, buffer, area) {
        panic!("{report}");
    }
}

pub fn moved(a: &Buffer, b: &Buffer) -> Vec<(u16, u16)> {
    assert_eq!(a.area, b.area, "motion frames must have the same geometry");
    (a.area.y..a.area.bottom())
        .flat_map(|y| (a.area.x..a.area.right()).map(move |x| (x, y)))
        .filter(|&pos| a[pos].symbol() != b[pos].symbol() || a[pos].style() != b[pos].style())
        .collect()
}

#[cfg(test)]
mod tests;
