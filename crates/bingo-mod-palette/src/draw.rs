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
        let mut props: PaletteProps =
            serde_json::from_value(request.props).map_err(crate::error)?;
        enrich(&mut props, &cx.host);
        let mut shared = self.0.lock().map_err(crate::error)?;
        let (groups, initial) = groups(&props, shared.recent.names());
        let cursor = shared
            .cursors
            .entry((cx.session.clone(), request.request_id.clone()))
            .or_default();
        if cursor.query != props.query {
            cursor.query = props.query.clone();
            cursor.selected = initial;
        }
        let count = groups.iter().map(|group| group.rows.len()).sum::<usize>();
        cursor.selected = cursor.selected.min(count.saturating_sub(1));
        serde_json::to_value(tree(
            &props,
            &groups,
            cursor.selected,
            &request.request_id,
            request.viewport.rows,
        ))
        .map_err(crate::error)
    }
}
fn enrich(props: &mut PaletteProps, host: &HostHandle) {
    if let Some(reader) = host.service(&command::COMMAND_CATALOG) {
        let mut state = SessionState::new(props.summary.clone());
        state.config = props.config.clone();
        state.context = props.context;
        state.signals = props.signals.clone();
        let providers = props
            .catalogs
            .get("providers")
            .into_iter()
            .flatten()
            .filter_map(|entry| serde_json::from_value(entry.meta["auth"].clone()).ok())
            .collect::<Vec<_>>();
        props.commands = reader
            .catalog(
                Catalog {
                    kind: CatalogKind::Commands,
                    entries: std::mem::take(&mut props.commands),
                },
                &command::SessionView {
                    state: &state,
                    providers: &providers,
                },
            )
            .entries;
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
        props.summary.id.as_str(),
    );
    crate::hints::append(&mut groups, props, query);
    (groups, 0)
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
    let mut children = lines.into_iter().skip(start).take(room).collect::<Vec<_>>();
    children.push(dim(
        if props.query.contains(' ') && props.query.starts_with('/') {
            "   ↑↓ choose · enter run · esc back"
        } else {
            "   type to search commands, sessions and keys together"
        },
    ));
    View::Box {
        children,
        layout: Box::new(view::BoxStyle {
            on_key: Some(Action {
                name: "palette:ui".into(),
                args: json!({"handler":request,"event":"press","props":props}),
            }),
            ..Default::default()
        }),
    }
}
fn row_view(row: &Row, selected: bool) -> View {
    let label = View::Box {
        children: vec![marked(&row.label, &row.marks, false)],
        layout: Box::new(view::BoxStyle {
            width: match row.target {
                crate::rows::Target::Command(_) => Some(14),
                crate::rows::Target::Key(_) => Some(13),
                _ => None,
            },
            ..Default::default()
        }),
    };
    let children = vec![
        View::text(if selected { " ❯ " } else { "   " }),
        label,
        marked(
            &format!("  {}", row.hint),
            &row.hint_marks.iter().map(|i| i + 2).collect::<Vec<_>>(),
            true,
        ),
    ];
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
