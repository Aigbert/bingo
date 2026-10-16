//! The replaceable owner of the focused site's interaction chrome.
mod draw;
use bingo_sdk::*;
use serde_json::{Value, json};
use std::sync::Arc;

pub struct KeyBar;
static MANIFEST: PluginManifest = PluginManifest {
    id: "bingo.mod.keybar",
    version: env!("CARGO_PKG_VERSION"),
    uses: &[],
    requires: &[],
    provides: &[],
    config: None,
};
impl Plugin for KeyBar {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }
    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        registrar.add(Contribution::Interceptor {
            placement: plugin::Placement::Builtin,
            external: None,
            link: Arc::new(Drawing),
        });
        Ok(())
    }
}
struct Drawing;
#[async_trait::async_trait]
impl Interceptor for Drawing {
    fn id(&self) -> &str {
        "keybar"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"component":"KeyBar"}),
            ..LinkMatcher::at(catalogue::UI_RENDER)
        }
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        _: &HookContext,
        value: Value,
        _: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let request: render::RenderRequest = serde_json::from_value(value).map_err(error)?;
        let props = serde_json::from_value(request.props).map_err(error)?;
        serde_json::to_value(draw::tree(&props, usize::from(request.viewport.columns)))
            .map_err(error)
    }
}
fn error(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::Internal, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn an_outside_mod_wraps_the_builtin_with_next() {
        let mut registrar = Registrar::new(
            "bingo.mod.keybar",
            json!({}),
            Env::rooted("/unused"),
            Default::default(),
        );
        KeyBar.register(&mut registrar).unwrap();
        let link = registrar
            .into_contributions()
            .into_iter()
            .find_map(|c| match c {
                Contribution::Interceptor {
                    placement: plugin::Placement::Builtin,
                    link,
                    ..
                } => Some(link),
                _ => None,
            })
            .expect("the built-in is innermost");
        let entry = catalogue::UI_RENDER;
        let cx = HookContext {
            session: SessionId::from_raw("session"),
            turn: None,
            item: None,
            cwd: "/unused".into(),
            provider: None,
            model: None,
            host: testing::NoHost::handle(),
        };
        let request = json!({"component":"KeyBar","requestId":"KeyBar","surface":"terminal","props":{"pairs":[{"keys":"esc","action":"close","emphasis":false}],"toast":null},"viewport":{"columns":80,"rows":1}});
        let next = Next::new(|value| {
            Box::pin(async move {
                link.chain(
                    entry,
                    &cx,
                    value,
                    Next::new(|_| Box::pin(async { panic!("the owner answers") })),
                )
                .await
            })
        });
        let builtin: View = serde_json::from_value(next.run(request).await.unwrap()).unwrap();
        assert!(builtin.fold().contains("esc close"));
        let outside = View::column(vec![View::text("outside"), builtin]);
        assert!(outside.fold().contains("outside"));
        assert!(outside.fold().contains("esc close"));
    }
}
