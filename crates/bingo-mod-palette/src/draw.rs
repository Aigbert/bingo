use crate::{
    Shared,
    rows::{Group, Row},
};
use bingo_sdk::{component::PaletteProps, *};
use serde_json::{Value, json};

pub(crate) struct Drawing(pub Shared);
#[async_trait::async_trait]
impl Interceptor for Drawing {
    fn id(&self) -> &str {
        "palette"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"component":"Palette"}),
            ..LinkMatcher::at(catalogue::UI_RENDER)
        }
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        cx: &HookContext,
        value: Value,
        _: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let request: render::RenderRequest = serde_json::from_value(value).map_err(crate::error)?;
        let props: PaletteProps = serde_json::from_value(request.props).map_err(crate::error)?;
        let mut shared = self.0.lock().map_err(crate::error)?;
        let (groups, initial) = groups(&props, shared.recent.names());
        let cursor = shared
            .cursors
            .entry((cx.session.clone(), request.request_id.clone()))
            .or_insert_with(|| crate::Cursor {
                query: props.query.clone(),
                selected: initial,
                drawn: None,
            });
        if cursor.query != props.query {
            cursor.query = props.query.clone();
            cursor.selected = initial;
        }
        let count = groups.iter().map(|group| group.rows.len()).sum::<usize>();
        cursor.selected = cursor.selected.min(count.saturating_sub(1));
        let rows = groups.iter().flat_map(|g| &g.rows).collect::<Vec<_>>();
        if rows
            .get(cursor.selected)
            .is_some_and(|row| matches!(row.target, crate::rows::Target::Inert))
        {
            cursor.selected = rows
                .iter()
                .position(|row| !matches!(row.target, crate::rows::Target::Inert))
                .unwrap_or(0);
        }
        let view = tree(
            &props,
            &groups,
            cursor.selected,
            &request.request_id,
            request.viewport.rows,
        );
        cursor.drawn = Some(std::sync::Arc::new(props));
        serde_json::to_value(view).map_err(crate::error)
    }
}
pub(crate) fn groups(props: &PaletteProps, recent: &[String]) -> (Vec<Group>, usize) {
    if let Some(args) = crate::args::arguments(&props.query, &props.commands, &props.catalogs) {
        return (
            vec![Group {
                name: args.title,
                rows: args.rows,
            }],
            args.selected,
        );
    }
    let query = props.query.strip_prefix('/').unwrap_or(&props.query);
    let mut groups = crate::rows::rows(
        props.open,
        query,
        &props.commands,
        &props.bindings,
        &props.sessions,
        recent,
        props.session.as_str(),
    );
    let conversation =
        crate::rows::conversation(&props.conversation, query, props.session.as_str());
    let at = if props.open == component::PaletteOpen::Sessions {
        0
    } else {
        groups
            .iter()
            .position(|g| g.name == "sessions")
            .unwrap_or(groups.len())
    };
    for group in &mut groups {
        if group.name == "sessions" {
            group.name = "stored sessions".into();
        }
    }
    if !conversation.is_empty()
        && (!query.is_empty() || props.open == component::PaletteOpen::Sessions)
    {
        groups.insert(
            at,
            Group {
                name: "this conversation".into(),
                rows: conversation,
            },
        );
    }
    all_sessions(&mut groups, props.open, query);
    crate::hints::append(&mut groups, props, query);
    let selected = if props.open == component::PaletteOpen::Sessions && query.is_empty() {
        groups.iter().flat_map(|g| &g.rows).position(|row| matches!(&row.target, crate::rows::Target::View(id) if id == props.session.as_str())).unwrap_or(0)
    } else {
        0
    };
    (groups, selected)
}
fn all_sessions(groups: &mut Vec<Group>, open: component::PaletteOpen, query: &str) {
    if open == component::PaletteOpen::Sessions
        || (!query.is_empty() && "all sessions".contains(&query.to_lowercase()))
    {
        let row = crate::rows::all_sessions();
        if let Some(group) = groups.iter_mut().find(|g| g.name == "stored sessions") {
            group.rows.push(row);
        } else {
            groups.push(Group {
                name: "stored sessions".into(),
                rows: vec![row],
            });
        }
    }
}

pub fn tree(
    props: &PaletteProps,
    groups: &[Group],
    selected: usize,
    request: &str,
    height: u16,
) -> View {
    let mut lines = Vec::new();
    let mut index = 0;
    let mut at = 0;
    for group in groups {
        lines.push(dim(format!("   {}", group.name)));
        for row in &group.rows {
            if index == selected {
                at = lines.len();
            }
            lines.push(row_view(row, index == selected));
            index += 1;
        }
    }
    if lines.is_empty() {
        lines.push(dim("   nothing matches in commands, sessions or keys"));
    }
    let room = usize::from(height).saturating_sub(1).max(1);
    let start = at
        .saturating_sub(room / 2)
        .min(lines.len().saturating_sub(room));
    let children = lines.into_iter().skip(start).take(room).collect::<Vec<_>>();
    View::Box {
        children,
        layout: Box::new(view::BoxStyle {
            hints: Some(chrome(props, groups)),
            on_key: Some(Action {
                name: "palette:ui".into(),
                args: json!({"handler":request,"event":"press","query":props.query}),
            }),
            ..Default::default()
        }),
    }
}
fn row_view(row: &Row, selected: bool) -> View {
    let inert = matches!(row.target, crate::rows::Target::Inert);
    let selected = selected && !inert;
    let label = View::Box {
        children: vec![marked(&row.label, &row.marks, inert)],
        layout: Box::new(view::BoxStyle {
            width: match row.target {
                crate::rows::Target::Command(_) => Some(14),
                crate::rows::Target::Key(_) | crate::rows::Target::Inert => Some(13),
                _ => None,
            },
            ..Default::default()
        }),
    };
    let mut children = vec![
        View::text(if selected { " ❯ " } else { "   " }),
        label,
        marked(
            &format!("  {}", row.hint),
            &row.hint_marks.iter().map(|i| i + 2).collect::<Vec<_>>(),
            true,
        ),
    ];
    if let Some(alert) = &row.alert {
        children.push(View::Text {
            children: vec![view::TextChild::String(format!(" · {alert}"))],
            style: view::TextStyle {
                color: Some("yellow".into()),
                ..Default::default()
            },
        });
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
            dim(format!("{}  ", row.now)),
        ],
        layout: Box::new(view::BoxStyle {
            flex_direction: Some(view::Direction::Row),
            justify_content: Some(view::Justify::SpaceBetween),
            height: Some(1),
            ..Default::default()
        }),
    }
}
fn dim(text: impl Into<String>) -> View {
    View::Text {
        children: vec![view::TextChild::String(text.into())],
        style: view::TextStyle {
            dim_color: Some(true),
            ..Default::default()
        },
    }
}

fn marked(text: &str, marks: &[usize], dim: bool) -> View {
    let mut runs: Vec<(String, bool)> = Vec::new();
    for (index, character) in text.chars().enumerate() {
        let bold = marks.contains(&index);
        if let Some((text, previous)) = runs.last_mut()
            && *previous == bold
        {
            text.push(character);
        } else {
            runs.push((character.to_string(), bold));
        }
    }
    View::Text {
        children: runs
            .into_iter()
            .map(|(text, bold)| {
                view::TextChild::Node(Box::new(View::Text {
                    children: vec![view::TextChild::String(text)],
                    style: view::TextStyle {
                        bold: Some(bold),
                        ..Default::default()
                    },
                }))
            })
            .collect(),
        style: view::TextStyle {
            dim_color: Some(dim),
            ..Default::default()
        },
    }
}

fn chrome(props: &PaletteProps, groups: &[Group]) -> component::SiteHints {
    use component::{KeyPair as Pair, PaletteOpen, SiteHints};
    let arguments = props.query.starts_with('/') && props.query.contains(' ');
    let empty = groups.iter().all(|group| group.rows.is_empty());
    if arguments
        && let Some(name) = props
            .query
            .strip_prefix('/')
            .and_then(|q| q.split_once(' '))
            .map(|(name, _)| name)
    {
        let free = props
            .commands
            .iter()
            .find(|entry| entry.id == name)
            .and_then(|entry| serde_json::from_value::<CommandSpec>(entry.meta.clone()).ok())
            .is_some_and(|spec| matches!(spec.args, ArgSpec::Free { .. }));
        if free {
            return SiteHints::new([Pair::new("⏎", name), Pair::new("esc", "cancel")])
                .composer("", "");
        }
    }

    let mut pairs = vec![Pair::new("↑↓", "pick")];
    pairs.push(Pair::new(
        "⏎",
        if empty {
            "nothing"
        } else if arguments {
            "choose"
        } else {
            match props.open {
                PaletteOpen::Commands => "choose",
                PaletteOpen::Keys => "do it",
                PaletteOpen::Sessions => "resume",
            }
        },
    ));
    if !arguments && !empty && props.open == PaletteOpen::Commands {
        pairs.push(Pair::new("tab", "fill in"));
    }
    pairs.push(Pair::new("esc", if arguments { "back" } else { "close" }));
    SiteHints::new(pairs).composer(
        match props.open {
            PaletteOpen::Commands => "",
            PaletteOpen::Keys => "? ",
            PaletteOpen::Sessions => "⟲ ",
        },
        "",
    )
}

#[cfg(test)]
mod chrome_tests {
    use super::*;
    #[test]
    fn palette_keeps_contextual_hints_out_of_the_body() {
        let props = crate::keys::review_tests::props();
        let (groups, selected) = groups(&props, &[]);
        bingo_sdk::testing::assert_separate_hints(&tree(&props, &groups, selected, "palette", 20));
    }
}
