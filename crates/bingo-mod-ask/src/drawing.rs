use crate::{States, rows, state::State};
use bingo_sdk::*;
use serde_json::{Value, json};

pub struct Drawing(pub States);
#[async_trait::async_trait]
impl Interceptor for Drawing {
    fn id(&self) -> &str {
        "ask"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"component":"Question"}),
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
            serde_json::from_value(value.clone()).map_err(error)?;
        let props: component::QuestionProps =
            serde_json::from_value(request.props.clone()).map_err(error)?;
        let Some(id) = &props.interaction_id else {
            return next.run(value).await;
        };
        let state = self
            .0
            .lock()
            .map_err(error)?
            .entry((cx.session.clone(), id.to_string()))
            .or_default()
            .clone();
        let asked = Asked {
            request: &request.request_id,
            session: &cx.session,
            columns: request.viewport.columns,
            glyphs: request.viewport.glyphs,
        };
        serde_json::to_value(tree(&props, &state, &asked)).map_err(error)
    }
}
fn error(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::Internal, error.to_string())
}

/// The request a question is drawn for and the dock it is drawn in.
pub struct Asked<'a> {
    pub request: &'a str,
    pub session: &'a SessionId,
    pub columns: u16,
    pub glyphs: glyph::GlyphSet,
}

pub fn tree(props: &component::QuestionProps, state: &State, asked: &Asked<'_>) -> View {
    let sheet = rows::Sheet {
        questions: &props.questions,
        state,
        columns: asked.columns,
        glyphs: asked.glyphs,
    };
    let field = state.field(state.tab);
    // A click on a choice is the same as walking to it and pressing Enter.
    sheet::clicks(View::Box {
        children: rows::rows(&sheet),
        layout: Box::new(view::BoxStyle {
            question: Some(component::QuestionFocus {
                field: state.tab,
                checked: field.ticks.len(),
                writing: state.words.is_some(),
            }),
            on_key: Some(Action {
                name: "ask:ui".into(),
                args: json!({"handler":props.interaction_id,"event":"press","request":asked.request,"session":asked.session,"props":props}),
            }),
            ..Default::default()
        }),
    })
}

#[cfg(test)]
mod chrome_tests {
    use super::*;
    #[test]
    fn questions_and_forms_leave_their_keys_to_the_sealed_host() {
        for form in [false, true] {
            let props = component::QuestionProps { interaction_id: Some(InteractionId::from_raw("ask")), form, tool: "ask".into(), questions: vec![serde_json::from_value(json!({"question":"Choose","options":[{"id":"a","label":"Alpha"}],"multi":true})).unwrap()], metadata_source: None };
            let session = SessionId::from_raw("session");
            let asked = Asked {
                request: "ask",
                session: &session,
                columns: 100,
                glyphs: Default::default(),
            };
            let view = tree(&props, &State::default(), &asked);
            for key in ["↑↓", "⏎", "esc", "tab next"] {
                assert!(!view.fold().contains(key));
            }
        }
    }
}
