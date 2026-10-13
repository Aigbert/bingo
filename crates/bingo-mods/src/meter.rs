//! A hook clock can be interrupted without acquiring the JS runtime lock.
use crate::budget::Budget;
use bingo_sdk::CancellationToken;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub struct Meter {
    clock: Instant,
    budget: Mutex<Budget>,
    limit: Duration,
    pub cancel: CancellationToken,
    changed: tokio::sync::Notify,
}
impl Meter {
    pub fn new(limit: Duration, cancel: CancellationToken) -> Arc<Self> {
        Arc::new(Self {
            clock: Instant::now(),
            budget: Mutex::new(Budget::default()),
            limit,
            cancel,
            changed: tokio::sync::Notify::new(),
        })
    }
    pub fn resume(&self) {
        self.budget
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .resume(self.clock.elapsed());
        self.changed.notify_one();
    }
    pub fn suspend(&self) {
        self.budget
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .suspend(self.clock.elapsed());
        self.changed.notify_one();
    }
    pub fn remaining(&self) -> Duration {
        self.limit.saturating_sub(
            self.budget
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .spent(self.clock.elapsed()),
        )
    }
    pub fn expired(&self) -> bool {
        self.remaining().is_zero()
    }
    pub fn interrupt(&self) -> bool {
        self.cancel.is_cancelled() || self.expired()
    }
    pub async fn deadline(&self) {
        loop {
            if self.interrupt() {
                return;
            }
            tokio::select! {
                _ = self.cancel.cancelled() => return,
                _ = self.changed.notified() => (),
                _ = tokio::time::sleep(self.remaining()) => (),
            }
        }
    }
}
