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
            && (!before.component.sealed() || before.props == after.props)
    }
    fn dispatch<'a>(
        link: &'a dyn Interceptor,
        cx: &'a HookContext,
        input: RenderRequest,
        next: Next<'a, RenderRequest, View>,
    ) -> BoxFuture<'a, Result<View, KernelError>> {
        Box::pin(async move {
            // A hook may pass on another mod's answer; its handlers stay valid.
            let inherited = Arc::new(Mutex::new(None));
            let answered = inherited.clone();
            let continued = Next::at(next.tier(), move |value| {
                Box::pin(async move {
                    let request = serde_json::from_value(value).map_err(invalid)?;
                    let view = next.run(request).await?;
                    if let Ok(mut answered) = answered.lock() {
                        *answered = Some(view.clone());
                    }
                    serde_json::to_value(view).map_err(invalid)
                })
            });
            let value = link
                .chain(Self::POINT, cx, Self::payload(&input), continued)
                .await?;
            let view = serde_json::from_value(value).map_err(invalid)?;
            let inherited = inherited.lock().ok().and_then(|view| view.clone());
            view::answer_ok(
                input.component,
                &input.props,
                link.id(),
                inherited.as_ref(),
                &view,
            )
            .map_err(invalid)?;
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
            plugin: "fixture".into(),
            placement: link.placement,
            external: link.external.clone(),
            tier: link.tier,
            interceptor: Arc::new(Owner(link.clone())),
        })
        .collect();
    let checked = Report::default();
    let fallback = request.component.sealed().then(|| request.engine());
    let result = run::<Render>(
        &owned,
        Next::new(|request: RenderRequest| Box::pin(async move { Ok(request.engine()) })),
        cx,
        request,
        checked.clone(),
    )
    .await;
    let notices = checked.take();
    let failed = !notices.is_empty();
    for notice in notices {
        report.push(notice);
    }
    if failed && let Some(engine) = fallback {
        return Ok(engine);
    }
    result
}

/// The config is a projection of the currently admitted links, never a registry.
pub(crate) fn matched(links: &[Link]) -> bingo_sdk::render::Matched {
    let mut matched = bingo_sdk::render::Matched::default();
    for link in links {
        let matcher = link.interceptor.matcher();
        if !matcher.patterns.is_empty()
            && !matcher
                .patterns
                .iter()
                .any(|pattern| pattern.matches("ui.render"))
        {
            continue;
        }
        let partials = matcher.render.unwrap_or_else(|| vec![matcher.prefilter]);
        for component in bingo_sdk::render::Component::ALL {
            let event = serde_json::json!({"component":component});
            for partial in &partials {
                if partial.is_null() || catalogue::prefilter(partial, &event) {
                    matched
                        .0
                        .entry(component)
                        .or_default()
                        .push(partial.clone());
                }
            }
        }
    }
    matched
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    #[test]
    fn only_ui_render_links_publish_components() {
        let link = Link {
            plugin: "fixture".into(),
            placement: bingo_sdk::plugin::Placement::Outer,
            external: None,
            interceptor: Arc::new(Answer(false)),
            tier: Tier::Rewrite,
        };
        let exposed = matched(&[link]);
        assert_eq!(exposed.0.len(), bingo_sdk::render::Component::ALL.len());
        assert!(matched(&[]).0.is_empty());
    }
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
                plugin: "fixture".into(),
                placement: bingo_sdk::plugin::Placement::Outer,
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
    struct Pass;
    #[async_trait::async_trait]
    impl Interceptor for Pass {
        fn id(&self) -> &str {
            "corner"
        }
        fn matcher(&self) -> LinkMatcher {
            LinkMatcher::at(catalogue::UI_RENDER)
        }
        async fn chain(
            &self,
            _: &catalogue::Entry,
            _: &HookContext,
            input: Value,
            next: Next<'_, Value, Value>,
        ) -> Result<Value, KernelError> {
            next.run(input).await
        }
    }
    struct Press;
    #[async_trait::async_trait]
    impl Interceptor for Press {
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
            Ok(json!({"kind":"Button","onPress":{"name":"counter:ui",
                "args":{"handler":"1:1","event":"press"}}}))
        }
    }
    #[tokio::test]
    async fn a_hook_passing_another_mods_answer_keeps_its_handlers() {
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
        let link = |id: &str, interceptor: Arc<dyn Interceptor>| Link {
            plugin: "fixture".into(),
            placement: bingo_sdk::plugin::Placement::Outer,
            external: Some(id.into()),
            interceptor,
            tier: Tier::Rewrite,
        };
        for links in [
            [
                link("corner", Arc::new(Pass)),
                link("counter", Arc::new(Press)),
            ],
            [
                link("counter", Arc::new(Press)),
                link("corner", Arc::new(Pass)),
            ],
        ] {
            let report = Report::default();
            let view = dispatch(&links, &cx, request.clone(), report.clone())
                .await
                .unwrap();
            assert!(matches!(view, View::Button { .. }), "{view:?}");
            assert!(report.take().is_empty());
        }
    }
}

#[cfg(test)]
mod sealed_tests;
