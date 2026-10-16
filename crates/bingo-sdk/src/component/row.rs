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
/// The least room the left part keeps before the right one gives way.
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
    let room = width.saturating_sub(MARGIN);
    let keeps_right = !right.is_empty() && room >= right_cells + 1 + LEFT_KEEPS;
    let left_room = match keeps_right {
        true => room - right_cells - 1,
        false => room,
    };
    let mut out = cut(left, left_room, &cells);
    if keeps_right {
        let gap = room - right_cells - measure(&out);
        out.push(Piece::new(" ".repeat(gap), Ink::Plain));
        out.extend(right);
    }
    out
}

/// The pieces that fit in `room` cells, the last of them ending in `…` when
/// anything had to go.
fn cut(pieces: Vec<Piece>, room: usize, cells: &impl Fn(&str) -> usize) -> Vec<Piece> {
    let whole: usize = pieces.iter().map(|p| cells(&p.text)).sum();
    if whole <= room {
        return pieces;
    }
    let mut left = room.saturating_sub(1);
    let mut out = Vec::new();
    for piece in pieces {
        let mut kept = String::new();
        for c in piece.text.chars() {
            let size = cells(c.encode_utf8(&mut [0; 4]));
            if size > left {
                left = 0;
                break;
            }
            left -= size;
            kept.push(c);
        }
        let ink = piece.ink;
        out.push(Piece::new(kept, ink));
        if left == 0 {
            out.push(Piece::new("…", ink));
            return out;
        }
    }
    out
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
