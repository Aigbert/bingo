use super::{HostCall, NoHost, TestHost};
use crate::*;
use futures::StreamExt;
use serde_json::json;
use std::sync::Arc;

fn who() -> ClientIdentity {
    ClientIdentity {
        name: "test".into(),
        surface: "contract".into(),
    }
}

fn summary(id: &str) -> SessionSummary {
    serde_json::from_value(json!({
        "id": id, "cwd": ".", "createdAt": "2026-01-01T00:00:00Z",
        "updatedAt": "2026-01-01T00:00:00Z", "usage": Usage::default()
    }))
    .expect("a summary")
}

#[tokio::test]
async fn unscripted_calls_keep_the_no_host_defaults_and_are_recorded() {
    let host = TestHost::default();
    let id = SessionId::from_raw("missing");
    let intent = IntentId::from_raw("input");
    let turn = TurnId::from_raw("turn");
    let selector = SessionSelector::ById { id: id.clone() };
    assert_eq!(
        host.sessions(SessionFilter::default()).await,
        NoHost.sessions(SessionFilter::default()).await
    );
    assert_eq!(
        host.open(selector.clone(), who(), OpenOptions::default())
            .await
            .unwrap_err(),
        NoHost
            .open(selector, who(), OpenOptions::default())
            .await
            .unwrap_err()
    );
    assert_eq!(
        host.close(&id, CloseReason::Client).await,
        NoHost.close(&id, CloseReason::Client).await
    );
    assert_eq!(host.delete(&id).await, NoHost.delete(&id).await);
    assert_eq!(
        host.deliver(
            &id,
            intent.clone(),
            Input::text("hello", Origin::default()),
            Delivery::Wake
        )
        .await,
        NoHost
            .deliver(
                &id,
                intent.clone(),
                Input::text("hello", Origin::default()),
                Delivery::Wake
            )
            .await
    );
    assert_eq!(
        host.extend(&id, "plugin", "state", json!(1)).await,
        NoHost.extend(&id, "plugin", "state", json!(1)).await
    );
    assert_eq!(
        host.signal(&id, "plugin", "live", json!(2)).await,
        NoHost.signal(&id, "plugin", "live", json!(2)).await
    );
    let call = ToolCall {
        call_id: "call".into(),
        name: "test".into(),
        input: json!({}),
    };
    assert_eq!(
        host.invoke(&id, call.clone()).await,
        NoHost.invoke(&id, call).await
    );
    let kind = InteractionKind::Confirm {
        title: "Continue?".into(),
        detail: "test".into(),
    };
    assert_eq!(
        host.ask(&id, kind.clone(), vec![]).await,
        NoHost.ask(&id, kind, vec![]).await
    );
    assert_eq!(
        host.rewind(&id, &turn).await,
        NoHost.rewind(&id, &turn).await
    );
    assert_eq!(
        host.withdraw(&id, &intent, who()).await,
        NoHost.withdraw(&id, &intent, who()).await
    );
    assert_eq!(
        host.reconfigure(&id, SessionChange::Title("new".into()))
            .await,
        NoHost
            .reconfigure(&id, SessionChange::Title("new".into()))
            .await
    );
    assert_eq!(
        host.catalog(CatalogKind::Tools).await,
        NoHost.catalog(CatalogKind::Tools).await
    );
    assert_eq!(
        host.notice(Level::Info, "code", "text").await,
        NoHost.notice(Level::Info, "code", "text").await
    );
    assert!(host.gateway_events().next().await.is_none());
    assert!(host.service_any("absent").is_none());
    assert!(host.service_wire("absent").is_none());
    let calls = host.calls();
    assert_eq!(calls.len(), 17);
    assert!(
        matches!(&calls[1], HostCall::Open { selector: SessionSelector::ById { id: recorded }, who: client, options } if recorded.as_str() == id.as_str() && client == &who() && options == &OpenOptions::default())
    );
    assert!(
        matches!(&calls[4], HostCall::Deliver { to, intent: recorded, input, delivery: Delivery::Wake } if to == &id && recorded == &intent && input == &Input::text("hello", Origin::default()))
    );
    assert!(
        matches!(&calls[5], HostCall::Extend { session, plugin, kind, payload } if session == &id && plugin == "plugin" && kind == "state" && payload == &json!(1))
    );
}

#[tokio::test]
async fn scripts_can_suspend_and_observe_the_recorded_arguments() {
    let host = Arc::new(TestHost::default());
    let entered = Arc::new(tokio::sync::Notify::new());
    let proceed = Arc::new(tokio::sync::Notify::new());
    host.on_deliver({
        let host = Arc::downgrade(&host);
        let entered = entered.clone();
        let proceed = proceed.clone();
        move |to, intent, input, delivery| {
            let host = host.clone();
            let entered = entered.clone();
            let proceed = proceed.clone();
            async move {
                assert_eq!(to, SessionId::from_raw("target"));
                assert_eq!(intent, IntentId::from_raw("intent"));
                assert_eq!(input, Input::text("payload", Origin::default()));
                assert_eq!(delivery, Delivery::Hold);
                assert_eq!(host.upgrade().expect("host").calls().len(), 1);
                entered.notify_one();
                proceed.notified().await;
                Err(KernelError::new(ErrorCode::NotReady, "scripted"))
            }
        }
    });
    let pending = tokio::spawn({
        let host = host.clone();
        async move {
            host.deliver(
                &SessionId::from_raw("target"),
                IntentId::from_raw("intent"),
                Input::text("payload", Origin::default()),
                Delivery::Hold,
            )
            .await
        }
    });
    entered.notified().await;
    assert!(!pending.is_finished());
    proceed.notify_one();
    assert_eq!(
        pending.await.expect("joined").unwrap_err().message,
        "scripted"
    );
}

#[tokio::test]
async fn seeded_sessions_open_detach_and_delete_without_losing_the_summary() {
    let first = summary("first");
    let host = TestHost::with_sessions(vec![first.clone()]);
    assert_eq!(
        host.sessions(SessionFilter::default()).await.expect("list"),
        vec![first.clone()]
    );
    let opened = host
        .open(
            SessionSelector::ById {
                id: first.id.clone(),
            },
            who(),
            OpenOptions::default(),
        )
        .await
        .expect("open");
    assert_eq!(opened.snapshot.summary, first);
    host.close(&first.id, CloseReason::Client)
        .await
        .expect("detach");
    assert_eq!(
        host.sessions(SessionFilter::default()).await.expect("list"),
        vec![first.clone()]
    );
    host.delete(&first.id).await.expect("delete");
    assert!(
        host.sessions(SessionFilter::default())
            .await
            .expect("list")
            .is_empty()
    );
    assert_eq!(
        host.open(
            SessionSelector::ById { id: first.id },
            who(),
            OpenOptions::default()
        )
        .await
        .unwrap_err()
        .code,
        ErrorCode::SessionNotFound
    );
}

#[tokio::test]
async fn scripts_override_the_session_table_without_changing_it() {
    let host = TestHost::with_sessions(vec![summary("kept")]);
    host.on_delete(|_| async { Err(KernelError::new(ErrorCode::PermissionDenied, "kept")) });
    assert!(host.delete(&SessionId::from_raw("kept")).await.is_err());
    assert_eq!(
        host.sessions(SessionFilter::default())
            .await
            .expect("list")
            .len(),
        1
    );
}

#[tokio::test]
async fn gateway_events_are_driven_by_the_test() {
    let host = TestHost::default();
    let sender = host.gateway();
    let mut events = host.gateway_events();
    sender
        .unbounded_send(GatewayEvent::SessionRemoved {
            session: SessionId::from_raw("gone"),
        })
        .expect("send");
    assert!(
        matches!(events.next().await, Some(GatewayEvent::SessionRemoved { session }) if session == SessionId::from_raw("gone"))
    );
    drop(sender);
    assert!(events.next().await.is_none());
    assert!(matches!(host.calls().as_slice(), [HostCall::GatewayEvents]));
}

struct Echo;

#[async_trait::async_trait]
impl WireService for Echo {
    async fn call(
        &self,
        _: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, ServiceError> {
        Ok(params)
    }
}

#[tokio::test]
async fn a_service_has_one_owner_and_both_faces() {
    let host = Arc::new(TestHost::default());
    let wire: Arc<dyn WireService> = Arc::new(Echo);
    host.open_service("echo", wire.clone()).expect("open");
    assert_eq!(
        host.open_service("echo", Arc::new(Echo)).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    assert!(Arc::ptr_eq(
        &host.service_wire("echo").expect("wire"),
        &wire
    ));
    let handle = host.handle();
    assert_eq!(
        handle
            .service::<ServiceHandle>("echo")
            .expect("typed")
            .call("echo", json!(42))
            .await
            .expect("answer"),
        json!(42)
    );
    assert!(
        matches!(&host.calls()[0], HostCall::OpenService { key, wire: recorded } if key == "echo" && Arc::ptr_eq(recorded, &wire))
    );
}
