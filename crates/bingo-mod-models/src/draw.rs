use crate::{rows::Row, state::Picker};
use bingo_sdk::*;
use serde_json::{Value, json};
pub(crate) struct Drawing(pub crate::Shared, pub render::Component);
#[async_trait::async_trait]
impl Interceptor for Drawing {
    fn id(&self) -> &str {
        match self.1 {
            render::Component::ModelPicker => "bingo.mod.models",
            _ => "models.notice",
        }
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"component":self.1,"surface":"terminal"}),
            ..LinkMatcher::at(catalogue::UI_RENDER)
        }
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        cx: &HookContext,
        value: Value,
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let component = value["component"].as_str().unwrap_or_default();
        if component == "InfoNotice"
            && let Some(text) = value["props"]["text"]
                .as_str()
                .filter(|s| s.starts_with('⇄'))
        {
            return serde_json::to_value(outcome(text)).map_err(crate::error);
        }
        if component != "ModelPicker" {
            return next.run(value).await;
        }
        let request: render::RenderRequest =
            serde_json::from_value(value.clone()).map_err(crate::error)?;
        let picker = self
            .0
            .lock()
            .map_err(crate::error)?
            .get(&cx.session)
            .cloned();
        let Some(picker) = picker.filter(|p| p.id == request.request_id) else {
            return next.run(value).await;
        };
        let attached = crate::bridge::attach(&cx.host, &cx.session).await?;
        let models = crate::state::models(&cx.host, &attached.snapshot).await?;
        let providers = cx.host.catalog(CatalogKind::Providers).await?.entries;
        serde_json::to_value(tree(
            &picker,
            &models,
            &providers,
            &crate::state::chat(&attached.snapshot),
            request.viewport.columns,
            request.viewport.rows,
        ))
        .map_err(crate::error)
    }
}
pub fn tree(
    picker: &Picker,
    models: &[CatalogEntry],
    providers: &[CatalogEntry],
    chat: &crate::card::Chat,
    width: u16,
    height: u16,
) -> View {
    let rows = crate::rows::rows(models, providers, &picker.props.filter, picker.all);
    let at = crate::rows::cursor(&rows, picker.props.selected.as_deref());
    let children = if let Some(sheet) = &picker.sheet {
        crate::sheet::lines(sheet, providers)
            .into_iter()
            .map(dim)
            .collect()
    } else {
        let list = List {
            rows: &rows,
            at,
            models,
            providers,
            chat,
        };
        list.draw(picker, width, height)
    };
    View::Box {
        children,
        layout: Box::new(view::BoxStyle {
            hints: Some(chrome(picker, &rows, at, models, providers, chat)),
            on_key: Some(on_key(picker)),
            ..Default::default()
        }),
    }
}
/// A key names only itself: it acts on the row the picker keeps selected,
/// which a key queued behind a redraw has already moved.
fn on_key(picker: &Picker) -> Action {
    Action {
        name: "bingo.mod.models:ui".into(),
        args: json!({"handler":picker.id,"event":"press"}),
    }
}
struct List<'a> {
    rows: &'a [Row],
    at: Option<usize>,
    models: &'a [CatalogEntry],
    providers: &'a [CatalogEntry],
    chat: &'a crate::card::Chat,
}
impl List<'_> {
    fn draw(&self, picker: &Picker, width: u16, height: u16) -> Vec<View> {
        let unlisted = crate::rows::unlisted(self.rows, &picker.props.filter, &self.chat.current);
        let row = self.at.map(|i| &self.rows[i]);
        let details = match &unlisted {
            Some(id) => self::unlisted(id),
            None => details(row, self.models, self.providers, self.chat, picker),
        };
        let left_width = if width >= 100 {
            56
        } else {
            width.saturating_mul(2) / 5
        };
        let mut children = vec![dim(format!(
            "   {}    ★ pinned · ∴ thinks · ▣ sees images",
            header(self.rows, &picker.props.filter, self.providers.len())
        ))];
        let room = usize::from(height).saturating_sub(4).max(1);
        children.extend((0..room).map(|i| {
            self.line(
                i,
                room,
                &details,
                left_width,
                width.saturating_sub(left_width),
            )
        }));
        children
    }
    /// One visible line: a keyed row on the left, a line of the card on the right.
    fn line(&self, i: usize, room: usize, details: &[String], left: u16, right: u16) -> View {
        let selected = self.at.unwrap_or(0);
        let start = selected
            .saturating_sub(room / 2)
            .min(self.rows.len().saturating_sub(room));
        let index = start + i;
        let more = i + 1 == room && start + room < self.rows.len() && selected != index;
        let (text, key) = match self.rows.get(index) {
            _ if more => (format!("↓ {} more", self.rows.len() - index), None),
            Some(row) => (
                row_text(row, self.models, self.providers, self.chat),
                Some(row.key()),
            ),
            None => (String::new(), None),
        };
        let chosen = self.at == Some(index);
        let mark = if chosen { " ❯ " } else { "   " };
        let detail = details.get(i).map(String::as_str).unwrap_or_default();
        View::Box {
            children: vec![
                cell(format!("{mark}{text}"), left, chosen),
                cell(format!("│ {detail}"), right, false),
            ],
            layout: Box::new(view::BoxStyle {
                flex_direction: Some(view::Direction::Row),
                key,
                ..Default::default()
            }),
        }
    }
}
/// What the list holds now: matches for a filter, else models and providers.
pub fn header(rows: &[Row], filter: &str, providers: usize) -> String {
    let models = rows.iter().filter(|r| matches!(r, Row::Model(_))).count();
    if filter.is_empty() {
        format!("{models} models · {providers} providers")
    } else {
        let matches = if models == 1 { "match" } else { "matches" };
        format!("{models} {matches} for “{filter}”")
    }
}
fn row_text(
    row: &Row,
    models: &[CatalogEntry],
    providers: &[CatalogEntry],
    chat: &crate::card::Chat,
) -> String {
    match row {
        Row::Heading => "pinned".into(),
        Row::Provider(id) => format!(
            "{id}  {}",
            providers
                .iter()
                .find(|p| &p.id == id)
                .map(crate::label::status)
                .unwrap_or_default()
        ),
        Row::Hidden(_, count) => format!("   · {count} hidden"),
        Row::Add => "+ add a provider".into(),
        Row::Pinned(id) | Row::Model(id) => models
            .iter()
            .find(|m| &m.id == id)
            .map(|m| {
                format!(
                    "{} {} {} {} {}{}",
                    if m.meta["pinned"] == true { "★" } else { " " },
                    if id == &chat.current { "●" } else { " " },
                    id.split_once('/').map_or(id.as_str(), |(_, m)| m),
                    crate::card::known(&m.meta["context"])
                        .map(crate::card::tokens)
                        .unwrap_or_else(|| "—".into()),
                    if m.meta["reasoning"] == true {
                        "∴"
                    } else {
                        " "
                    },
                    if m.meta["images"] == true { "▣" } else { " " }
                )
            })
            .unwrap_or_default(),
    }
}
fn details(
    row: Option<&Row>,
    models: &[CatalogEntry],
    providers: &[CatalogEntry],
    chat: &crate::card::Chat,
    picker: &Picker,
) -> Vec<String> {
    match row {
        Some(Row::Pinned(id) | Row::Model(id)) => models
            .iter()
            .find(|m| &m.id == id)
            .map(|m| crate::card::card(m, chat, picker.props.effort, picker.keep()))
            .unwrap_or_default(),
        Some(Row::Provider(id)) => providers
            .iter()
            .find(|p| &p.id == id)
            .map(|p| {
                vec![
                    id.clone(),
                    String::new(),
                    format!(
                        "address   {}",
                        p.meta["address"].as_str().unwrap_or("provider managed")
                    ),
                    format!(
                        "key       {}",
                        p.meta["auth"]["from"].as_str().unwrap_or("none")
                    ),
                    crate::label::status(p),
                    String::new(),
                    "test, sign in or manage".into(),
                ]
            })
            .unwrap_or_default(),
        Some(Row::Hidden(..)) => vec![
            "Hidden models".into(),
            "Snapshots and non-chat models".into(),
            "hidden models".into(),
        ],
        Some(Row::Add) => vec![
            "add a provider".into(),
            "paste a key or an address".into(),
            String::new(),
            "sk-ant-…   Anthropic".into(),
            "sk-or-…    OpenRouter".into(),
            "sk-…       paste its address next".into(),
            "localhost:11434   Ollama".into(),
            "localhost:1234    LM Studio".into(),
            "any http(s) address".into(),
        ],
        _ => vec!["No matching models".into()],
    }
}
fn unlisted(id: &str) -> Vec<String> {
    vec![
        "No matching models".into(),
        String::new(),
        format!("use {id}"),
        "the provider decides whether it exists".into(),
    ]
}
fn cell(text: String, width: u16, selected: bool) -> View {
    View::Box {
        children: vec![View::Text {
            children: vec![view::TextChild::String(text)],
            style: view::TextStyle {
                bold: Some(selected),
                dim_color: Some(!selected),
                wrap: Some("truncate".into()),
                ..Default::default()
            },
        }],
        layout: Box::new(view::BoxStyle {
            width: Some(width),
            ..Default::default()
        }),
    }
}
fn dim(text: String) -> View {
    View::Text {
        children: vec![view::TextChild::String(text)],
        style: view::TextStyle {
            dim_color: Some(true),
            ..Default::default()
        },
    }
}
pub fn outcome(text: &str) -> View {
    View::Text {
        children: vec![
            view::TextChild::Node(Box::new(View::Text {
                children: vec![view::TextChild::String("⇄".into())],
                style: view::TextStyle {
                    color: Some("attention".into()),
                    dim_color: Some(false),
                    ..Default::default()
                },
            })),
            view::TextChild::String(text.trim_start_matches('⇄').into()),
        ],
        style: view::TextStyle {
            dim_color: Some(true),
            ..Default::default()
        },
    }
}

fn action_label(
    row: Option<&Row>,
    models: &[CatalogEntry],
    providers: &[CatalogEntry],
    picker: &Picker,
) -> String {
    match row {
        Some(Row::Pinned(id) | Row::Model(id)) => models
            .iter()
            .find(|m| &m.id == id)
            .and_then(|m| {
                providers
                    .iter()
                    .find(|p| m.meta["provider"] == p.id)
                    .map(|p| crate::label::label(m, p, picker.keep()))
            })
            .unwrap_or_default(),
        Some(Row::Provider(_)) => "manage".into(),
        Some(Row::Hidden(..)) => "show hidden".into(),
        Some(Row::Add) => "add".into(),
        _ => String::new(),
    }
}

fn chrome(
    picker: &Picker,
    rows: &[Row],
    at: Option<usize>,
    models: &[CatalogEntry],
    providers: &[CatalogEntry],
    chat: &crate::card::Chat,
) -> component::SiteHints {
    use component::{KeyPair as Pair, SiteHints};
    if let Some(sheet) = &picker.sheet {
        return sheet_hints(sheet);
    }
    let row = at.and_then(|i| rows.get(i));
    let mut pairs = vec![Pair::new("↑↓", "pick")];
    if let Some(id) = row.and_then(Row::model) {
        let pinned = models
            .iter()
            .any(|m| m.id == id && m.meta["pinned"] == true);
        pairs.extend([
            Pair::new("←→", "thinking"),
            Pair::new("*", if pinned { "unpin" } else { "pin" }),
            Pair::new("tab", if picker.keep() { "here only" } else { "keep" }),
        ]);
    } else {
        pairs.push(Pair::new("+", "add provider"));
    }
    pairs.extend([
        Pair::new(
            "⏎",
            crate::rows::unlisted(rows, &picker.props.filter, &chat.current).map_or_else(
                || action_label(row, models, providers, picker),
                |id| format!("use {id}"),
            ),
        ),
        Pair::new("esc", "close"),
    ]);
    SiteHints::new(pairs).composer(format!("/model {}", picker.props.filter), "")
}

fn sheet_hints(sheet: &crate::sheet::Sheet) -> component::SiteHints {
    use component::{KeyPair as Pair, SiteHints};
    let pairs = match sheet {
        crate::sheet::Sheet::Add { selected, .. } => vec![
            Pair::new("↑↓", "field"),
            Pair::new(
                "⏎",
                if *selected == 4 {
                    "check and add"
                } else {
                    "edit"
                },
            ),
            Pair::new("esc", "back"),
        ],
        _ => vec![
            Pair::new("↑↓", "pick"),
            Pair::new("⏎", "do it"),
            Pair::new("esc", "back"),
        ],
    };
    SiteHints::new(pairs).composer(format!("/model › {}", sheet.title()), "")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_header_counts_what_the_list_holds() {
        let rows = [
            Row::Heading,
            Row::Pinned("fake/a".into()),
            Row::Provider("fake".into()),
            Row::Model("fake/a".into()),
            Row::Model("fake/b".into()),
        ];
        assert_eq!(header(&rows, "", 3), "2 models · 3 providers");
        assert_eq!(header(&rows, "a", 3), "2 matches for “a”");
        assert_eq!(header(&rows[..4], "a", 3), "1 match for “a”");
    }
    #[test]
    fn a_window_nobody_reported_is_a_dash() {
        let models = [CatalogEntry {
            id: "openai/chatgpt-image-latest".into(),
            label: String::new(),
            meta: json!({"context": 0}),
        }];
        let text = row_text(
            &Row::Model(models[0].id.clone()),
            &models,
            &[],
            &crate::card::Chat::default(),
        );
        assert!(text.contains("chatgpt-image-latest —"), "{text}");
    }
}
