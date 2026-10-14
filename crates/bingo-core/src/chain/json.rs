//! Catalogue dispatch uses the same fail-open continuation as typed links.
use super::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

#[derive(Clone, PartialEq)]
struct Input {
    entry: Arc<catalogue::Entry>,
    value: Value,
}
struct Json;
impl Point for Json {
    type Input = Input;
    type Output = Value;
    const POINT: LinkPoint = catalogue::TURN_COMPLETE;
    fn entry(input: &Self::Input) -> &catalogue::Entry {
        &input.entry
    }
    fn payload(input: &Self::Input) -> Value {
        input.value.clone()
    }
    fn dispatch<'a>(
        link: &'a dyn Interceptor,
        cx: &'a HookContext,
        input: Self::Input,
        next: Next<'a, Self::Input, Value>,
    ) -> BoxFuture<'a, Result<Value, KernelError>> {
        let entry = input.entry;
        let continued_entry = entry.clone();
        let result_identity = input.value.get("name").cloned();
        let original = input.value.clone();
        let result_pins = original.clone();
        let next = Next::at(next.tier(), move |value| {
            let valid = (continued_entry.valid_input)(&value)
                && (continued_entry.as_ref() != catalogue::PLUGIN_REGISTER
                    || super::registration::values(&original, &value));
            let pinned = catalogue::pinned_changed(&original, &value, continued_entry.pinned);
            Box::pin(async move {
                if let Some(field) = pinned {
                    return Err(invalid(&format!("pinned field {field}")));
                }
                if !valid {
                    return Err(invalid(&continued_entry.name));
                }
                next.run(Input {
                    entry: continued_entry,
                    value,
                })
                .await
            })
        });
        Box::pin(async move {
            let result = link.chain(&entry, cx, input.value, next).await?;
            let describes =
                [catalogue::TOOL_DESCRIBE, catalogue::COMMAND_DESCRIBE].contains(&entry.as_ref());
            if describes && !result.is_null() && result.get("name") != result_identity.as_ref() {
                return Err(invalid("pinned field /name"));
            }
            if [
                catalogue::SESSION_APPEND,
                catalogue::CONFIG_SET,
                catalogue::PROMPT_AUTOCOMPLETE,
            ]
            .contains(&entry.as_ref())
                && let Some(field) = catalogue::pinned_changed(&result_pins, &result, entry.pinned)
            {
                return Err(invalid(&format!("pinned field {field}")));
            }
            if entry.as_ref() == catalogue::PLUGIN_REGISTER
                && !super::registration::values(&result_pins, &result)
            {
                return Err(invalid("plugin.register: grants may only be reduced"));
            }
            if !(entry.valid_result)(&result) {
                return Err(invalid(&entry.name));
            }
            Ok(result)
        })
    }
    fn pinned(before: &Self::Input, after: &Self::Input) -> bool {
        before.entry == after.entry
            && catalogue::pinned_changed(&before.value, &after.value, before.entry.pinned).is_none()
    }
}
fn invalid(name: &str) -> KernelError {
    KernelError::new(
        ErrorCode::InvalidInput,
        format!("{name}: invalid shape or pinned field"),
    )
}

pub(crate) async fn dispatch<I, O>(
    entry: &catalogue::Entry,
    links: &[Link],
    cx: &HookContext,
    input: I,
    next: Next<'_, I, O>,
    report: Report,
) -> Result<O, KernelError>
where
    I: Serialize + DeserializeOwned + Send,
    O: Serialize + DeserializeOwned + Send,
{
    let value = serde_json::to_value(input).map_err(|_| invalid(&entry.name))?;
    let kernel = Next::new(move |input: Input| {
        Box::pin(async move {
            let input = serde_json::from_value(input.value).map_err(|_| invalid(&entry.name))?;
            serde_json::to_value(next.run(input).await?).map_err(|_| invalid(&entry.name))
        })
    });
    let value = run::<Json>(
        links,
        kernel,
        cx,
        Input {
            entry: Arc::new(entry.clone()),
            value,
        },
        report,
    )
    .await?;
    serde_json::from_value(value).map_err(|_| invalid(&entry.name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    struct Answer;
    #[async_trait::async_trait]
    impl Interceptor for Answer {
        fn id(&self) -> &str {
            "answer"
        }
        fn matcher(&self) -> LinkMatcher {
            LinkMatcher::at(catalogue::TURN_COMPLETE)
        }
        async fn chain(
            &self,
            _: &catalogue::Entry,
            _: &HookContext,
            input: Value,
            next: Next<'_, Value, Value>,
        ) -> Result<Value, KernelError> {
            let below = next.run(input).await?;
            Ok(json!(format!(
                "{} outer",
                below.as_str().unwrap_or_default()
            )))
        }
    }
    fn cx() -> HookContext {
        HookContext {
            session: SessionId::from_raw("s"),
            turn: None,
            item: None,
            cwd: "/".into(),
            provider: None,
            model: None,
            host: testing::NoHost::handle(),
        }
    }
    #[tokio::test]
    async fn completion_wraps_the_text_from_beneath() {
        let links = [Link {
            placement: bingo_sdk::plugin::Placement::Outer,
            external: None,
            interceptor: Arc::new(Answer),
            tier: Tier::Rewrite,
        }];
        let input = catalogue::Turn {
            turn: TurnId::from_raw("t"),
            outcome: TurnStatus::Completed,
            usage: Usage::default(),
            answer: "answer".into(),
        };
        let result: Option<String> = dispatch(
            catalogue::TURN_COMPLETE,
            &links,
            &cx(),
            input,
            Next::new(|_| Box::pin(async { Ok(Some("inner".to_string())) })),
            Report::default(),
        )
        .await
        .unwrap();
        assert_eq!(result.as_deref(), Some("inner outer"));
    }
}

#[cfg(test)]
mod contracts {
    use super::*;
    use serde_json::json;
    struct Rewrite(&'static str, Value);
    #[async_trait::async_trait]
    impl Interceptor for Rewrite {
        fn id(&self) -> &str {
            "rewrite"
        }
        fn matcher(&self) -> LinkMatcher {
            LinkMatcher::default()
        }
        async fn chain(
            &self,
            _: &catalogue::Entry,
            _: &HookContext,
            mut input: Value,
            next: Next<'_, Value, Value>,
        ) -> Result<Value, KernelError> {
            if self.0 == "answer" {
                return Ok(self.1.clone());
            }
            input[self.0] = self.1.clone();
            next.run(input).await
        }
    }
    fn cx() -> HookContext {
        HookContext {
            session: SessionId::from_raw("s"),
            turn: None,
            item: None,
            cwd: "/".into(),
            provider: None,
            model: None,
            host: testing::NoHost::handle(),
        }
    }
    fn link(key: &'static str, value: Value, tier: Tier) -> Vec<Link> {
        vec![Link {
            placement: bingo_sdk::plugin::Placement::Outer,
            interceptor: Arc::new(Rewrite(key, value)),
            external: None,
            tier,
        }]
    }
    #[tokio::test]
    async fn descriptions_and_received_text_are_rewritable() {
        let links = link("description", json!("new"), Tier::Rewrite);
        let spec: ToolSpec =
            serde_json::from_value(json!({"name":"Read", "description":"old", "inputSchema":{}}))
                .unwrap();
        let described = crate::chain::describe::tools(vec![spec], &links, &cx(), Report::default())
            .await
            .unwrap();
        assert_eq!(described[0].description, "new");
        let links = link("text", json!("new"), Tier::Rewrite);
        let input = bingo_sdk::Input::text("old", Origin::surface("test"));
        let received = crate::chain::deliver::receive(input, &links, &cx(), Report::default())
            .await
            .unwrap();
        assert!(matches!(received, bingo_sdk::Input::Text { text, .. } if text == "new"));
    }
    #[tokio::test]
    async fn pinned_changes_and_tighten_answers_are_skipped_once() {
        let links = link("name", json!("Other"), Tier::Full);
        let spec: ToolSpec =
            serde_json::from_value(json!({"name":"Read", "description":"old", "inputSchema":{}}))
                .unwrap();
        let report = Report::default();
        let described = crate::chain::describe::tools(vec![spec], &links, &cx(), report.clone())
            .await
            .unwrap();
        assert_eq!(described[0].name, "Read");
        let notices = report.take();
        assert_eq!(notices.len(), 1);
        assert!(
            matches!(&notices[0], ItemBody::Notice { text, .. } if text.contains("/name") && text.contains("rewrite") && text.contains("tool.describe"))
        );
        let links = link(
            "answer",
            serde_json::to_value(CommandOutcome::Applied {
                message: Some("fake".into()),
            })
            .unwrap(),
            Tier::Tighten,
        );
        let report = Report::default();
        let answer: CommandOutcome = dispatch(
            catalogue::COMMAND_RUN,
            &links,
            &cx(),
            catalogue::CommandRun {
                name: "test".into(),
                args: "".into(),
            },
            Next::new(|_| {
                Box::pin(async {
                    Ok(CommandOutcome::Applied {
                        message: Some("kernel".into()),
                    })
                })
            }),
            report.clone(),
        )
        .await
        .unwrap();
        assert!(
            matches!(answer, CommandOutcome::Applied { message: Some(message) } if message == "kernel")
        );
        assert_eq!(report.take().len(), 1);
    }
}
