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
            prefilter: json!({"component":self.1}),
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
        let request: render::RenderRequest =
            serde_json::from_value(value.clone()).map_err(crate::error)?;
        if !request.surface.shows_sheets() {
            return next.run(value).await;
        }
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
            request.viewport.glyphs,
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
    glyphs: bingo_sdk::glyph::GlyphSet,
) -> View {
    use bingo_sdk::{pick, sheet::Sheet};
    let listed = crate::rows::rows(models, &picker.props.filter, picker.all);
    let at = crate::rows::cursor(&listed, picker.props.selected.as_deref());
    let rows = listed
        .iter()
        .enumerate()
        .map(|(i, row)| model_row(row, models, chat, at == Some(i)))
        .collect();
    let model = at
        .and_then(|i| listed[i].model())
        .and_then(|id| models.iter().find(|m| m.id == id));
    let mut sheet = Sheet {
        head: "Model".into(),
        fact: Some(pick::text(
            format!("{} {}", short(&chat.current), Effort::word(chat.effort)),
            "dim",
        )),
        top: vec![],
        rows,
        selected: at.map(|i| listed[i].key()),
        foot: model
            .map(|m| effort_scale(m, picker.props.effort))
            .unwrap_or_default(),
        pick: true,
    };
    if let Some(manage) = &picker.sheet {
        sheet = manage.build(providers);
    }
    let mut view = sheet.view(width, height, glyphs);
    if let View::Box { layout, .. } = &mut view {
        layout.on_key = Some(Action {
            name: "bingo.mod.models:ui".into(),
            args: json!({"handler":picker.id,"event":"press"}),
        });
        layout.hints = Some(component::SiteHints::new([
            component::KeyPair::new("↑↓", "model"),
            component::KeyPair::new("←→", "effort"),
            component::KeyPair::new("⏎", "use"),
            component::KeyPair::new("esc", "close"),
        ]));
    }
    bingo_sdk::sheet::clicks(view)
}
fn short(id: &str) -> &str {
    id.split_once('/').map_or(id, |(_, name)| name)
}
fn model_row(
    row: &Row,
    models: &[CatalogEntry],
    chat: &crate::card::Chat,
    selected: bool,
) -> pick::Row {
    use bingo_sdk::pick::{bold, line, text};
    let Some(model) = row
        .model()
        .and_then(|id| models.iter().find(|m| m.id == id))
    else {
        return pick::Row::new(row.key(), text("", "dim"));
    };
    let name = short(&model.id);
    let provider = model.meta["provider"].as_str().unwrap_or_default();
    let context = crate::card::known(&model.meta["context"])
        .map(crate::card::tokens)
        .unwrap_or_default();
    let images = match model.meta["images"].as_bool() {
        Some(true) => "reads images",
        Some(false) => "text only",
        None => "",
    };
    let left = line([
        if selected {
            pick::pad(bold(name, "fg"), 15)
        } else {
            pick::pad(text(name, "muted"), 15)
        },
        pick::line([
            pick::pad(text(provider, "dim"), 12),
            pick::pad(text(context, "dim"), 7),
            text(images, "dim"),
        ]),
    ]);
    let mut row = pick::Row::new(row.key(), left);
    if model.id == chat.current {
        row.right = Some(text("in use", "good"));
    }
    row
}
fn effort_scale(model: &CatalogEntry, effort: Option<Effort>) -> Vec<View> {
    use bingo_sdk::pick::{bold, line, text};
    let effort = crate::card::clamp(model, effort);
    let mut runs = vec![text("effort   ", "dim")];
    for (i, value) in crate::card::levels(model).iter().enumerate() {
        if i > 0 {
            runs.push(text("  ", "dim"));
        }
        runs.push(if *value == effort {
            bold(Effort::word(*value), "glow")
        } else {
            text(Effort::word(*value), "dim")
        });
    }
    vec![
        line(runs),
        text(effort_words(Effort::word(effort)), "muted"),
    ]
}
pub(crate) fn effort_words(effort: &str) -> &str {
    match effort {
        "off" => "answers without thinking",
        "minimal" => "barely thinks",
        "low" => "a short think on hard steps",
        "medium" => "thinks on most steps",
        "high" => "thinks before most answers",
        "xhigh" => "thinks long on hard problems",
        "max" => "thinks as long as it needs",
        _ => "",
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn models_use_the_shared_sheet_window_and_ground() {
        let models = (0..12)
            .map(|i| CatalogEntry {
                id: format!("p/model-{i}"),
                label: String::new(),
                meta: json!({"provider":"p"}),
            })
            .collect::<Vec<_>>();
        let picker = Picker {
            props: component::ModelPickerProps {
                selected: Some("model:p/model-11".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        let view = tree(
            &picker,
            &models,
            &[],
            &Default::default(),
            100,
            18,
            Default::default(),
        );
        assert!(view.fold().contains("8 of 12"));
        assert!(!view.fold().contains("model-0"));
        assert!(!view.fold().contains("text only"));
        assert!(
            serde_json::to_string(&view)
                .unwrap()
                .contains("\"ground\":\"pick\"")
        );
    }
}
