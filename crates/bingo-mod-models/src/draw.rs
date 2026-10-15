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
        let models = cx.host.catalog(CatalogKind::Models).await?.entries;
        let providers = cx.host.catalog(CatalogKind::Providers).await?.entries;
        let attached = crate::bridge::attach(&cx.host, &cx.session).await?;
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
    let mut children = if let Some(sheet) = &picker.sheet {
        crate::sheet::lines(sheet, providers)
            .into_iter()
            .map(dim)
            .collect()
    } else {
        list(picker, models, providers, chat, width, height)
    };
    if let Some(message) = &picker.message {
        children.push(dim(format!("   {message}")));
    }
    children.push(dim(if picker.sheet.is_some() {
        "   ↑↓ pick · ⏎ do it · esc back".into()
    } else {
        format!("   /model {}", picker.props.filter)
    }));
    View::Box {
        children,
        layout: Box::new(view::BoxStyle {
            on_key: Some(Action {
                name: "bingo.mod.models:ui".into(),
                args: json!({"handler":picker.id,"event":"press"}),
            }),
            ..Default::default()
        }),
    }
}
fn list(
    picker: &Picker,
    models: &[CatalogEntry],
    providers: &[CatalogEntry],
    chat: &crate::card::Chat,
    width: u16,
    height: u16,
) -> Vec<View> {
    let rows = crate::rows::rows(models, providers, &picker.props.filter, picker.all);
    let selected = picker.props.selected.min(rows.len().saturating_sub(1));
    let details = details(rows.get(selected), models, providers, chat, picker);
    let room = usize::from(height).saturating_sub(4).max(1);
    let start = selected
        .saturating_sub(room / 2)
        .min(rows.len().saturating_sub(room));
    let left_width = if width >= 100 {
        56
    } else {
        width.saturating_mul(2) / 5
    };
    let mut children = vec![dim(format!(
        "   {} models · {} providers    ★ pinned · ∴ thinks · ▣ sees images",
        models.len(),
        providers.len()
    ))];
    for i in 0..room {
        let left = if i + 1 == room && start + room < rows.len() && selected != start + i {
            format!("↓ {} more", rows.len() - start - room + 1)
        } else {
            rows.get(start + i)
                .map(|row| row_text(row, models, providers, chat))
                .unwrap_or_default()
        };
        let mark = if start + i == selected {
            " ❯ "
        } else {
            "   "
        };
        let left = cell(format!("{mark}{left}"), left_width, start + i == selected);
        let right = cell(
            format!(
                "│ {}",
                details.get(i).map(String::as_str).unwrap_or_default()
            ),
            width.saturating_sub(left_width),
            false,
        );
        children.push(View::Box {
            children: vec![left, right],
            layout: Box::new(view::BoxStyle {
                flex_direction: Some(view::Direction::Row),
                ..Default::default()
            }),
        });
    }
    let label = action_label(rows.get(selected), models, providers, chat, picker);
    children.push(key_hint(&label, picker.keep()));
    children
}
fn key_hint(label: &str, keep: bool) -> View {
    dim(format!(
        " ↑↓ pick  ←→ effort  * pin  tab {}  ⏎ {label}  esc close",
        if keep { "here only" } else { "keep" }
    ))
}
fn row_text(
    row: &Row,
    models: &[CatalogEntry],
    providers: &[CatalogEntry],
    chat: &crate::card::Chat,
) -> String {
    match row {
        Row::Heading => "pinned · alt+m cycles them".into(),
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
        Row::Model(id) => models
            .iter()
            .find(|m| &m.id == id)
            .map(|m| {
                format!(
                    "{} {} {} {} {}{}",
                    if m.meta["pinned"] == true { "★" } else { " " },
                    if id == &chat.current { "●" } else { " " },
                    id.split_once('/').map_or(id.as_str(), |(_, m)| m),
                    m.meta["context"]
                        .as_u64()
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
        Some(Row::Model(id)) => models
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
                    "⏎ test, sign in or manage".into(),
                ]
            })
            .unwrap_or_default(),
        Some(Row::Hidden(..)) => vec![
            "Hidden models".into(),
            "Snapshots and non-chat models".into(),
            "⏎ show them".into(),
        ],
        Some(Row::Add) => vec![
            "add a provider".into(),
            "paste a key or an address".into(),
            String::new(),
            "sk-ant-…   Anthropic".into(),
            "sk-or-…    OpenRouter".into(),
            "sk-…       OpenAI".into(),
            "localhost:11434   Ollama".into(),
            "localhost:1234    LM Studio".into(),
            "any http(s) address".into(),
        ],
        _ => vec!["No matching models".into()],
    }
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
    chat: &crate::card::Chat,
    picker: &Picker,
) -> String {
    match row {
        Some(Row::Model(id)) => models
            .iter()
            .find(|m| &m.id == id)
            .and_then(|m| {
                providers
                    .iter()
                    .find(|p| m.meta["provider"] == p.id)
                    .map(|p| {
                        crate::label::label(
                            m,
                            p,
                            if m.id == chat.current { 0 } else { chat.used },
                            picker.keep(),
                        )
                    })
            })
            .unwrap_or_default(),
        Some(Row::Provider(_)) => "manage".into(),
        Some(Row::Hidden(..)) => "show hidden".into(),
        Some(Row::Add) => "add".into(),
        _ => String::new(),
    }
}
