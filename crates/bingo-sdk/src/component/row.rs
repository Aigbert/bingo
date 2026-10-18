//! One row of words in named inks, laid into a width. Every owner of a site
//! that draws such a row lays it with the same function, so the built-in
//! drawing and a surface's own fallback cannot disagree by a cell.

/// What a piece of a row is drawn in. The surface owns the colour each names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    Plain,
    Dim,
    /// Quieter than dim: what the person already said.
    Ash,
    Good,
    Bad,
    /// Wants a person.
    Glow,
    /// bingo at work.
    Presence,
}

impl Ink {
    /// The ink a tone is drawn in: news that is neither good nor bad is dim.
    pub fn of(tone: crate::view::Tone) -> Self {
        match tone {
            crate::view::Tone::Neutral => Ink::Dim,
            crate::view::Tone::Good => Ink::Good,
            crate::view::Tone::Bad => Ink::Bad,
            crate::view::Tone::Attention => Ink::Glow,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Piece {
    pub text: String,
    pub ink: Ink,
}

impl Piece {
    pub fn new(text: impl Into<String>, ink: Ink) -> Self {
        Self {
            text: text.into(),
            ink,
        }
    }
}

/// The cells a right-hand part stops short of the edge, as the prototype
/// leaves them.
pub const MARGIN: usize = 2;
/// The least space the left part keeps before the right one gives way.
const LEFT_KEEPS: usize = 12;

/// `left` from the row's start and `right` ending [`MARGIN`] cells short of
/// `width`, the gap between them filled. A left part too long for the row is
/// cut with `…`; a row too narrow for both drops the right part first.
/// `cells` measures a string as the surface draws it.
pub fn laid(
    left: Vec<Piece>,
    right: Vec<Piece>,
    width: usize,
    cells: impl Fn(&str) -> usize,
) -> Vec<Piece> {
    let measure = |pieces: &[Piece]| pieces.iter().map(|p| cells(&p.text)).sum::<usize>();
    let right_cells = measure(&right);
    let space = width.saturating_sub(MARGIN);
    let keeps_right = !right.is_empty() && space >= right_cells + 1 + LEFT_KEEPS;
    let left_space = match keeps_right {
        true => space - right_cells - 1,
        false => space,
    };
    let mut out: Vec<_> = cut(
        left.into_iter().map(|p| (p.text, p.ink)).collect(),
        left_space,
        "…",
        Ink::Dim,
        &cells,
    )
    .into_iter()
    .map(|(text, ink)| Piece::new(text, ink))
    .collect();
    if keeps_right {
        let gap = space - right_cells - measure(&out);
        out.push(Piece::new(" ".repeat(gap), Ink::Plain));
        out.extend(right);
    }
    out
}

/// The pieces that fit in `space` cells, the last of them ending in `…` when
/// anything had to go.
pub fn cut<T: Clone>(
    pieces: Vec<(String, T)>,
    space: usize,
    ellipsis: &str,
    dim: T,
    cells: &impl Fn(&str) -> usize,
) -> Vec<(String, T)> {
    let whole: usize = pieces.iter().map(|(text, _)| cells(text)).sum();
    if whole <= space {
        return pieces;
    }
    let mut left = space.saturating_sub(cells(ellipsis));
    let mut out = Vec::new();
    for (text, ink) in pieces {
        let mut kept = String::new();
        for c in text.chars() {
            let size = cells(c.encode_utf8(&mut [0; 4]));
            if size > left {
                out.push((kept.trim_end().to_owned(), ink));
                out.push((ellipsis.chars().take(space).collect(), dim));
                return out;
            }
            left -= size;
            kept.push(c);
        }
        out.push((kept, ink));
    }
    out
}

/// A path gives up leading directories before its basename.
pub fn short_path(path: &str, width: usize, ellipsis: &str) -> (String, String) {
    use unicode_width::UnicodeWidthStr;
    let cut = path.rfind('/').map_or(0, |at| at + 1);
    let base = &path[cut..];
    let mut dirs: Vec<_> = path[..cut].split('/').filter(|s| !s.is_empty()).collect();
    let mut lead = if path.starts_with('/') {
        "/".into()
    } else {
        String::new()
    };
    loop {
        let head = if dirs.is_empty() {
            lead.clone()
        } else {
            format!("{lead}{}/", dirs.join("/"))
        };
        if dirs.is_empty() || head.width() + base.width() <= width {
            return (head, base.into());
        }
        dirs.remove(0);
        lead = format!("{ellipsis}/");
    }
}

/// Whole elapsed seconds in Ember's compact clock notation.
pub fn fmt_dur(ms: u64) -> String {
    let seconds = ms / 1000;
    match seconds {
        0..60 => format!("{seconds}s"),
        60..3600 => format!("{}m {:02}s", seconds / 60, seconds % 60),
        _ => format!("{}h {:02}m", seconds / 3600, seconds / 60 % 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(pieces: &[Piece]) -> String {
        pieces.iter().map(|p| p.text.as_str()).collect()
    }
    fn width(s: &str) -> usize {
        s.chars().count()
    }

    #[test]
    fn the_right_part_ends_two_cells_short_of_the_edge() {
        let row = laid(
            vec![Piece::new("read src/parse.rs", Ink::Plain)],
            vec![Piece::new("3 passed", Ink::Good)],
            40,
            width,
        );
        let drawn = text(&row);
        assert_eq!(width(&drawn), 38);
        assert!(drawn.ends_with(" 3 passed"), "{drawn:?}");
        assert!(drawn.starts_with("read src/parse.rs "), "{drawn:?}");
    }

    #[test]
    fn a_long_left_is_cut_and_a_narrow_row_drops_the_right() {
        let left = vec![Piece::new(
            "bash cargo test --workspace --locked",
            Ink::Plain,
        )];
        let right = vec![Piece::new("exit 1", Ink::Bad)];
        let row = laid(left.clone(), right.clone(), 30, width);
        assert_eq!(text(&row), "bash cargo test --wo… exit 1");
        assert_eq!(width(&text(&row)), 28);
        let narrow = laid(left, right, 16, width);
        assert_eq!(text(&narrow), "bash cargo te…", "the right gave way");
    }
}

#[cfg(test)]
mod clock_tests {
    #[test]
    fn elapsed_reads_like_the_prototype() {
        for (ms, text) in [
            (999, "0s"),
            (1000, "1s"),
            (61999, "1m 01s"),
            (3660000, "1h 01m"),
        ] {
            assert_eq!(super::fmt_dur(ms), text);
        }
    }
}
