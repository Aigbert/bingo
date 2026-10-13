//! Render answers pass through the shared chain and structural validation.
use super::*;
use bingo_sdk::render::RenderRequest;

pub(crate) async fn dispatch(
    links: &[Link],
    cx: &HookContext,
    request: RenderRequest,
    report: Report,
) -> Result<View, KernelError> {
    super::json::dispatch(
        catalogue::UI_RENDER, links, cx, request,
        Next::new(|request: RenderRequest| Box::pin(async move { Ok(request.engine()) })), report,
    ).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    struct Answer(bool);
    #[async_trait::async_trait]
    impl Interceptor for Answer {
        fn id(&self) -> &str { "counter" }
        fn matcher(&self) -> LinkMatcher { LinkMatcher::at(catalogue::UI_RENDER) }
        async fn chain(&self, _: &catalogue::Entry, _: &HookContext, _: Value,
            _: Next<'_, Value, Value>) -> Result<Value, KernelError> {
            if self.0 { Err(KernelError::new(ErrorCode::Internal, "broken")) }
            else { Ok(json!({"kind":"Engine", "component":"ToolUse", "props":{}})) }
        }
    }
    #[tokio::test]
    async fn kernel_and_failed_hooks_return_the_requested_engine() {
        let request: RenderRequest = serde_json::from_value(json!({"component":"Pane",
            "requestId":"counter", "surface":"terminal", "props":{},
            "viewport":{"columns":80,"rows":8}})).unwrap();
        let cx = HookContext { session: SessionId::from_raw("s"), turn:None, item:None,
            cwd:"/".into(), provider:None, model:None, host:testing::NoHost::handle() };
        assert_eq!(dispatch(&[], &cx, request.clone(), Report::default()).await.unwrap(), request.engine());
        for throws in [true, false] {
            let report = Report::default();
            let links = [Link { external:Some("counter".into()), interceptor:Arc::new(Answer(throws)), tier:Tier::Rewrite }];
            assert_eq!(dispatch(&links, &cx, request.clone(), report.clone()).await.unwrap(), request.engine());
            let notices = report.take();
            assert_eq!(notices.len(), 1);
            assert!(matches!(&notices[0], ItemBody::Notice {text,..} if text.contains("counter")));
        }
    }
}
