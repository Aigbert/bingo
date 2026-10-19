//! One row of the list as it is drawn: the name with the letters typed in
//! glow, the description at the name column, and the value set right.

use crate::rows::{Row, Target};
use bingo_sdk::{
    View,
    component::row::cut,
    glyph::{self, Glyph, GlyphSet},
    pick,
};

/// The widest the name column grows.
const NAME_CAP: usize = 18;
/// Between the longest name and the description.
const GAP: usize = 3;
/// What `pick_line` keeps between the left run's end and the value.
const BEFORE_VALUE: usize = 3;

/// The name column, measured over the whole list so it holds still while
/// the cursor walks; `width` is the frame's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Columns {
    pub width: u16,
    pub name: usize,
}

impl Columns {
    pub fn of(rows: &[Row], width: u16) -> Self {
        let widest = rows
            .iter()
            .map(|row| pick::cells(&row.label))
            .max()
            .unwrap_or(0);
        Self {
            width,
            name: NAME_CAP.min(widest + GAP),
        }
    }
}

/// A line of the list's own words, dim.
pub fn note(text: &str) -> View {
    pick::text(text, "dim")
}

/// One row, picked or not.
pub fn row(row: &Row, picked: bool, columns: Columns, glyphs: GlyphSet) -> pick::Row {
    let right = value(row, glyphs);
    let room = usize::from(columns.width.saturating_sub(2))
        .saturating_sub(pick::width(&right) + BEFORE_VALUE);
    let name = name(row, picked);
    let used = pick::width(&name).max(columns.name.saturating_sub(1)) + 1;
    let hint = described(
        &row.hint,
        room.saturating_sub(used + BEFORE_VALUE),
        picked,
        glyphs,
    );
    pick::Row::new(id(row), pick::line([pick::pad(name, used), hint])).right(right)
}

/// A command or skill reads as typed, after a `/`; anything else as is.
fn name(row: &Row, picked: bool) -> View {
    let ink = if picked { "fg" } else { "muted" };
    let label = pick::marked(&row.label, &row.marks, "glow", |_| ink);
    match row.target {
        Target::Command(_) | Target::Skill(_) => {
            pick::line([pick::text("/", if picked { "glow" } else { "dim" }), label])
        }
        _ => label,
    }
}

fn described(hint: &str, room: usize, picked: bool, glyphs: GlyphSet) -> View {
    let ink = if picked { "muted" } else { "dim" };
    pick::line(
        cut(
            vec![(hint.to_owned(), ink)],
            room,
            Glyph::Ell.spell(glyphs),
            "dim",
            &pick::cells,
        )
        .into_iter()
        .map(|(text, ink)| pick::text(text, ink)),
    )
}

/// The value at the row's end: a command's key, a skill's shelf, a
/// session's age, or what needs the person's attention, in amber.
fn value(row: &Row, glyphs: GlyphSet) -> View {
    if let Some(alert) = &row.alert {
        return pick::text(alert, "amber");
    }
    let text = match row.target {
        Target::Command(_) => glyph::chord(&row.now, glyphs),
        Target::Skill(_) => format!("skill{}{}", Glyph::Sep.spell(glyphs), row.now),
        _ => row.now.clone(),
    };
    pick::text(text, if row.now == "now" { "glow" } else { "dim" })
}

/// The row's identity: what choosing it does, so a redraw keeps the pick.
pub fn id(row: &Row) -> String {
    format!("{:?}", row.target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(label: &str, hint: &str, now: &str, target: Target) -> Row {
        Row {
            label: label.into(),
            hint: hint.into(),
            alert: None,
            now: now.into(),
            marks: vec![],
            target,
        }
    }

    fn drawn(row: &Row, picked: bool, width: u16, glyphs: GlyphSet) -> String {
        let columns = Columns::of(std::slice::from_ref(row), width);
        super::row(row, picked, columns, glyphs).left.fold()
    }

    #[test]
    fn the_name_column_is_the_widest_name_and_three_up_to_eighteen() {
        let rows = [
            row("model", "", "", Target::Command("model".into())),
            row("experience", "", "", Target::Command("experience".into())),
        ];
        assert_eq!(Columns::of(&rows, 100).name, 13);
        let long = [row(
            "a-very-long-skill-name",
            "",
            "",
            Target::Skill("x".into()),
        )];
        assert_eq!(Columns::of(&long, 100).name, NAME_CAP);
    }

    #[test]
    fn a_command_reads_slash_name_then_its_description_at_the_column() {
        let model = row(
            "model",
            "switch model and effort",
            "alt+m",
            Target::Command("model".into()),
        );
        assert_eq!(
            drawn(&model, false, 100, GlyphSet::Unicode),
            "/model  switch model and effort"
        );
    }

    #[test]
    fn a_description_too_long_ends_in_the_sets_ellipsis() {
        let long = row(
            "permission",
            "mode and rules for what runs without asking",
            "shift+tab",
            Target::Command("permission".into()),
        );
        let unicode = drawn(&long, false, 40, GlyphSet::Unicode);
        assert!(unicode.ends_with('…'), "{unicode}");
        assert!(!unicode.contains(" …"), "{unicode}");
        let ascii = drawn(&long, false, 40, GlyphSet::Ascii);
        assert!(ascii.ends_with("..."), "{ascii}");
    }

    #[test]
    fn the_value_is_the_key_as_the_set_spells_it_or_the_skills_shelf() {
        let permission = row(
            "permission",
            "",
            "shift+tab",
            Target::Command("permission".into()),
        );
        assert_eq!(value(&permission, GlyphSet::Unicode).fold(), "⇧tab");
        assert_eq!(value(&permission, GlyphSet::Ascii).fold(), "shift+tab");
        let review = row("review", "", "project", Target::Skill("review".into()));
        assert_eq!(value(&review, GlyphSet::Unicode).fold(), "skill · project");
        assert_eq!(value(&review, GlyphSet::Ascii).fold(), "skill - project");
    }
}
