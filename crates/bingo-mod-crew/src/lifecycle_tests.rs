use bingo_sdk::*;
use serde_json::{Value, json};

fn context(state: Value) -> (testing::TestHost, HookContext, crate::Shared) {
    let host = testing::TestHost::default();
    host.on_signal(|_, _, _, _| async { Ok(()) });
    host.on_catalog_for(move |_, kind| {
        let state = state.clone();
        async move {
            assert_eq!(kind, CatalogKind::Plugins);
            Ok(Catalog {
                kind,
                entries: vec![CatalogEntry {
                    id: crate::ID.into(),
                    label: "crew".into(),
                    meta: json!({"state":state}),
                }],
            })
        }
    });
    let cx = HookContext {
        session: SessionId::from_raw("ses_parent"),
        turn: None,
        item: None,
        cwd: "/unused".into(),
        provider: None,
        model: None,
        host: host.handle(),
    };
    let shared = crate::Shared::default();
    shared
        .lock()
        .unwrap()
        .insert(cx.session.clone(), Default::default());
    (host, cx, shared)
}

async fn disable(
    shared: crate::Shared,
    cx: &HookContext,
    reply: Result<Value, KernelError>,
) -> Result<Value, KernelError> {
    crate::lifecycle::Disable(shared)
        .chain(
            catalogue::COMMAND_RUN,
            cx,
            json!({"command":"plugins","args":"disable bingo.mod.crew"}),
            Next::new(move |_| Box::pin(async move { reply })),
        )
        .await
}

#[tokio::test]
async fn a_successful_live_disable_drops_every_panel_and_removes_its_signal() {
    let (host, cx, shared) = context(json!({"kind":"off","why":"switched off"}));
    let second = SessionId::from_raw("ses_second");
    shared
        .lock()
        .unwrap()
        .insert(second.clone(), Default::default());
    let reply = json!(CommandOutcome::Applied {
        message: Some("bingo.mod.crew is off now.".into()),
    });
    assert_eq!(
        disable(shared.clone(), &cx, Ok(reply.clone()))
            .await
            .unwrap(),
        reply
    );
    assert!(shared.lock().unwrap().is_empty());
    let removed: Vec<_> = host
        .calls()
        .into_iter()
        .filter_map(|call| match call {
            testing::HostCall::Signal {
                session,
                plugin,
                kind,
                payload,
            } => {
                assert_eq!(plugin, crate::ID);
                assert_eq!(kind, crate::REQUEST);
                assert!(payload.is_null());
                Some(session)
            }
            _ => None,
        })
        .collect();
    assert_eq!(removed, [cx.session, second]);
}

#[tokio::test]
async fn pending_or_overridden_disables_preserve_the_open_panel() {
    for state in [
        json!({"kind":"on"}),
        json!({"kind":"pending","now":true,"at":"nextStart"}),
    ] {
        let (host, cx, shared) = context(state);
        let reply = json!(CommandOutcome::Applied {
            message: Some("switch saved".into()),
        });
        assert_eq!(
            disable(shared.clone(), &cx, Ok(reply.clone()))
                .await
                .unwrap(),
            reply
        );
        assert!(shared.lock().unwrap().contains_key(&cx.session));
        assert!(
            !host
                .calls()
                .iter()
                .any(|call| matches!(call, testing::HostCall::Signal { .. }))
        );
    }
}

#[tokio::test]
async fn refused_disables_preserve_the_open_panel_without_reading_or_signalling() {
    let (host, cx, shared) = context(json!({"kind":"off","why":"switched off"}));
    let error = KernelError::new(ErrorCode::InvalidInput, "the switch was refused");
    assert!(disable(shared.clone(), &cx, Err(error)).await.is_err());
    assert!(shared.lock().unwrap().contains_key(&cx.session));
    assert!(host.calls().is_empty());
}
