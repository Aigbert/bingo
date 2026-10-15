use crate::delivery::{Answer, Fired};
use crate::entry::{Provenance, tests::entry};
use crate::signed::asked_origin;
use crate::tests::Fixture;
use bingo_sdk::{
    ErrorCode, Input, IntentOutcome, KernelError, TurnId,
    testing::{HostCall, TestHost},
};
use std::time::Duration;

#[path = "delivery_live.rs"]
mod live;
use live::Live;

/// The person's own `/guard`: a command line they asked for.
fn persons_command() -> crate::Entry {
    crate::Entry {
        text: "/guard".into(),
        by: Provenance::Person,
        ..entry()
    }
}

fn started() -> IntentOutcome {
    IntentOutcome::TurnStarted {
        turn: TurnId::from_raw("trn_live"),
    }
}

/// Let paused time run `seconds`, a rescan at a time, and every task settle.
async fn elapse(seconds: u64) {
    for _ in 0..seconds.div_ceil(10) {
        tokio::time::advance(Duration::from_secs(10)).await;
        for _ in 0..20 {
            tokio::task::yield_now().await;
        }
    }
}

fn line_text(input: &Input) -> &str {
    match input {
        Input::Text { text, .. } => text,
        Input::Action { .. } => "",
    }
}

#[tokio::test]
async fn a_line_the_session_closed_on_is_spent_and_said() {
    let fixture = Fixture::new();
    let entry = entry();
    fixture.store().save(&entry).unwrap();
    let host = TestHost::with_sessions(vec![]);
    host.on_deliver(|_, _, _, _| async { Ok(()) });
    let owner = crate::service::Owner::new(fixture.schedules.clone(), host.handle());
    use bingo_services::schedule::Service;
    let result = bingo_sdk::render::under_input(true, owner.run(entry.id.clone())).await;
    assert_eq!(result.unwrap_err().code, ErrorCode::SessionClosed);
    assert!(
        fixture.shelf().entries[0].last_fired.is_some(),
        "its session opened, so the occurrence is spent"
    );
    assert!(
        fixture
            .schedules
            .trouble()
            .unwrap()
            .contains("refused or failed")
    );
}

#[tokio::test]
async fn manual_refusals_share_the_process_notice_and_trouble() {
    let fixture = Fixture::new();
    let mut live = Live::new(0);
    fixture
        .store()
        .save(&crate::Entry {
            text: "/guard".into(),
            ..entry()
        })
        .unwrap();
    for _ in 0..2 {
        let dispatch = fixture.schedules.dispatcher(live.host.handle());
        let current = fixture.shelf().entries[0].clone();
        let store = fixture.store();
        let fire = tokio::spawn(async move { dispatch.fire(&store, &current).await });
        let (intent, input) = live.lines.recv().await.unwrap();
        assert_eq!(line_text(&input), "/guard", "said to the model as prose");
        live.answer(&intent, started());
        assert!(matches!(fire.await.unwrap(), Ok(Fired::Spent { .. })));
    }
    let trouble = fixture.schedules.trouble().unwrap();
    assert!(trouble.contains("/schedule confirm"), "{trouble}");
    let notices = live.host.calls();
    let notices = notices
        .iter()
        .filter(|call| matches!(call, HostCall::Notice { .. }));
    assert_eq!(notices.count(), 1);
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
            Err(KernelError::new(ErrorCode::Internal, "fixture refused"))
        }
    });
    fixture.schedules.start(host.handle());
    let mut seen = vec![opened.recv().await.unwrap(), opened.recv().await.unwrap()];
    seen.sort();
    assert_eq!(seen, ["schedule/fast", "schedule/slow"]);
    tokio::time::advance(Duration::from_secs(31)).await;
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

/// A `/command` acknowledges when it ends. One that runs past every bound
/// and answers late was spent when its session opened, and fires once.
#[tokio::test(start_paused = true)]
async fn a_line_answered_after_the_bound_fires_exactly_once() {
    let fixture = Fixture::new();
    fixture.store().save(&persons_command()).unwrap();
    let mut live = Live::new(0);
    fixture.schedules.start(live.host.handle());
    let (intent, input) = live.lines.recv().await.unwrap();
    assert_eq!(line_text(&input), "/guard");
    assert!(
        fixture.shelf().entries[0].last_fired.is_some(),
        "spent before its line was submitted"
    );
    elapse(200).await;
    live.answer(&intent, started());
    elapse(200).await;
    assert_eq!(live.opened(), 1);
    assert!(live.lines.try_recv().is_err(), "the occurrence fired once");
    fixture.schedules.stop().await;
}

#[tokio::test(start_paused = true)]
async fn a_refused_line_is_spent_and_badged_not_fired_again() {
    let fixture = Fixture::new();
    fixture.store().save(&persons_command()).unwrap();
    let mut live = Live::new(0);
    fixture.schedules.start(live.host.handle());
    let (intent, _) = live.lines.recv().await.unwrap();
    let refused = KernelError::new(ErrorCode::PermissionDenied, "guarded");
    live.answer(&intent, IntentOutcome::Rejected { error: refused });
    elapse(200).await;
    assert_eq!(live.opened(), 1);
    assert!(live.lines.try_recv().is_err(), "a refusal is not a retry");
    assert!(fixture.shelf().entries[0].last_fired.is_some());
    let trouble = fixture.schedules.trouble().unwrap();
    assert!(trouble.contains("refused or failed: guarded"), "{trouble}");
    fixture.schedules.stop().await;
}

#[tokio::test(start_paused = true)]
async fn a_stamp_the_disk_refused_does_not_fire_the_occurrence_again() {
    let fixture = Fixture::new();
    fixture.store().save(&persons_command()).unwrap();
    let blocker = fixture.dir().join(format!(".{}.tmp", persons_command().id));
    std::fs::create_dir(&blocker).unwrap();
    let mut live = Live::new(0);
    fixture.schedules.start(live.host.handle());
    let (intent, _) = live.lines.recv().await.unwrap();
    live.answer(&intent, started());
    elapse(200).await;
    assert_eq!(live.opened(), 1);
    assert!(live.lines.try_recv().is_err(), "remembered in this process");
    assert!(fixture.shelf().entries[0].last_fired.is_none(), "unwritten");
    let trouble = fixture.schedules.trouble().unwrap();
    assert!(trouble.contains("not written down"), "{trouble}");
    fixture.schedules.stop().await;
}

#[tokio::test(start_paused = true)]
async fn a_fire_that_panicked_frees_its_entry_after_a_rescan() {
    let fixture = Fixture::new();
    fixture.store().save(&persons_command()).unwrap();
    let mut live = Live::new(1);
    fixture.schedules.start(live.host.handle());
    elapse(10).await;
    let trouble = fixture.schedules.trouble().unwrap();
    assert!(trouble.contains("stopped unexpectedly"), "{trouble}");
    assert!(live.lines.try_recv().is_err(), "nothing ran");
    elapse(70).await;
    let (intent, _) = live.lines.recv().await.unwrap();
    live.answer(&intent, started());
    assert_eq!(live.opened(), 1, "the second open is the first to succeed");
    fixture.schedules.stop().await;
}

/// The person's own prose speaks as their unattended line, as a wake.
#[tokio::test]
async fn a_persons_prose_is_delivered_as_their_own_line() {
    let fixture = Fixture::new();
    let prose = crate::Entry {
        by: Provenance::Person,
        ..entry()
    };
    fixture.store().save(&prose).unwrap();
    let mut live = Live::new(0);
    let dispatch = fixture.schedules.dispatcher(live.host.handle());
    let store = fixture.store();
    let fired = prose.clone();
    let fire = tokio::spawn(async move { dispatch.fire(&store, &fired).await });
    let (intent, input) = live.lines.recv().await.unwrap();
    live.answer(&intent, started());
    let Ok(Fired::Spent { answer, .. }) = fire.await.unwrap() else {
        panic!("spent");
    };
    assert_eq!(answer, Answer::Taken);
    let Input::Text { text, origin, .. } = input else {
        panic!("a line of text");
    };
    assert_eq!(text, prose.text);
    assert_eq!(origin, asked_origin(), "signed as the person's print line");
    assert!(
        live.host
            .calls()
            .iter()
            .any(|call| matches!(call, HostCall::Deliver { intent: sent, .. } if sent == &intent)),
        "delivered, as a wake, not submitted"
    );
}
