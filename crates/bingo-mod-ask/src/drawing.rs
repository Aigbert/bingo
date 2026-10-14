use crate::{
    States,
    state::{Field, State},
};
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
            .get(&(cx.session.clone(), id.to_string()))
            .cloned()
            .unwrap_or_default();
        serde_json::to_value(tree(&props, &state, &request.request_id, &cx.session)).map_err(error)
    }
}
fn error(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::Internal, error.to_string())
}

pub fn tree(
    props: &component::QuestionProps,
    state: &State,
    request: &str,
    session: &SessionId,
) -> View {
    let mut children = Vec::new();
    for (index, question) in props.questions.iter().enumerate() {
        let field = state.fields.get(index).cloned().unwrap_or_default();
        if props.form
            && crate::state::fields(props.questions.len(), props.metadata_source.is_some())
        {
            children.push(View::text(form_row(
                question,
                &field,
                index == state.cursor,
            )));
        } else {
            children.extend(choice_rows(question, &field));
        }
    }
    View::Box {
        children,
        layout: Box::new(view::BoxStyle {
            on_key: Some(Action {
                name: "ask:ui".into(),
                args: json!({"handler":props.interaction_id,"event":"press","request":request,"session":session}),
            }),
            ..Default::default()
        }),
    }
}
fn choice_rows(question: &Question, field: &Field) -> Vec<View> {
    let mut rows = vec![View::text(format!("     {}", question.question))];
    for (i, option) in question.options.iter().enumerate() {
        let cursor = if i == field.selected {
            "   ❯ "
        } else {
            "     "
        };
        let mark = if question.multi {
            if field.ticks.contains(&option.id) {
                "[x] ".into()
            } else {
                "[ ] ".into()
            }
        } else {
            format!("{}  ", i + 1)
        };
        rows.push(View::text(format!("{cursor}{mark}{}", option.label)));
        if let Some(description) = &option.description {
            rows.push(View::text(format!("        {description}")));
        }
    }
    if question.free_text {
        let cursor = if field.selected == question.options.len() {
            "   ❯ "
        } else {
            "     "
        };
        rows.push(View::text(format!(
            "{cursor}{}  {}",
            question.options.len() + 1,
            field.words.as_deref().unwrap_or("something else…")
        )));
    }
    rows
}
fn form_row(question: &Question, field: &Field, focused: bool) -> String {
    let cursor = if focused { "   ❯ " } else { "     " };
    let label = question.header.as_deref().unwrap_or(&question.question);
    let value = if let Some(words) = &field.words {
        words.clone()
    } else if question.multi {
        question
            .options
            .iter()
            .map(|opt| {
                format!(
                    "[{}] {}",
                    if field.ticks.contains(&opt.id) {
                        "x"
                    } else {
                        " "
                    },
                    opt.label
                )
            })
            .collect::<Vec<_>>()
            .join("  ")
    } else {
        question
            .options
            .get(field.selected)
            .map(|opt| format!("‹ {} ›", opt.label))
            .unwrap_or_default()
    };
    format!("{cursor}{label:<13} {value}")
}
