use crate::{Shared, rows::Group};
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
        let (cursor, groups) = shared.walk(&cx.session, &request.request_id, &props);
        let view = tree(
            &props,
            &groups,
            cursor.selected,
            &request.request_id,
            &request.viewport,
        );
        cursor.drawn = Some(std::sync::Arc::new(props));
        serde_json::to_value(view).map_err(crate::error)
    }
}
pub(crate) fn groups(
    props: &PaletteProps,
    recent: &[String],
    expanded: &std::collections::BTreeSet<String>,
) -> (Vec<Group>, usize) {
    if let Some(args) = crate::resume::arguments(props)
        .or_else(|| crate::args::arguments(&props.query, &props.commands, &props.catalogs))
    {
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
    crate::skills::fold(&mut groups, query, expanded);
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
        if let Some(group) = groups.iter_mut().find(|g| g.name == "sessions") {
            group.rows.push(row);
        } else {
            groups.push(Group {
                name: "sessions".into(),
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
    viewport: &render::Viewport,
) -> View {
    let columns = crate::line::Columns::of(groups, usize::from(viewport.columns));
    let mut lines = Vec::new();
    let mut index = 0;
    let mut at = 0;
    for group in groups {
        lines.push(crate::line::heading(&group.name));
        for row in &group.rows {
            if index == selected {
                at = lines.len();
            }
            lines.push(crate::line::row(row, index == selected, columns));
            index += 1;
        }
    }
    if lines.is_empty() {
        let query = props.query.trim_start_matches('/');
        lines.push(crate::line::note(&format!(
            "nothing matches “{query}” in commands, skills, sessions or keys"
        )));
    }
    let footer = footer(props);
    let room = usize::from(viewport.rows)
        .saturating_sub(usize::from(footer.is_some()))
        .max(1);
    let start = at
        .saturating_sub(room / 2)
        .min(lines.len().saturating_sub(room));
    let children = lines
        .into_iter()
        .skip(start)
        .take(room)
        .chain(footer)
        .collect::<Vec<_>>();
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

/// What a bare `/` says under its list: that typing searches it all.
fn footer(props: &PaletteProps) -> Option<View> {
    (props.open == component::PaletteOpen::Commands
        && props.query.trim_start_matches('/').is_empty())
    .then(|| crate::line::note("type to search commands, skills, sessions and keys together"))
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
                PaletteOpen::Commands => "run",
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
        let (groups, selected) = groups(&props, &[], &Default::default());
        let viewport = render::Viewport {
            place: None,
            columns: 80,
            rows: 20,
        };
        bingo_sdk::testing::assert_separate_hints(&tree(
            &props, &groups, selected, "palette", &viewport,
        ));
    }
}
