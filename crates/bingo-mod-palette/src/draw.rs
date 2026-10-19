use crate::{Shared, rows::Row};
use bingo_sdk::{
    component::{PaletteOpen, PaletteProps},
    *,
};
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
        let (cursor, listing) = shared.walk(&cx.session, &request.request_id, &props);
        let view = tree(
            &props,
            &listing,
            cursor.selected,
            &request.request_id,
            &request.viewport,
        );
        cursor.drawn = Some(std::sync::Arc::new(props));
        serde_json::to_value(view).map_err(crate::error)
    }
}

/// What one drawing lists: the rows, the row a fresh drawing starts on, and
/// the command whose arguments they are.
pub(crate) struct Listing {
    pub rows: Vec<Row>,
    pub initial: usize,
    pub title: Option<String>,
}

pub(crate) fn listing(props: &PaletteProps) -> Listing {
    if let Some(args) = crate::resume::arguments(props)
        .or_else(|| crate::args::arguments(&props.query, &props.commands, &props.catalogs))
    {
        return Listing {
            rows: args.rows,
            initial: args.selected,
            title: Some(args.title),
        };
    }
    let query = props.query.strip_prefix('/').unwrap_or(&props.query);
    if props.open == PaletteOpen::Sessions {
        return sessions(props, query);
    }
    Listing {
        rows: crate::rows::commands(&props.commands, &props.bindings, query),
        initial: 0,
        title: None,
    }
}

/// This conversation's sessions, then the stored ones, then the door to
/// every session; a fresh drawing rests on the session in view.
fn sessions(props: &PaletteProps, query: &str) -> Listing {
    let current = props.session.as_str();
    let rows = crate::rows::conversation(&props.conversation, query, current)
        .into_iter()
        .chain(
            props
                .sessions
                .iter()
                .map(|summary| crate::rows::session(summary, current))
                .filter_map(|row| crate::rows::filter(row, query)),
        )
        .chain(crate::rows::filter(crate::rows::all_sessions(), query))
        .collect::<Vec<_>>();
    let initial = rows
        .iter()
        .position(|row| matches!(&row.target, crate::rows::Target::View(id) if id == current))
        .unwrap_or(0);
    Listing {
        rows,
        initial,
        title: None,
    }
}

pub(crate) fn tree(
    props: &PaletteProps,
    listing: &Listing,
    selected: usize,
    request: &str,
    viewport: &render::Viewport,
) -> View {
    let columns = crate::line::Columns::of(&listing.rows, viewport.columns);
    let rows = listing
        .rows
        .iter()
        .enumerate()
        .map(|(at, row)| crate::line::row(row, at == selected, columns, viewport.glyphs))
        .collect::<Vec<_>>();
    let top = listing
        .title
        .iter()
        .map(|title| crate::line::note(title))
        .chain(rows.is_empty().then(|| crate::line::note(&empty(props))))
        .collect();
    let sheet = sheet::Sheet {
        head: String::new(),
        fact: None,
        top,
        selected: rows.get(selected).map(|row| row.id.clone()),
        rows,
        foot: Vec::new(),
        pick: true,
    };
    let mut view = sheet.list(
        viewport.columns,
        viewport.rows,
        LIST_MAX,
        sheet::Tally::Counted,
        viewport.glyphs,
    );
    if let View::Box { layout, .. } = &mut view {
        layout.hints = Some(chrome(props, listing));
        layout.on_key = Some(Action {
            name: "palette:ui".into(),
            args: json!({"handler":request,"event":"press","query":props.query}),
        });
    }
    view
}

/// The most rows the list shows at once.
const LIST_MAX: usize = 8;

/// What an empty list says.
fn empty(props: &PaletteProps) -> String {
    let query = props.query.trim_start_matches('/');
    match props.open {
        PaletteOpen::Sessions => format!("no session matches {query}"),
        _ => format!("no command matches {}", props.query.trim()),
    }
}

fn chrome(props: &PaletteProps, listing: &Listing) -> component::SiteHints {
    use component::{KeyPair as Pair, SiteHints};
    if listing.title.is_none() && props.open != PaletteOpen::Sessions {
        return SiteHints::new([
            Pair::new("↑↓", "choose"),
            Pair::new("⏎", "run"),
            Pair::new("tab", "complete"),
        ])
        .composer("", "");
    }
    if let Some(name) = free_argument(props) {
        return SiteHints::new([Pair::new("⏎", name), Pair::new("esc", "cancel")]).composer("", "");
    }
    let resume = props.open == PaletteOpen::Sessions && listing.title.is_none();
    SiteHints::new([
        Pair::new("↑↓", "choose"),
        Pair::new("⏎", if resume { "resume" } else { "choose" }),
        Pair::new("esc", if resume { "close" } else { "back" }),
    ])
    .composer(if resume { "⟲ " } else { "" }, "")
}

/// The command being typed, when what follows it is free text.
fn free_argument(props: &PaletteProps) -> Option<&str> {
    let (name, _) = props.query.strip_prefix('/')?.split_once(' ')?;
    props
        .commands
        .iter()
        .find(|entry| entry.id == name)
        .and_then(|entry| serde_json::from_value::<CommandSpec>(entry.meta.clone()).ok())
        .is_some_and(|spec| matches!(spec.args, ArgSpec::Free { .. }))
        .then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport(rows: u16) -> render::Viewport {
        render::Viewport {
            glyphs: Default::default(),
            place: None,
            columns: 100,
            rows,
        }
    }

    fn props(query: &str, names: &[&str]) -> PaletteProps {
        let mut props = crate::keys::review_tests::props();
        props.open = PaletteOpen::Commands;
        props.query = query.into();
        props.commands = names
            .iter()
            .map(|name| CatalogEntry {
                id: (*name).into(),
                label: (*name).into(),
                meta: json!({"name":name,"hint":format!("{name} hint"),"args":{"kind":"none"},"family":"session"}),
            })
            .collect();
        props
    }

    fn drawn(props: &PaletteProps, rows: u16) -> View {
        let listing = listing(props);
        tree(props, &listing, 0, "palette", &viewport(rows))
    }

    #[test]
    fn the_list_has_no_head_and_counts_what_the_window_leaves_out() {
        let names = [
            "model", "think", "compact", "status", "resume", "rewind", "agents", "tasks", "room",
            "team",
        ];
        let text = drawn(&props("/", &names), 22).fold();
        assert!(text.trim_start().starts_with("▎\n/model"), "{text}");
        assert!(text.contains("8 of 10"), "{text}");
        assert!(!text.contains("/room"), "{text}");
        let small = drawn(&props("/", &names), 6).fold();
        assert!(small.contains("3 of 10"), "{small}");
    }

    #[test]
    fn nothing_matching_says_so_and_keeps_the_keys() {
        let props = props("/zz", &["model"]);
        let view = drawn(&props, 22);
        assert!(view.fold().contains("no command matches /zz"));
        let View::Box { layout, .. } = &view else {
            panic!("a box")
        };
        let words = layout.hints.as_ref().map(|hints| {
            hints
                .pairs
                .iter()
                .map(|pair| pair.action.clone())
                .collect::<Vec<_>>()
        });
        assert_eq!(
            words,
            Some(vec!["choose".into(), "run".into(), "complete".into()])
        );
    }

    #[test]
    fn palette_keeps_contextual_hints_out_of_the_body() {
        bingo_sdk::testing::assert_separate_hints(&drawn(&props("/", &["model"]), 20));
    }
}
