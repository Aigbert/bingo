use crate::{entry::tests::entry, tests::Fixture};
use bingo_sdk::{
    ErrorCode,
    testing::{HostCall, TestHost},
};

#[tokio::test]
async fn acceptance_without_an_ack_does_not_spend_the_entry() {
    let fixture = Fixture::new();
    let entry = entry();
    fixture.store().save(&entry).unwrap();
    let host = TestHost::with_sessions(vec![]);
    host.on_deliver(|_, _, _, _| async { Ok(()) });
    let owner = crate::service::Owner::new(fixture.schedules.clone(), host.handle());
    use bingo_services::schedule::Service;
    let result = bingo_sdk::render::under_input(true, owner.run(entry.id.clone())).await;
    assert_eq!(result.unwrap_err().code, ErrorCode::SessionClosed);
    assert_eq!(fixture.shelf().entries[0], entry);
}

#[tokio::test]
async fn manual_refusals_share_the_process_notice_and_trouble() {
    let fixture = Fixture::new();
    let host = TestHost::with_sessions(vec![]);
    host.on_deliver(|_, _, _, _| async { Ok(()) });
    host.on_notice(|_, _, _| async { Ok(()) });
    let entry = crate::Entry {
        text: "/guard".into(),
        ..entry()
    };
    for _ in 0..2 {
        let dispatch = fixture.schedules.dispatcher(host.handle());
        assert!(dispatch.deliver(&entry).await.is_err());
    }
    assert!(
        fixture
            .schedules
            .trouble()
            .unwrap()
            .contains("/schedule confirm")
    );
    assert_eq!(
        host.calls()
            .iter()
            .filter(|call| matches!(call, HostCall::Notice { .. }))
            .count(),
        1
    );
}

#[tokio::test(start_paused = true)]
async fn a_stalled_entry_does_not_hold_another_and_times_out_unspent() {
    let fixture = Fixture::new();
    for id in ["slow", "fast"] {
        fixture
            .store()
            .save(&crate::Entry {
                id: id.into(),
                ..entry()
            })
            .unwrap();
    }
    let host = TestHost::with_sessions(vec![]);
    let (sent, mut opened) = tokio::sync::mpsc::unbounded_channel();
    host.on_open(move |selector, _, _| {
        let sent = sent.clone();
        async move {
            let bingo_sdk::SessionSelector::ByKey { key } = selector else {
                panic!("key");
            };
            sent.send(key.clone()).unwrap();
            if key.ends_with("slow") {
                std::future::pending::<()>().await;
            }
            Err(bingo_sdk::KernelError::new(
                ErrorCode::Internal,
                "fixture refused",
            ))
        }
    });
    fixture.schedules.start(host.handle());
    let mut seen = vec![opened.recv().await.unwrap(), opened.recv().await.unwrap()];
    seen.sort();
    assert_eq!(seen, ["schedule/fast", "schedule/slow"]);
    tokio::time::advance(std::time::Duration::from_secs(31)).await;
    tokio::task::yield_now().await;
    assert!(
        fixture
            .shelf()
            .entries
            .iter()
            .all(|entry| entry.last_fired.is_none())
    );
    assert!(
        opened.try_recv().is_err(),
        "neither failure spins or overlaps itself"
    );
    fixture.schedules.stop().await;
}
