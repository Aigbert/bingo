use crate::{Shared, rows::Target};
use bingo_sdk::{component::SessionsProps, *};
use serde_json::{Value, json};
pub(crate) struct Open;
#[async_trait::async_trait]
impl Command for Open {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            opens: false,
            name: "sessions".into(),
            aliases: vec![],
            hint: "find every session".into(),
            args: ArgSpec::None,
            instant: true,
            family: "terminal".into(),
            hidden: false,
            keys: vec![],
        }
    }
    async fn run(&self, _: &str, _: &CommandContext) -> Result<CommandOutcome, KernelError> {
        Ok(CommandOutcome::Surface {
            action: Action {
                name: "Sessions".into(),
                args: json!({}),
            },
            fallback: Some("/sessions is terminal only.".into()),
        })
    }
}
pub(crate) struct Handler(pub Shared);
#[async_trait::async_trait]
impl Command for Handler {
    fn spec(&self) -> CommandSpec {
        CommandSpec::ui("sessions")
    }
    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let args: Value = serde_json::from_str(args).map_err(crate::error)?;
        if args["event"] != "press" {
            return Ok(CommandOutcome::Handled { handled: false });
        }
        let props: SessionsProps =
            serde_json::from_value(args["props"].clone()).map_err(crate::error)?;
        let request = args["handler"].as_str().unwrap_or_default();
        let Some(press) = sheet::pressed(&args) else {
            return Ok(CommandOutcome::Handled { handled: false });
        };
        let Some(value) = self.press(request, &press, &props)? else {
            return Ok(CommandOutcome::Handled { handled: false });
        };
        cx.host
            .invalidate(&cx.session, vec![request.into()])
            .await?;
        let action = props
            .action
            .with_value(value)
            .ok_or_else(|| crate::error(Action::UNVALUED))?;
        Ok(CommandOutcome::Surface {
            action,
            fallback: None,
        })
    }
}
impl Handler {
    fn press(
        &self,
        request: &str,
        press: &sheet::Press,
        props: &SessionsProps,
    ) -> Result<Option<Value>, KernelError> {
        let key = press.key().unwrap_or("enter");
        if press.picked().is_none() && !matches!(key, "up" | "down" | "enter" | "esc" | "ctrl+c") {
            return Ok(None);
        }
        let mut state = self.0.lock().map_err(crate::error)?;
        let identity = (props.session.clone(), request.into());
        if matches!(key, "esc" | "ctrl+c") {
            state.cursors.remove(&identity);
            return Ok(Some(json!({"close":true})));
        }
        let cursor = state.cursors.entry(identity.clone()).or_default();
        cursor.sync(&props.query);
        let rows = crate::rows::listed(props, cursor);
        cursor.settle(&rows);
        let key = if let Some(id) = press.picked() {
            let target = Target::Session(SessionId::from_raw(id));
            if !rows.iter().any(|row| row.target == target) {
                return Ok(Some(json!({})));
            }
            cursor.selected = Some(target);
            "enter"
        } else {
            key
        };
        let at = rows
            .iter()
            .position(|row| Some(&row.target) == cursor.selected.as_ref());
        if key == "enter" {
            let value = match at.map(|i| &rows[i].target) {
                Some(Target::Session(id)) => json!({"resume":id}),
                _ => json!({}),
            };
            if value.get("resume").is_some() {
                state.cursors.remove(&identity);
            }
            return Ok(Some(value));
        }
        let picks = rows
            .iter()
            .map(|row| pick::Row::new(crate::draw::identity(&row.target), pick::text("", "fg")))
            .collect::<Vec<_>>();
        let selected = cursor.selected.as_ref().map(crate::draw::identity);
        let moved = pick::moved(&picks, selected.as_deref(), key == "down");
        cursor.selected = rows
            .iter()
            .find(|row| Some(crate::draw::identity(&row.target)) == moved)
            .map(|row| row.target.clone());
        Ok(Some(json!({})))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refresh_keeps_identity_and_letters_filter_instead_of_acting() {
        let shared = Shared::default();
        let handler = Handler(shared.clone());
        let mut props = crate::tests::props();
        props.sessions = vec![
            crate::tests::summary("a", None),
            crate::tests::summary("b", None),
        ];
        handler
            .press("list", &sheet::Press::Key("down".into()), &props)
            .unwrap();
        props.sessions.reverse();
        assert_eq!(
            handler
                .press("list", &sheet::Press::Key("r".into()), &props)
                .unwrap(),
            None
        );
        assert_eq!(
            handler
                .press("list", &sheet::Press::Key("enter".into()), &props)
                .unwrap(),
            Some(json!({"resume":"b"}))
        );
        handler
            .press("list", &sheet::Press::Key("down".into()), &props)
            .unwrap();
        props.sessions.clear();
        assert_eq!(
            handler
                .press("list", &sheet::Press::Key("enter".into()), &props)
                .unwrap(),
            Some(json!({}))
        );
        assert_eq!(
            handler
                .press("list", &sheet::Press::Key("esc".into()), &props)
                .unwrap(),
            Some(json!({"close":true}))
        );
    }
    #[test]
    fn a_new_filter_starts_the_cursor_at_the_best_match() {
        let handler = Handler(Shared::default());
        let mut props = crate::tests::props();
        props.sessions = vec![
            crate::tests::summary("a", Some("ci timeout")),
            crate::tests::summary("b", Some("cargo")),
        ];
        handler
            .press("list", &sheet::Press::Key("down".into()), &props)
            .unwrap();
        props.query = "c".into();
        assert_eq!(
            handler
                .press("list", &sheet::Press::Key("enter".into()), &props)
                .unwrap(),
            Some(json!({"resume":"a"}))
        );
    }
}
