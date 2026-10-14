use super::*;
use futures::FutureExt;

fn complete(started: &Started, session: &SessionId) {
    started.send_modify(|sessions| {
        sessions.insert(session.clone());
    });
}

#[tokio::test]
async fn startup_waits_for_every_module_and_the_requested_session() {
    let environment = Environment::default();
    let first = environment.track_start("first", true).unwrap();
    let last = environment.track_start("last", true).unwrap();
    let session = SessionId::mint();
    complete(&first, &session);
    let waiting = environment.wait_for_start(&session);
    tokio::pin!(waiting);
    assert!(waiting.as_mut().now_or_never().is_none());
    complete(&last, &SessionId::mint());
    assert!(waiting.as_mut().now_or_never().is_none());
    complete(&last, &session);
    waiting.await.unwrap();
    environment.wait_for_start(&session).await.unwrap();
}

#[tokio::test]
async fn reload_waits_for_the_new_generation() {
    let environment = Environment::default();
    let old = environment.track_start("module", true).unwrap();
    let session = SessionId::mint();
    complete(&old, &session);
    let new = environment.track_start("module", true).unwrap();
    let waiting = environment.wait_for_start(&session);
    tokio::pin!(waiting);
    complete(&old, &session);
    assert!(waiting.as_mut().now_or_never().is_none());
    complete(&new, &session);
    waiting.await.unwrap();
}

#[tokio::test]
async fn removed_observers_and_replaced_hosts_do_not_hold_startup() {
    let environment = Environment::default();
    let _old = environment.track_start("module", true).unwrap();
    assert!(environment.track_start("module", false).is_none());
    environment
        .wait_for_start(&SessionId::mint())
        .await
        .unwrap();
    let _old = environment.track_start("module", true).unwrap();
    environment.reset_starts();
    environment
        .wait_for_start(&SessionId::mint())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_dropped_observer_reports_the_module_instead_of_hanging() {
    let environment = Environment::default();
    drop(environment.track_start("module", true));
    assert_eq!(
        environment
            .wait_for_start(&SessionId::mint())
            .await
            .unwrap_err(),
        "module: session.start observer closed"
    );
}
