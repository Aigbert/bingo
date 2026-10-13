use bingo_alarm::{Alarm, Alarms};
use std::{sync::{Arc, atomic::{AtomicUsize, Ordering}}, time::Duration};
use tokio::time::Instant;

fn delivery(count: &Arc<AtomicUsize>, delay: u64) -> Alarm {
    let count = count.clone();
    Alarm::new(Instant::now() + Duration::from_secs(delay), async move {
        count.fetch_add(1, Ordering::SeqCst);
    })
}

async fn advance(seconds: u64) {
    tokio::time::advance(Duration::from_secs(seconds)).await;
    for _ in 0..8 { tokio::task::yield_now().await; }
}

#[tokio::test(start_paused = true)]
async fn arm_cancel_replace_reconcile_and_drop_are_owned() {
    let count = Arc::new(AtomicUsize::new(0));
    let mut alarms = Alarms::default();
    alarms.arm("fire", delivery(&count, 10));
    alarms.arm("cancel", delivery(&count, 10));
    alarms.cancel(&"cancel");
    alarms.arm("replace", delivery(&count, 5));
    alarms.arm("replace", delivery(&count, 20));
    advance(10).await;
    assert_eq!(count.load(Ordering::SeqCst), 1);
    advance(10).await;
    assert_eq!(count.load(Ordering::SeqCst), 2);
    alarms.arm("keep", delivery(&count, 10));
    alarms.arm("remove", delivery(&count, 10));
    alarms.reconcile([("keep", delivery(&count, 100)), ("add", delivery(&count, 10))]);
    advance(10).await;
    assert_eq!(count.load(Ordering::SeqCst), 4, "keep its deadline, remove absent, arm new");
    alarms.arm("drop", delivery(&count, 10));
    drop(alarms);
    advance(100).await;
    assert_eq!(count.load(Ordering::SeqCst), 4);
}
