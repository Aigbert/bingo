//! Inline image bytes draw as terminal cells; unresolved sources keep their alt.
use crate::theme;
use base64::Engine;
use ratatui::{
    style::Style,
    text::{Line, Span},
};
pub fn lines(
    source: &str,
    alt: &str,
    columns: Option<u16>,
    rows: Option<u16>,
    width: usize,
) -> Vec<Line<'static>> {
    let columns = usize::from(columns.unwrap_or(32)).min(width).min(256);
    let rows = usize::from(rows.unwrap_or(8)).min(128);
    if columns == 0 || rows == 0 {
        return Vec::new();
    }
    if theme::current().colors == theme::Colors::Plain {
        return super::text::lines(&format!("[image: {alt}]"));
    }
    let Some(pixels) = decode(source, columns, rows) else {
        return super::text::lines(&format!("[image: {alt}]"));
    };
    (0..rows)
        .map(|y| {
            Line::from(
                (0..columns)
                    .map(|x| {
                        let top = pixels.at(
                            (x * pixels.width as usize / columns) as u32,
                            (y * 2 * pixels.height as usize / (rows * 2)) as u32,
                        );
                        let bottom = pixels.at(
                            (x * pixels.width as usize / columns) as u32,
                            ((y * 2 + 1) * pixels.height as usize / (rows * 2)) as u32,
                        );
                        let style = if theme::current().colors == theme::Colors::Plain {
                            Style::new()
                        } else {
                            Style::new()
                                .fg((top[0], top[1], top[2]).into())
                                .bg((bottom[0], bottom[1], bottom[2]).into())
                        };
                        Span::styled("▀", style)
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}
fn decode(source: &str, columns: usize, rows: usize) -> Option<bingo_pictures::Pixels> {
    let (header, data) = source.strip_prefix("data:")?.split_once(",")?;
    let media = header.strip_suffix(";base64")?;
    if data.len() > bingo_sdk::Image::MAX_BYTES * 2 {
        return None;
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data)
        .ok()?;
    let image = bingo_sdk::Image::from_bytes(media, &bytes).ok()?;
    let (width, height) = bingo_pictures::size(&image)?;
    if u64::from(width) * u64::from(height) > 16_000_000 {
        return None;
    }
    let fitted = bingo_pictures::fitted(&image, (columns as u32, (rows * 2) as u32)).ok()?;
    bingo_pictures::pixels(&fitted.bytes).ok()
}
