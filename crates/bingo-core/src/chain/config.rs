//! Config descriptions are shaped before clients see them.
use super::*;
use serde_json::Value;

pub(crate) async fn describe(
    config: ConfigView,
    links: &[Link],
    cx: &HookContext,
    report: Report,
) -> Result<ConfigView, KernelError> {
    json::dispatch(
        catalogue::CONFIG_DESCRIBE,
        links,
        cx,
        config,
        Next::new(|config| Box::pin(async { Ok(config) })),
        report,
    )
    .await
}

/// The policy a session runs under, and what it says of itself, are the
/// kernel's words (ADR-0009 §5): a link may describe every other slice of
/// the config, but neither name another policy nor speak for this one.
pub(super) fn kept(entry: &catalogue::Entry, before: &Value, after: &Value) -> bool {
    if entry != catalogue::CONFIG_DESCRIBE {
        return true;
    }
    let policy = |config: &Value| config["kernel"][ConfigView::POLICY].clone();
    let id = policy(before);
    let slice = |config: &Value| match id.as_str() {
        Some(id) => config["plugins"].get(id).cloned(),
        None => None,
    };
    policy(after) == id && slice(after) == slice(before)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A Rewrite-tier link that would rename the policy, or speak for it,
    /// on the way in or on the way out.
    struct Relabel {
        pointer: &'static str,
        value: Value,
        outbound: bool,
    }
    #[async_trait::async_trait]
    impl Interceptor for Relabel {
        fn id(&self) -> &str {
            "relabel"
        }
        fn matcher(&self) -> LinkMatcher {
            LinkMatcher::at(catalogue::CONFIG_DESCRIBE)
        }
        async fn chain(
            &self,
            _: &catalogue::Entry,
            _: &HookContext,
            mut input: Value,
            next: Next<'_, Value, Value>,
        ) -> Result<Value, KernelError> {
            if !self.outbound {
                set(&mut input, self.pointer, self.value.clone());
            }
            let mut output = next.run(input).await?;
            if self.outbound {
                set(&mut output, self.pointer, self.value.clone());
            }
            Ok(output)
        }
    }
    fn set(config: &mut Value, pointer: &str, value: Value) {
        if let Some(slot) = config.pointer_mut(pointer) {
            *slot = value;
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
    fn mode(label: &str) -> Value {
        json!({"slots":[{"kind":"mode","current":label,
            "modes":[{"label":label,"action":{"name":"permission","args":label}}]}]})
    }
    fn given() -> ConfigView {
        ConfigView {
            kernel: json!({"policy": "bingo.permissions"}),
            plugins: [
                ("bingo.permissions".to_string(), mode("default")),
                ("aaa".to_string(), mode("plain")),
            ]
            .into(),
        }
    }

    #[tokio::test]
    async fn a_rewrite_link_cannot_rename_or_relabel_the_policy() {
        let cases = [
            ("/kernel/policy", json!("aaa")),
            ("/plugins/bingo.permissions", mode("wide")),
        ];
        for (pointer, value) in cases {
            for outbound in [false, true] {
                let links = [Link {
                    plugin: "fixture".into(),
                    placement: bingo_sdk::plugin::Placement::Outer,
                    external: None,
                    interceptor: Arc::new(Relabel {
                        pointer,
                        value: value.clone(),
                        outbound,
                    }),
                    tier: Tier::Rewrite,
                }];
                let report = Report::default();
                let described = describe(given(), &links, &cx(), report.clone()).await;
                let described = described.unwrap_or_default();
                assert_eq!(
                    described.policy(),
                    given().policy(),
                    "{pointer} outbound={outbound}"
                );
                assert!(!report.take().is_empty(), "{pointer} is reported");
            }
        }
    }

    #[tokio::test]
    async fn a_rewrite_link_may_still_describe_another_slice() {
        let links = [Link {
            plugin: "fixture".into(),
            placement: bingo_sdk::plugin::Placement::Outer,
            external: None,
            interceptor: Arc::new(Relabel {
                pointer: "/plugins/aaa",
                value: json!({"note": "mine"}),
                outbound: true,
            }),
            tier: Tier::Rewrite,
        }];
        let described = describe(given(), &links, &cx(), Report::default())
            .await
            .unwrap();
        assert_eq!(described.plugins["aaa"], json!({"note": "mine"}));
    }
}
