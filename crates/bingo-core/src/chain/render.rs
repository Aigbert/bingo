//! Render answers pass through the shared chain and structural validation.
use super::*;
use bingo_sdk::render::RenderRequest;

struct Render;
impl Point for Render {
    type Input = RenderRequest;
    type Output = View;
    const POINT: LinkPoint = catalogue::UI_RENDER;
    fn payload(input: &RenderRequest) -> serde_json::Value {
        serde_json::to_value(input).unwrap_or_default()
    }
    fn pinned(before: &RenderRequest, after: &RenderRequest) -> bool {
        before.component == after.component
            && before.request_id == after.request_id
            && before.surface == after.surface
            && before.viewport == after.viewport
    }
    fn dispatch<'a>(
        link: &'a dyn Interceptor,
        cx: &'a HookContext,
        input: RenderRequest,
        next: Next<'a, RenderRequest, View>,
    ) -> BoxFuture<'a, Result<View, KernelError>> {
        Box::pin(async move {
            let component = input.component;
            let continued = Next::at(next.tier(), move |value| {
                Box::pin(async move {
                    let request = serde_json::from_value(value).map_err(invalid)?;
                    serde_json::to_value(next.run(request).await?).map_err(invalid)
                })
            });
            let value = link
                .chain(Self::POINT, cx, Self::payload(&input), continued)
                .await?;
            let view = serde_json::from_value(value).map_err(invalid)?;
            view::answer_ok(component, link.id(), &view).map_err(invalid)?;
            Ok(view)
        })
    }
}
fn invalid(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::InvalidInput, error.to_string())
}
struct Owner(Link);
#[async_trait::async_trait]
impl Interceptor for Owner {
    fn id(&self) -> &str {
        self.0
            .external
            .as_deref()
            .unwrap_or_else(|| self.0.interceptor.id())
    }
    fn matcher(&self) -> LinkMatcher {
        self.0.interceptor.matcher()
    }
    fn owns_deadline(&self) -> bool {
        self.0.interceptor.owns_deadline()
    }
    async fn chain(
        &self,
        entry: &catalogue::Entry,
        cx: &HookContext,
        input: serde_json::Value,
        next: Next<'_, serde_json::Value, serde_json::Value>,
    ) -> Result<serde_json::Value, KernelError> {
        self.0.interceptor.chain(entry, cx, input, next).await
    }
}

pub(crate) async fn dispatch(
    links: &[Link],
    cx: &HookContext,
    request: RenderRequest,
    report: Report,
) -> Result<View, KernelError> {
    let owned: Vec<_> = links
        .iter()
        .map(|link| Link {
            external: link.external.clone(),
            tier: link.tier,
            interceptor: Arc::new(Owner(link.clone())),
        })
        .collect();
    run::<Render>(
        &owned,
        Next::new(|request: RenderRequest| Box::pin(async move { Ok(request.engine()) })),
        cx,
        request,
        report,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    struct Answer(bool);
    #[async_trait::async_trait]
    impl Interceptor for Answer {
        fn id(&self) -> &str {
            "counter"
        }
        fn matcher(&self) -> LinkMatcher {
            LinkMatcher::at(catalogue::UI_RENDER)
        }
        async fn chain(
            &self,
            _: &catalogue::Entry,
            _: &HookContext,
            _: Value,
            _: Next<'_, Value, Value>,
        ) -> Result<Value, KernelError> {
            if self.0 {
                Err(KernelError::new(ErrorCode::Internal, "broken"))
            } else {
                Ok(json!({"kind":"Engine", "component":"ToolUse", "props":{}}))
            }
        }
    }
    #[tokio::test]
    async fn kernel_and_failed_hooks_return_the_requested_engine() {
        let request: RenderRequest = serde_json::from_value(json!({"component":"Pane",
            "requestId":"counter", "surface":"terminal", "props":{},
            "viewport":{"columns":80,"rows":8}}))
        .unwrap();
        let cx = HookContext {
            session: SessionId::from_raw("s"),
            turn: None,
            item: None,
            cwd: "/".into(),
            provider: None,
            model: None,
            host: testing::NoHost::handle(),
        };
        assert_eq!(
            dispatch(&[], &cx, request.clone(), Report::default())
                .await
                .unwrap(),
            request.engine()
        );
        for throws in [true, false] {
            let report = Report::default();
            let links = [Link {
                external: Some("counter".into()),
                interceptor: Arc::new(Answer(throws)),
                tier: Tier::Rewrite,
            }];
            assert_eq!(
                dispatch(&links, &cx, request.clone(), report.clone())
                    .await
                    .unwrap(),
                request.engine()
            );
            let notices = report.take();
            assert_eq!(notices.len(), 1);
            assert!(matches!(&notices[0], ItemBody::Notice {text,..} if text.contains("counter")));
        }
    }
}
