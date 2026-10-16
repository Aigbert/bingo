use bingo_sdk::*;
use serde_json::{Value, json};

pub struct Disable(pub crate::Shared);
#[async_trait::async_trait]
impl Interceptor for Disable {
    fn id(&self) -> &str {
        "models.disable"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"command":"plugins"}),
            ..LinkMatcher::at(catalogue::COMMAND_RUN)
        }
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        cx: &HookContext,
        value: Value,
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let run: catalogue::CommandRun =
            serde_json::from_value(value.clone()).map_err(crate::error)?;
        if run.name != "plugins"
            || run.args.split_whitespace().collect::<Vec<_>>() != ["disable", "bingo.mod.models"]
        {
            return next.run(value).await;
        }
        let reply = next.run(value).await?;
        if !matches!(
            serde_json::from_value::<CommandOutcome>(reply.clone()),
            Ok(CommandOutcome::Applied { .. })
        ) {
            return Ok(reply);
        }
        let plugins = cx.host.catalog(CatalogKind::Plugins).await?;
        let state = plugins
            .entries
            .iter()
            .find(|plugin| plugin.id == "bingo.mod.models")
            .and_then(|plugin| {
                serde_json::from_value::<PluginState>(plugin.meta["state"].clone()).ok()
            });
        if matches!(state, Some(PluginState::Off { .. })) {
            self.close(cx).await?;
        }
        Ok(reply)
    }
}
impl Disable {
    async fn close(&self, cx: &HookContext) -> Result<(), KernelError> {
        let sessions = self
            .0
            .lock()
            .map_err(crate::error)?
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for session in sessions {
            crate::state::close(&cx.host, &session).await?;
            self.0.lock().map_err(crate::error)?.remove(&session);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(state: Value) -> (testing::TestHost, HookContext, crate::Shared) {
        let host = testing::TestHost::default();
        host.on_signal(|_, _, _, _| async { Ok(()) });
        host.on_catalog(move |kind| {
            assert_eq!(kind, CatalogKind::Plugins);
            let state = state.clone();
            async move {
                Ok(Catalog {
                    kind,
                    entries: vec![CatalogEntry {
                        id: "bingo.mod.models".into(),
                        label: "models".into(),
                        meta: json!({"state":state}),
                    }],
                })
            }
        });
        let cx = HookContext {
            session: SessionId::from_raw("one"),
            cwd: "/unused".into(),
            host: host.handle(),
            item: None,
            turn: None,
            provider: None,
            model: None,
        };
        let shared = crate::Shared::default();
        for id in ["one", "two"] {
            shared.lock().unwrap().insert(
                SessionId::from_raw(id),
                crate::state::Picker {
                    id: format!("picker-{id}"),
                    ..Default::default()
                },
            );
        }
        (host, cx, shared)
    }

    #[tokio::test]
    async fn an_applied_disable_retires_every_picker_and_its_signal() {
        for args in [
            "disable bingo.mod.models",
            " \tdisable   bingo.mod.models\n",
        ] {
            let (host, cx, shared) = setup(json!({"kind":"off","why":"disabled"}));
            let input = json!({"command":"plugins","args":args});
            let expected = input.clone();
            let disable = Disable(shared.clone());
            assert!(disable.matcher().matches(catalogue::COMMAND_RUN, &input));
            let reply = disable
                .chain(
                    catalogue::COMMAND_RUN,
                    &cx,
                    input,
                    Next::new(move |input| {
                        assert_eq!(input, expected);
                        Box::pin(async { Ok(json!({"kind":"applied","message":"off now"})) })
                    }),
                )
                .await
                .unwrap();
            assert_eq!(reply, json!({"kind":"applied","message":"off now"}));
            assert!(shared.lock().unwrap().is_empty());
            let retired = host
                .calls()
                .into_iter()
                .filter_map(|call| match call {
                    testing::HostCall::Signal {
                        session,
                        plugin,
                        kind,
                        payload,
                    } if plugin == "bingo.mod.models" && kind == "picker" && payload.is_null() => {
                        Some(session)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                retired,
                [SessionId::from_raw("one"), SessionId::from_raw("two")]
            );
        }
    }

    #[tokio::test]
    async fn pending_overridden_and_refused_switches_keep_the_picker() {
        for (state, refused) in [
            (json!({"kind":"pending","now":true,"at":"afterTurn"}), false),
            (json!({"kind":"on"}), false),
            (json!({"kind":"failed","why":"refused"}), false),
            (Value::Null, false),
            (json!({"kind":"off","why":"disabled"}), true),
        ] {
            let (host, cx, shared) = setup(state);
            let reply = Disable(shared.clone())
                .chain(
                    catalogue::COMMAND_RUN,
                    &cx,
                    json!({"command":"plugins","args":"disable bingo.mod.models"}),
                    Next::new(move |_| {
                        Box::pin(async move {
                            if refused {
                                Err(crate::error("switch refused"))
                            } else {
                                Ok(json!({"kind":"applied","message":"switch saved"}))
                            }
                        })
                    }),
                )
                .await;
            assert_eq!(reply.is_err(), refused);
            assert_eq!(shared.lock().unwrap().len(), 2);
            assert!(
                !host
                    .calls()
                    .iter()
                    .any(|call| matches!(call, testing::HostCall::Signal { .. }))
            );
        }
    }

    #[tokio::test]
    async fn another_switch_never_queries_or_clears_the_picker() {
        let (host, cx, shared) = setup(json!({"kind":"off","why":"disabled"}));
        Disable(shared.clone())
            .chain(
                catalogue::COMMAND_RUN,
                &cx,
                json!({"command":"plugins","args":"enable bingo.mod.models"}),
                Next::new(|_| Box::pin(async { Ok(json!({"kind":"applied"})) })),
            )
            .await
            .unwrap();
        assert_eq!(shared.lock().unwrap().len(), 2);
        assert!(host.calls().is_empty());
    }

    #[tokio::test]
    async fn a_non_applied_reply_keeps_state_even_when_the_catalogue_says_off() {
        let (host, cx, shared) = setup(json!({"kind":"off","why":"disabled"}));
        let reply = json!({"kind":"handled","handled":false});
        let expected = reply.clone();
        let result = Disable(shared.clone())
            .chain(
                catalogue::COMMAND_RUN,
                &cx,
                json!({"command":"plugins","args":"disable bingo.mod.models"}),
                Next::new(move |_| Box::pin(async move { Ok(reply) })),
            )
            .await
            .unwrap();
        assert_eq!(result, expected);
        assert_eq!(shared.lock().unwrap().len(), 2);
        assert!(host.calls().is_empty());
    }
}
