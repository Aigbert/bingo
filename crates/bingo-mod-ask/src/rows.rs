//! A question drawn in the dock: a tab row when there are several, the
//! question, then its choices as a pick list with a reason each, and the
//! picked choice's preview beside them when the dock is wide enough.
use crate::state::{self, Field, State};
use bingo_sdk::{
    Question, View,
    glyph::{Glyph, GlyphSet},
    pick,
    view::{BoxStyle, Direction},
};

/// The text column and the right margin of the dock.
const TX: u16 = 3;
const RM: u16 = 2;
/// Columns a dock needs before a preview sits beside the choices.
const SIDE_FROM: u16 = 90;
const PREVIEW_MAX: usize = 10;
pub const SOMETHING_ELSE: &str = "Something else";

pub struct Sheet<'a> {
    pub questions: &'a [Question],
    pub state: &'a State,
    pub columns: u16,
    pub glyphs: GlyphSet,
}

pub fn rows(sheet: &Sheet<'_>) -> Vec<View> {
    let Some(question) = sheet.questions.get(sheet.state.tab) else {
        return Vec::new();
    };
    let several = sheet.questions.len() > 1;
    let mut out = Vec::new();
    if several {
        out.push(tabs(sheet));
    }
    let header = question.header.as_deref().filter(|_| !several);
    out.push(line(
        pick::bold(&question.question, "fg"),
        header.map(|header| pick::text(header, "dim")),
        sheet,
    ));
    out.push(pick::text("", "fg"));
    out.extend(choices(sheet, question));
    out
}

/// Each question's header, ticked once answered, and where the person is.
fn tabs(sheet: &Sheet<'_>) -> View {
    let mut runs = Vec::new();
    for (index, question) in sheet.questions.iter().enumerate() {
        let on = index == sheet.state.tab;
        let done = sheet.state.field(index).answer.is_some();
        if done {
            let ok = Glyph::Ok.spell(sheet.glyphs);
            runs.push(pick::text(format!("{ok} "), "good"));
        }
        let header = question.header.as_deref().unwrap_or(&question.question);
        runs.push(match (on, done) {
            (true, _) => pick::bold(header, "fg"),
            (false, true) => pick::text(header, "muted"),
            (false, false) => pick::text(header, "dim"),
        });
        runs.push(pick::text("    ", "dim"));
    }
    let count = format!("{} of {}", sheet.state.tab + 1, sheet.questions.len());
    line(pick::line(runs), Some(pick::text(count, "dim")), sheet)
}

fn line(left: View, right: Option<View>, sheet: &Sheet<'_>) -> View {
    // No identity: a heading is not a choice a click could pick.
    let row = pick::Row::new("", left);
    let row = match right {
        Some(right) => row.right(right),
        None => row,
    };
    pick::pick_line(&row, false, sheet.columns, sheet.glyphs)
}

/// The choices, `Something else` last, with the picked one's preview.
fn choices(sheet: &Sheet<'_>, question: &Question) -> Vec<View> {
    let field = sheet.state.field(sheet.state.tab);
    let preview = question
        .options
        .get(field.cursor)
        .and_then(|option| option.preview.as_deref())
        .filter(|_| sheet.columns >= SIDE_FROM);
    let room = sheet.columns.saturating_sub(TX + RM);
    let list = if preview.is_some() { room / 2 } else { room };
    let choice = Choice {
        question,
        field: &field,
        words: sheet.state.words.as_deref(),
        label_width: label_width(question),
        list,
        glyphs: sheet.glyphs,
    };
    let rows = (0..state::choices(question)).map(|index| choice.row(index));
    match preview {
        None => rows
            .enumerate()
            .map(|(index, row)| {
                pick::pick_line(&row, choice.picked(index), sheet.columns, sheet.glyphs)
            })
            .collect(),
        Some(preview) => beside(rows, &choice, preview, sheet),
    }
}

fn label_width(question: &Question) -> usize {
    question
        .options
        .iter()
        .map(|option| cells(&option.label))
        .chain(question.free_text.then(|| cells(SOMETHING_ELSE)))
        .max()
        .unwrap_or(0)
        + 3
}

struct Choice<'a> {
    question: &'a Question,
    field: &'a Field,
    words: Option<&'a str>,
    label_width: usize,
    list: u16,
    glyphs: GlyphSet,
}

impl Choice<'_> {
    fn picked(&self, index: usize) -> bool {
        index == self.field.cursor
    }

    fn row(&self, index: usize) -> pick::Row {
        let picked = self.picked(index);
        let left = match self.question.options.get(index) {
            Some(option) => {
                let mark = self.mark(index);
                // The reasons of a multiple choice sit four cells further in,
                // two for each of the tick's runs, as the prototype has them.
                let marked = 2 * mark.len();
                let mut runs = mark;
                runs.push(label(&option.label, picked));
                let head = pick::pad(pick::line(runs), self.label_width + marked);
                let room = usize::from(self.list).saturating_sub(self.label_width + 3);
                let reason = option.description.as_deref().map(|text| {
                    fitted(
                        text,
                        room,
                        if picked { "muted" } else { "dim" },
                        self.glyphs,
                    )
                });
                pick::line([head].into_iter().chain(reason))
            }
            None => match self.words.filter(|_| picked) {
                Some(typed) => pick::line([
                    pick::text(format!("{SOMETHING_ELSE}: "), "muted"),
                    pick::text(typed, "fg"),
                    pick::text(Glyph::Caret.spell(self.glyphs), "glow"),
                ]),
                None => label(SOMETHING_ELSE, picked),
            },
        };
        pick::Row::new(index.to_string(), left)
    }

    /// A multiple choice ticks each option with `✓` or leaves it at `·`.
    fn mark(&self, index: usize) -> Vec<View> {
        if !self.question.multi {
            return Vec::new();
        }
        let (glyph, ink) = match self.field.ticks.contains(&index) {
            true => (Glyph::Ok, "good"),
            false => (Glyph::Dot, "dim"),
        };
        vec![
            pick::text(glyph.spell(self.glyphs), ink),
            pick::text(" ", "dim"),
        ]
    }
}

fn label(text: &str, picked: bool) -> View {
    match picked {
        true => pick::bold(text, "fg"),
        false => pick::text(text, "muted"),
    }
}

/// The list on the left and the picked choice's preview on `wellDeep` to
/// its right, running below the last choice when it is longer.
fn beside(
    rows: impl Iterator<Item = pick::Row>,
    choice: &Choice<'_>,
    preview: &str,
    sheet: &Sheet<'_>,
) -> Vec<View> {
    let list = TX + choice.list;
    let lines: Vec<&str> = preview.lines().take(PREVIEW_MAX).collect();
    // The pick band ends a cell before the gap; the well stops at the margin.
    let band = list;
    let well = sheet.columns.saturating_sub(band + 3);
    let room = usize::from(well.saturating_sub(4));
    // The choice's own cells sit in the row itself, so a click finds its
    // identity where it finds every pick list's (`sheet::clicks`).
    let mut out: Vec<Vec<View>> = rows
        .enumerate()
        .map(|(index, row)| {
            match pick::pick_line(&row, choice.picked(index), band + 1, sheet.glyphs) {
                View::Box { children, .. } => children,
                other => vec![pick::cell(other, band)],
            }
        })
        .collect();
    while out.len() < lines.len() {
        out.push(vec![blank(band)]);
    }
    out.into_iter()
        .enumerate()
        .map(|(index, mut row)| {
            let text = lines.get(index).copied().unwrap_or("");
            let shown = fitted(text, room, "code", sheet.glyphs);
            let ground = pick::line([pick::text("  ", "code"), shown]);
            row.extend([blank(2), grounded(pick::cell(ground, well), well), blank(1)]);
            boxed(row)
        })
        .collect()
}

fn blank(width: u16) -> View {
    pick::cell(pick::text("", "fg"), width)
}

fn grounded(view: View, width: u16) -> View {
    View::Box {
        children: vec![view],
        layout: Box::new(BoxStyle {
            ground: Some("wellDeep".into()),
            width: Some(width),
            height: Some(1),
            ..Default::default()
        }),
    }
}

fn boxed(children: Vec<View>) -> View {
    View::Box {
        children,
        layout: Box::new(BoxStyle {
            flex_direction: Some(Direction::Row),
            height: Some(1),
            ..Default::default()
        }),
    }
}

/// The words cut to `room` cells, an ellipsis standing for the rest.
fn fitted(text: &str, room: usize, ink: &str, glyphs: GlyphSet) -> View {
    if cells(text) <= room {
        return pick::text(text, ink);
    }
    let ell = Glyph::Ell.spell(glyphs);
    let room = room.saturating_sub(cells(ell));
    let mut kept = String::new();
    for ch in text.chars() {
        let mut next = kept.clone();
        next.push(ch);
        if cells(&next) > room {
            break;
        }
        kept = next;
    }
    pick::line([pick::text(kept, ink), pick::text(ell, "dim")])
}

fn cells(text: &str) -> usize {
    pick::width(&pick::text(text, "fg"))
}
