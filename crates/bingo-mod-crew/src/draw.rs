//! The crew sheet (`SHEETS.crew`), on the anatomy every sheet shares.
use crate::state::Entry;
use bingo_sdk::{pick, sheet::Sheet, *};
use serde_json::{Value, json};
pub struct Drawing(pub crate::Shared);
#[async_trait::async_trait]
impl Interceptor for Drawing {
    fn id(&self) -> &str {
        crate::ID
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"component":"Pane","requestId":crate::REQUEST}),
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
        let crew = crate::read::read(&cx.host, &cx.session).await?;
        if let Some(watching) = self.0.lock().map_err(crate::error)?.get_mut(&cx.session) {
            *watching = crew.watching;
        }
        let view = tree(
            &crew.entries,
            request.props["selected"].as_str(),
            now_ms(),
            &request.viewport,
        );
        serde_json::to_value(view).map_err(crate::error)
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_millis()).unwrap_or(i64::MAX)
        })
}

pub fn tree(
    entries: &[Entry],
    selected: Option<&str>,
    now_ms: i64,
    viewport: &render::Viewport,
) -> View {
    let glyphs = viewport.glyphs;
    let at = selected
        .and_then(|id| entries.iter().position(|entry| entry.id().as_str() == id))
        .unwrap_or(0);
    let picked = entries.get(at);
    let rows = entries
        .iter()
        .enumerate()
        .map(|(i, entry)| crate::look::row(entry, i == at, now_ms, glyphs))
        .collect();
    let mut view = Sheet {
        head: "Crew".into(),
        fact: Some(pick::text(
            crate::state::fact(entries, glyph::Glyph::Sep.spell(glyphs)),
            "dim",
        )),
        top: if entries.is_empty() {
            vec![pick::text("no teammates or rooms yet", "dim")]
        } else {
            vec![]
        },
        rows,
        selected: picked.map(|entry| entry.id().to_string()),
        foot: vec![],
        pick: true,
    }
    .view(viewport.columns, viewport.rows, glyphs);
    if let View::Box { layout, .. } = &mut view {
        layout.hints = Some(component::SiteHints::new(hints(picked)));
        layout.on_key = Some(Action {
            name: CommandSpec::ui_name(crate::ID),
            args: json!({
                "handler": crate::REQUEST,
                "event": "press",
                "selected": picked.map(|entry| entry.id().to_string()),
            }),
        });
    }
    sheet::clicks(view)
}

fn hints(picked: Option<&Entry>) -> Vec<component::KeyPair> {
    use component::KeyPair as Pair;
    let mut pairs = vec![Pair::new("↑↓", "who"), Pair::new("⏎", "open")];
    if picked.and_then(Entry::mate).is_some() {
        pairs.push(Pair::new("tab", "mention"));
    }
    pairs.push(Pair::new("esc", "close"));
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{
        Need, ordered,
        tests::{mate, place},
    };

    fn viewport(columns: u16, rows: u16) -> render::Viewport {
        render::Viewport {
            glyphs: Default::default(),
            place: None,
            columns,
            rows,
        }
    }

    fn crew() -> Vec<Entry> {
        ordered(
            vec![mate("ada", Need::Idle), mate("scout", Need::Working)],
            vec![place("design")],
        )
    }

    #[test]
    fn the_sheet_heads_crew_with_its_fact_and_answers_the_render_contract() {
        let view = tree(&crew(), None, 0, &viewport(100, 18));
        view::answer_ok(render::Component::Pane, &json!({}), crate::ID, None, &view).unwrap();
        let folded = view.fold();
        assert!(
            folded.contains("Crew") && folded.contains("1 working"),
            "{folded}"
        );
        assert!(folded.find("scout").unwrap() < folded.find("ada").unwrap());
        assert!(folded.find("ada").unwrap() < folded.find("#design").unwrap());
        bingo_sdk::testing::assert_separate_hints(&view);
    }

    #[test]
    fn tab_is_offered_on_a_teammate_and_not_on_a_room() {
        let words = |selected| {
            let View::Box { layout, .. } = tree(&crew(), selected, 0, &viewport(100, 18)) else {
                panic!("a sheet is a box");
            };
            serde_json::to_string(&layout.hints).unwrap()
        };
        assert!(words(Some("ses_ada")).contains("mention"));
        assert!(!words(Some("ses_design")).contains("mention"));
    }

    #[test]
    fn an_empty_crew_says_so_and_the_press_carries_no_pick() {
        let view = tree(&[], None, 0, &viewport(80, 12));
        assert!(view.fold().contains("all idle"));
        assert!(view.fold().contains("no teammates or rooms yet"));
        let View::Box { layout, .. } = view else {
            panic!("a sheet is a box");
        };
        assert!(layout.on_key.unwrap().args["selected"].is_null());
    }
}
