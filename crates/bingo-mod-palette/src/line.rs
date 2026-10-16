//! One row of the list as it is drawn: a name column as wide as the widest
//! name of its kind, a description cut on a word, and the value set right.

use crate::rows::{Group, Row, Target};
use bingo_sdk::view::{self, View};

/// The widest a name column grows; a longer name ends in `…`.
const NAME_CAP: usize = 16;
/// Between the name column and the description.
const GAP: usize = 3;
/// The cursor's column, or its blank.
const LEAD: usize = 3;
/// After the value, so it does not touch the edge.
const TAIL: usize = 2;

/// The name column of each kind of row, measured over the whole list so it
/// holds still while the cursor walks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Columns {
    width: usize,
    command: usize,
    skill: usize,
    key: usize,
}

impl Columns {
    pub fn of(groups: &[Group], width: usize) -> Self {
        let widest = |kind: Kind| {
            groups
                .iter()
                .flat_map(|group| &group.rows)
                .filter(|row| Kind::of(&row.target) == Some(kind))
                .map(|row| row.label.chars().count())
                .max()
                .unwrap_or(0)
                .min(NAME_CAP)
        };
        Self {
            width,
            command: widest(Kind::Command),
            skill: widest(Kind::Skill),
            key: widest(Kind::Key),
        }
    }

    fn name(&self, target: &Target) -> Option<usize> {
        Some(match Kind::of(target)? {
            Kind::Command => self.command,
            Kind::Skill => self.skill,
            Kind::Key => self.key,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Command,
    Skill,
    Key,
}

impl Kind {
    fn of(target: &Target) -> Option<Self> {
        match target {
            Target::Command(_) => Some(Self::Command),
            Target::Skill(_) => Some(Self::Skill),
            Target::Key(_) | Target::Inert => Some(Self::Key),
            _ => None,
        }
    }
}

/// A group's name, dim, or an argument level's title: the command in glow
/// and what it does dim.
pub fn heading(name: &str) -> View {
    match name.split_once("  ").filter(|_| name.starts_with('/')) {
        Some((command, hint)) => View::Box {
            children: vec![
                styled(format!("   {command}  "), Some("glow"), false),
                styled(hint.into(), None, true),
            ],
            layout: Box::new(view::BoxStyle {
                flex_direction: Some(view::Direction::Row),
                height: Some(1),
                ..Default::default()
            }),
        },
        None => styled(format!("   {name}"), None, true),
    }
}

/// A line of the list's own words, in ash.
pub fn note(text: &str) -> View {
    styled(format!("   {text}"), Some("ash"), false)
}

/// One row: cursor, name, description, value.
pub fn row(row: &Row, selected: bool, columns: Columns) -> View {
    let inert = matches!(row.target, Target::Inert);
    let selected = selected && !inert;
    let name = columns.name(&row.target);
    let label = match name {
        Some(width) => cut_name(&row.label, width),
        None => row.label.clone(),
    };
    let label_width = name.map_or(label.chars().count() + 2, |width| width + GAP);
    let room = columns
        .width
        .saturating_sub(LEAD + label_width + row.now.chars().count() + TAIL + 1);
    let mut children = vec![
        styled(
            (if selected { " ❯ " } else { "   " }).into(),
            Some("glow"),
            false,
        ),
        View::Box {
            children: vec![name_view(row, &label, inert)],
            layout: Box::new(view::BoxStyle {
                width: u16::try_from(label_width).ok(),
                ..Default::default()
            }),
        },
        marked(&cut(&row.hint, room), &row.hint_marks, Style::dim()),
    ];
    if let Some(alert) = &row.alert {
        children.push(styled(format!(" · {alert}"), Some("yellow"), false));
    }
    View::Box {
        children: vec![
            View::Box {
                children,
                layout: Box::new(view::BoxStyle {
                    flex_direction: Some(view::Direction::Row),
                    flex_grow: Some(1),
                    ..Default::default()
                }),
            },
            value(row),
        ],
        layout: Box::new(view::BoxStyle {
            flex_direction: Some(view::Direction::Row),
            justify_content: Some(view::Justify::SpaceBetween),
            height: Some(1),
            ..Default::default()
        }),
    }
}

fn name_view(row: &Row, label: &str, inert: bool) -> View {
    let style = match row.target {
        Target::Key(_) => Style {
            color: Some("glow"),
            bold: true,
            dim: false,
        },
        _ => Style {
            color: None,
            bold: false,
            dim: inert,
        },
    };
    marked(label, &row.marks, style)
}

/// The value at the row's end: a skill's shelf in ash, the current argument
/// in glow, anything else dim.
fn value(row: &Row) -> View {
    let text = format!("{}  ", row.now);
    match row.target {
        Target::Skill(_) => styled(text, Some("ash"), false),
        _ if row.now == "now" => styled(text, Some("glow"), false),
        _ => styled(text, None, true),
    }
}

/// A name cut to its column, ending in `…` when it is longer.
fn cut_name(name: &str, width: usize) -> String {
    if name.chars().count() <= width {
        return name.into();
    }
    let mut kept = name
        .chars()
        .take(width.saturating_sub(1))
        .collect::<String>();
    kept.push('…');
    kept
}

/// A description cut to `width` on a word, ending in `…` when anything was
/// left out; a first word longer than the room is cut where it ends.
pub fn cut(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.into();
    }
    if width == 0 {
        return String::new();
    }
    let room = width - 1;
    let head = text.chars().take(room + 1).collect::<String>();
    let kept = match head.rfind(' ') {
        Some(at) if at > 0 => head[..at].trim_end(),
        _ => {
            &head[..head
                .char_indices()
                .nth(room)
                .map_or(head.len(), |(at, _)| at)]
        }
    };
    format!("{kept}…")
}

#[derive(Clone, Copy)]
struct Style {
    color: Option<&'static str>,
    bold: bool,
    dim: bool,
}

impl Style {
    fn dim() -> Self {
        Self {
            color: None,
            bold: false,
            dim: true,
        }
    }
}

fn styled(text: String, color: Option<&str>, dim: bool) -> View {
    View::Text {
        children: vec![view::TextChild::String(text)],
        style: view::TextStyle {
            color: color.map(str::to_owned),
            dim_color: dim.then_some(true),
            ..Default::default()
        },
    }
}

/// `text` with the letters at `marks` in bold.
fn marked(text: &str, marks: &[usize], style: Style) -> View {
    let mut runs: Vec<(String, bool)> = Vec::new();
    for (index, character) in text.chars().enumerate() {
        let hit = marks.contains(&index);
        match runs.last_mut() {
            Some((run, previous)) if *previous == hit => run.push(character),
            _ => runs.push((character.to_string(), hit)),
        }
    }
    View::Text {
        children: runs
            .into_iter()
            .map(|(text, hit)| {
                view::TextChild::Node(Box::new(View::Text {
                    children: vec![view::TextChild::String(text)],
                    style: view::TextStyle {
                        bold: Some(hit || style.bold),
                        ..Default::default()
                    },
                }))
            })
            .collect(),
        style: view::TextStyle {
            color: style.color.map(str::to_owned),
            dim_color: Some(style.dim),
            ..Default::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(label: &str, target: Target) -> Row {
        Row {
            label: label.into(),
            hint: String::new(),
            alert: None,
            now: String::new(),
            marks: vec![],
            hint_marks: vec![],
            target,
        }
    }

    #[test]
    fn each_kind_measures_its_own_widest_name_up_to_the_cap() {
        let groups = vec![Group {
            name: "commands".into(),
            rows: vec![
                row("/model", Target::Command("model".into())),
                row("/permission", Target::Command("permission".into())),
                row("shift+tab", Target::Key("/permission".into())),
                row(
                    "/a-very-long-skill-name",
                    Target::Skill("a-very-long-skill-name".into()),
                ),
            ],
        }];
        let columns = Columns::of(&groups, 100);
        assert_eq!(columns.name(&Target::Command(String::new())), Some(11));
        assert_eq!(columns.name(&Target::Key(String::new())), Some(9));
        assert_eq!(columns.name(&Target::Skill(String::new())), Some(NAME_CAP));
        assert_eq!(columns.name(&Target::Session(String::new())), None);
        assert_eq!(
            cut_name("/a-very-long-skill-name", NAME_CAP),
            "/a-very-long-sk…"
        );
    }

    #[test]
    fn a_description_is_cut_on_a_word() {
        assert_eq!(cut("summarise older turns", 40), "summarise older turns");
        assert_eq!(cut("summarise older turns", 16), "summarise older…");
        assert_eq!(cut("summarise older turns", 15), "summarise…");
        assert_eq!(cut("unbreakableword", 6), "unbre…");
        assert_eq!(cut("anything", 0), "");
    }
}
