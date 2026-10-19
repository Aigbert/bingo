//! A schedule switch confirms the command's success with the prompt it changed.
use bingo_sdk::*;
use serde_json::Value;
pub struct Receipts;
#[async_trait::async_trait]
impl Interceptor for Receipts {
    fn id(&self) -> &str {
        "schedules.receipts"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher::at(catalogue::COMMAND_RUN)
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        cx: &HookContext,
        value: Value,
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let switch = target(&value);
        let said = if let Some((verb, id)) = switch {
            crate::listing(&cx.host).await.ok().and_then(|listing| {
                listing
                    .entries
                    .into_iter()
                    .find(|row| row.id == id)
                    .map(|row| {
                        toast::Say::new([
                            String::from(verb),
                            bingo_services::schedule::visible(&row.text).into_owned(),
                        ])
                    })
            })
        } else {
            None
        };
        let result = next.run(value).await?;
        match said {
            Some(said) if result["kind"] == "applied" => {
                serde_json::to_value(said.outcome(said.plain())).map_err(crate::error)
            }
            _ => Ok(result),
        }
    }
}
fn target(value: &Value) -> Option<(&'static str, String)> {
    let run: catalogue::CommandRun = serde_json::from_value(value.clone()).ok()?;
    if !run.surface.is_some_and(render::SurfaceName::shows_sheets) || run.name != "schedule" {
        return None;
    }
    let mut words = run.args.split_whitespace();
    let verb = match words.next()? {
        "pause" => "paused",
        "resume" => "resumed",
        _ => return None,
    };
    let id = words.next()?;
    words.next().is_none().then_some((verb, id.to_owned()))
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn only_terminal_switches_replace_the_receipt() {
        assert_eq!(
            target(&json!({"command":"schedule","args":"pause abc","surface":"terminal"})),
            Some(("paused", "abc".into()))
        );
        assert_eq!(
            target(&json!({"command":"schedule","args":"resume abc","surface":"text"})),
            None
        );
        assert_eq!(
            target(&json!({"command":"schedule","args":"run abc","surface":"terminal"})),
            None
        );
    }
}
