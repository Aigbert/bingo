//! Keyed, owned timers. Owners derive deliveries from their own records.
use bingo_sdk::CancellationToken;
use std::{collections::BTreeMap, future::Future, pin::Pin, time::Duration};
use tokio::{task::JoinHandle, time::Instant};

/// A deadline and the work it releases, kept together during reconciliation.
pub struct Alarm {
    at: Instant,
    deliver: Pin<Box<dyn Future<Output = ()> + Send>>,
}

impl Alarm {
    pub fn new(at: Instant, deliver: impl Future<Output = ()> + Send + 'static) -> Self {
        Self {
            at,
            deliver: Box::pin(deliver),
        }
    }
}

#[derive(Debug)]
struct Task {
    cancel: CancellationToken,
    running: JoinHandle<()>,
}
impl Drop for Task {
    fn drop(&mut self) {
        self.cancel.cancel();
        self.running.abort();
    }
}

/// A completed key remains taken until its owner cancels or replaces it.
/// Re-reading the same durable obligation cannot fire it twice in one run.
#[derive(Debug)]
pub struct Alarms<K: Ord + Clone> {
    tasks: BTreeMap<K, Task>,
}
impl<K: Ord + Clone> Default for Alarms<K> {
    fn default() -> Self {
        Self {
            tasks: BTreeMap::new(),
        }
    }
}
impl<K: Ord + Clone> Alarms<K> {
    pub fn arm(&mut self, key: K, alarm: Alarm) {
        self.cancel(&key);
        let cancel = CancellationToken::new();
        let stopped = cancel.clone();
        let running = tokio::spawn(async move {
            tokio::select! {
                biased;
                () = stopped.cancelled() => {},
                () = async { if alarm.at > Instant::now() { tokio::time::sleep_until(alarm.at).await; } alarm.deliver.await; } => {},
            }
        });
        self.tasks.insert(key, Task { cancel, running });
    }

    pub fn cancel(&mut self, key: &K) {
        self.tasks.remove(key);
    }
    pub fn contains(&self, key: &K) -> bool {
        self.tasks.contains_key(key)
    }
    pub fn pending(&self, key: &K) -> bool {
        self.tasks
            .get(key)
            .is_some_and(|task| !task.running.is_finished())
    }
    pub fn clear(&mut self) {
        self.tasks.clear();
    }
    pub fn retain(&mut self, mut keep: impl FnMut(&K) -> bool) {
        self.tasks.retain(|key, _| keep(key));
    }

    pub fn reconcile(&mut self, wanted: impl IntoIterator<Item = (K, Alarm)>) {
        let wanted: BTreeMap<K, Alarm> = wanted.into_iter().collect();
        self.retain(|key| wanted.contains_key(key));
        for (key, alarm) in wanted {
            if !self.contains(&key) {
                self.arm(key, alarm);
            }
        }
    }
}

/// Convert a journal timestamp once; paused Tokio clocks remain deterministic.
pub fn deadline(at: jiff::Timestamp, now: jiff::Timestamp) -> Instant {
    let delay = at.duration_since(now);
    let delay = if delay.is_negative() {
        Duration::ZERO
    } else {
        delay.unsigned_abs()
    };
    Instant::now() + delay
}
