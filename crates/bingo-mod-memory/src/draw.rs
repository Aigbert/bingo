use crate::{
    Shared,
    rows::{Group, Target},
};
use bingo_sdk::*;
use serde_json::{Value, json};

pub(crate) struct Drawing(pub Shared);
#[async_trait::async_trait]
impl Interceptor for Drawing {
    fn id(&self) -> &str {
        crate::ID
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"component":"Pane","props":{"owner":crate::ID}}),
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
        if request.props["owner"] != crate::ID {
            return next.run(value).await;
        }
        let shared = self.0.lock().map_err(crate::error)?;
        let Some(cursor) = shared.get(&cx.session) else {
            return serde_json::to_value(dim("")).map_err(crate::error);
        };
        serde_json::to_value(tree(
            &cursor.groups,
            cursor.selected.as_deref(),
            request.viewport.rows,
            request.viewport.columns,
            request.viewport.glyphs,
        ))
        .map_err(crate::error)
    }
}
pub fn tree(
    groups: &[Group],
    selected: Option<&str>,
    height: u16,
    width: u16,
    glyphs: bingo_sdk::glyph::GlyphSet,
) -> View {
    use bingo_sdk::{pick, sheet::Sheet};
    let all = groups.iter().flat_map(|g| &g.rows).collect::<Vec<_>>();
    let selected = all
        .iter()
        .position(|r| Some(r.target.id().as_str()) == selected)
        .unwrap_or(0);
    let target = all.get(selected).map(|r| &r.target);
    let count = all
        .iter()
        .filter(|r| matches!(r.target, Target::Memory { .. }))
        .count();
    let rows = all
        .iter()
        .enumerate()
        .map(|(i, r)| row_view(r, i == selected))
        .collect();
    let mut view = Sheet {
        head: "Memory".into(),
        fact: Some(pick::text(format!("{count} memories"), "dim")),
        top: vec![],
        rows,
        selected: target.map(Target::id),
        foot: vec![],
        pick: true,
    }
    .view(width, height, glyphs);
    if let View::Box { layout, .. } = &mut view {
        let mut pairs = vec![component::KeyPair::new("↑↓", "item")];
        if matches!(target, Some(Target::Memory { .. })) {
            pairs.push(component::KeyPair::new("⌫", "forget"));
        }
        pairs.push(component::KeyPair::new("esc", "close"));
        layout.hints = Some(component::SiteHints::new(pairs));
        layout.on_key = Some(Action {
            name: CommandSpec::ui_name(crate::ID),
            args: json!({"handler":crate::REQUEST,"event":"press","target":target}),
        });
    }
    bingo_sdk::sheet::clicks(view)
}
fn row_view(r: &crate::rows::Row, selected: bool) -> pick::Row {
    let kind = match r.target {
        Target::Instruction(_) => "instructions",
        Target::Memory { .. } => "memory",
        Target::Experience(_) => "experience",
    };
    let name = if selected {
        pick::bold(&r.label, "fg")
    } else {
        pick::text(&r.label, "muted")
    };
    let row = pick::Row::new(
        r.target.id(),
        pick::line([pick::pad(pick::text(kind, "dim"), 14), name]),
    );
    if r.note.is_empty() {
        row
    } else {
        row.right(pick::text(&r.note, "dim"))
    }
}
fn dim(text: impl Into<String>) -> View {
    pick::text(text, "dim")
}
