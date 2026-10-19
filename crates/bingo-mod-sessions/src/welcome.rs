//! The welcome reads the directory service; the terminal retains only its drawing.
use bingo_sdk::*;
use serde_json::{Value, json};
pub(crate) struct Welcome;
#[async_trait::async_trait]
impl Interceptor for Welcome {
    fn id(&self) -> &str {
        "sessions.welcome"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"component":"Welcome"}),
            ..LinkMatcher::at(catalogue::UI_RENDER)
        }
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        cx: &HookContext,
        value: Value,
        _: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let request: render::RenderRequest = serde_json::from_value(value).map_err(crate::error)?;
        let mut props: component::WelcomeProps =
            serde_json::from_value(request.props).map_err(crate::error)?;
        let mut sessions = cx
            .host
            .sessions(SessionFilter {
                cwd: Some(props.session.cwd.clone().into()),
                parent: None,
                limit: None,
            })
            .await?;
        sessions.retain(|row| {
            row.id != props.session.id && row.parent.is_none() && row.driver == Driver::Model
        });
        sessions.sort_by_key(|row| std::cmp::Reverse(row.updated_at));
        sessions.truncate(3);
        props.recent = Some(sessions);
        serde_json::to_value(View::Engine {
            component: render::Component::Welcome,
            props: serde_json::to_value(props).map_err(crate::error)?,
        })
        .map_err(crate::error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn recent_reads_the_service_and_selects_three_roots_in_recency_order() {
        let host = testing::TestHost::default();
        host.on_sessions(|filter| async move {
            assert_eq!(filter.cwd, Some(std::path::PathBuf::from("/work")));
            Ok((0..5)
                .map(|n| {
                    let mut row = crate::tests::summary(&format!("s{n}"), Some("past"));
                    row.updated_at += jiff::SignedDuration::from_secs(n);
                    row
                })
                .collect())
        });
        let cx = HookContext {
            session: SessionId::from_raw("current"),
            turn: None,
            item: None,
            cwd: "/work".into(),
            provider: None,
            model: None,
            host: host.handle(),
        };
        let props = json!({"session":crate::tests::summary("current", None),"effort":null,"modes":[],"update":null,"resume":null});
        let value = json!({"component":"Welcome","requestId":"Welcome","surface":"terminal","props":props,"viewport":{"columns":100,"rows":28}});
        let next = Next::new(|_| Box::pin(async { panic!("the built-in answers") }));
        let view = Welcome
            .chain(catalogue::UI_RENDER, &cx, value, next)
            .await
            .unwrap();
        let ids: Vec<_> = view["props"]["recent"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, ["s4", "s3", "s2"]);
    }
}
