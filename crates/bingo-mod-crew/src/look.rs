//! One row of the crew sheet (`crewRow`, `stateMark`): a lamp in the row's
//! ink, the name, what it is doing, and on the right where it stands.
use crate::state::{Entry, Mate, Need, Place};
use bingo_sdk::{
    View, glyph,
    pick::{self, bold, line, pad, text},
    view,
};

/// The name column, in cells.
const NAME: usize = 11;

/// What an entry's row says, with the clock it measures work against.
pub fn row(entry: &Entry, picked: bool, now_ms: i64, glyphs: glyph::GlyphSet) -> pick::Row {
    let (left, right) = match entry {
        Entry::Mate(mate) => mate_row(mate, picked, now_ms, glyphs),
        Entry::Room(place) => room_row(place, picked, glyphs),
    };
    pick::Row::new(entry.id().as_str(), left).right(right)
}

fn mate_row(mate: &Mate, picked: bool, now_ms: i64, glyphs: glyph::GlyphSet) -> (View, View) {
    let what = match (&mate.doing, &mate.role) {
        (Some(doing), _) => text(first_line(doing), "muted"),
        (None, Some(role)) => text(role, "dim"),
        (None, None) => text("", "dim"),
    };
    let left = line([
        lamp(mate.need, mate.ink, glyphs),
        text("  ", "fg"),
        name(&mate.name, mate.ink, picked),
        what,
    ]);
    let right = match mate.need {
        Need::Asking => text("needs you", "amber"),
        Need::Working => text(
            format!(
                "working {}",
                bingo_sdk::remaining(now_ms - mate.since.unwrap_or(now_ms))
            ),
            "muted",
        ),
        Need::Idle => text("idle", "dim"),
    };
    (left, right)
}

fn room_row(place: &Place, picked: bool, glyphs: glyph::GlyphSet) -> (View, View) {
    let need = if place.writing {
        Need::Working
    } else {
        Need::Idle
    };
    let left = line([
        lamp(need, "plan", glyphs),
        text("  ", "fg"),
        name(&place.name, "plan", picked),
        text(place.purpose.as_deref().unwrap_or_default(), "dim"),
    ]);
    let right = if place.closed {
        "closed".to_string()
    } else {
        format!("{} seated", place.seated)
    };
    (left, text(right, "dim"))
}

fn name(name: &str, ink: &str, picked: bool) -> View {
    let name = if picked {
        bold(name, ink)
    } else {
        text(name, ink)
    };
    pad(name, NAME)
}

/// A list row's lamp: amber breathing when it needs you, its ink breathing
/// while it works, quiet otherwise.
pub fn lamp(need: Need, ink: &str, glyphs: glyph::GlyphSet) -> View {
    let mark = glyph::Glyph::Lamp.spell(glyphs);
    let (ink, period_ms, depth) = match need {
        Need::Asking => ("amberGlow", 1100, 0.6),
        Need::Working => (ink, 1800, 0.55),
        Need::Idle => return text(mark, "faint"),
    };
    let mut lamp = text(mark, ink);
    if let View::Text { style, .. } = &mut lamp {
        style.motion = Some(Box::new(view::TextMotion::Breathe {
            with: "you".into(),
            period_ms,
            depth,
        }));
    }
    lamp
}

fn first_line(text: &str) -> &str {
    text.lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default()
        .trim()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::tests::{mate, place};

    fn said(row: &pick::Row) -> (String, String) {
        (
            row.left.fold(),
            row.right.as_ref().map(View::fold).unwrap_or_default(),
        )
    }

    #[test]
    fn a_teammate_says_what_it_does_or_its_role_and_where_it_stands() {
        let mut working = mate("scout", Need::Working);
        working.doing = Some("\nmap the callers of reset()\nand more".into());
        working.since = Some(1_000);
        let (left, right) = said(&row(
            &Entry::Mate(working),
            false,
            76_000,
            glyph::GlyphSet::Unicode,
        ));
        assert_eq!(left, "●  scout      map the callers of reset()");
        assert_eq!(right, "working 1m 15s");
        let mut idle = mate("ada", Need::Idle);
        idle.role = Some("reviewer".into());
        let (left, right) = said(&row(&Entry::Mate(idle), true, 0, Default::default()));
        assert_eq!(
            (left.as_str(), right.as_str()),
            ("●  ada        reviewer", "idle")
        );
        let asking = row(
            &Entry::Mate(mate("bo", Need::Asking)),
            false,
            0,
            Default::default(),
        );
        assert_eq!(said(&asking).1, "needs you");
    }

    #[test]
    fn a_room_says_its_purpose_and_its_seats_or_that_it_closed() {
        let mut room = place("design");
        room.purpose = Some("settle reset()".into());
        room.seated = 2;
        let (left, right) = said(&row(
            &Entry::Room(room.clone()),
            false,
            0,
            Default::default(),
        ));
        assert_eq!(left, "●  #design    settle reset()");
        assert_eq!(right, "2 seated");
        room.closed = true;
        assert_eq!(
            said(&row(&Entry::Room(room), false, 0, Default::default())).1,
            "closed"
        );
    }

    #[test]
    fn only_a_lamp_that_needs_or_works_breathes() {
        let breathes = |need| {
            serde_json::to_string(&lamp(need, "crew1", Default::default()))
                .unwrap()
                .contains("breathe")
        };
        assert!(breathes(Need::Asking) && breathes(Need::Working));
        assert!(!breathes(Need::Idle));
    }
}
