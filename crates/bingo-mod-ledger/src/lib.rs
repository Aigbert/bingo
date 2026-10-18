//! The replaceable owner of a turn's step rows and its folded line.
pub mod draw;
use bingo_sdk::*;
use serde_json::{Value, json};
use std::sync::Arc;

pub struct Ledger;
static MANIFEST: PluginManifest = PluginManifest {
    id: "bingo.mod.ledger",
    version: env!("CARGO_PKG_VERSION"),
    uses: &[],
    requires: &[],
    provides: &[],
    config: None,
};
impl Plugin for Ledger {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }
    fn register(&self, registrar: &mut Registrar) -> Result<(), PluginError> {
        for site in [Site::Ledger, Site::TurnFold] {
            registrar.add(Contribution::Interceptor {
                placement: plugin::Placement::Builtin,
                external: None,
                link: Arc::new(site),
            });
        }
        Ok(())
    }
}

/// One link per site: a prefilter matches one component.
#[derive(Clone, Copy)]
enum Site {
    Ledger,
    TurnFold,
}
#[async_trait::async_trait]
impl Interceptor for Site {
    fn id(&self) -> &str {
        match self {
            Site::Ledger => "ledger",
            Site::TurnFold => "turn-fold",
        }
    }
    fn matcher(&self) -> LinkMatcher {
        let component = match self {
            Site::Ledger => render::Component::Ledger,
            Site::TurnFold => render::Component::TurnFold,
        };
        LinkMatcher {
            prefilter: json!({ "component": component }),
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
        let width = usize::from(request.viewport.columns);
        let view = match self {
            Site::Ledger => draw::ledger(
                &serde_json::from_value(request.props).map_err(error)?,
                width,
            ),
            Site::TurnFold => draw::fold(
                &serde_json::from_value(request.props).map_err(error)?,
                width,
            ),
        };
        serde_json::to_value(view).map_err(error)
    }
}
fn error(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::Internal, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn links() -> Vec<Arc<dyn Interceptor>> {
        let mut registrar = Registrar::new(
            "bingo.mod.ledger",
            json!({}),
            Env::rooted("/unused"),
            Default::default(),
        );
        Ledger.register(&mut registrar).unwrap();
        registrar
            .into_contributions()
            .into_iter()
            .filter_map(|c| match c {
                Contribution::Interceptor {
                    placement: plugin::Placement::Builtin,
                    link,
                    ..
                } => Some(link),
                _ => None,
            })
            .collect()
    }

    async fn drawn(link: &Arc<dyn Interceptor>, request: Value) -> View {
        let cx = HookContext {
            session: SessionId::from_raw("session"),
            turn: None,
            item: None,
            cwd: "/unused".into(),
            provider: None,
            model: None,
            host: testing::NoHost::handle(),
        };
        let last = Next::new(|_| Box::pin(async { panic!("the owner answers") }));
        let answer = link
            .chain(catalogue::UI_RENDER, &cx, request, last)
            .await
            .unwrap();
        serde_json::from_value(answer).unwrap()
    }

    #[tokio::test]
    async fn the_builtin_owns_both_sites() {
        let links = links();
        assert_eq!(links.len(), 2);
        let fold = json!({"component":"TurnFold","requestId":"itm_1","surface":"terminal","props":{"steps":[{"mark":"¶","reads":"path","tier":"lookup","verb":"read","target":"a","state":"done"}],"outcome":"done","elapsed":2000,"marks":{"done":"✓","asked":"◆","running":"◌","failed":"✗","folded":"▸","open":"▾"}},"viewport":{"columns":60,"rows":1}});
        let row = json!({"component":"Ledger","requestId":"itm_1","surface":"terminal","props":{"steps":[{"mark":"¶","reads":"path","tier":"lookup","verb":"read","target":"a","state":"done"}],"marks":{"done":"✓","asked":"◆","running":"◌","failed":"✗","folded":"▸","open":"▾"}},"viewport":{"columns":60,"rows":1}});
        let matched = |link: &Arc<dyn Interceptor>, request: &Value| {
            link.matcher().prefilter["component"] == request["component"]
        };
        let fold_link = links.iter().find(|l| matched(l, &fold)).expect("fold link");
        let row_link = links.iter().find(|l| matched(l, &row)).expect("row link");
        assert!(
            drawn(fold_link, fold)
                .await
                .fold()
                .starts_with("   ▸ 1 step · read")
        );
        assert!(drawn(row_link, row).await.fold().starts_with("   ✓ read a"));
    }
}
