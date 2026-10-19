//! The run's paired clocks: system time in a run, a scene's clock in tests.
use jiff::Timestamp;
use std::{sync::Arc, time::Instant};

pub(super) type Clock = Arc<dyn Fn() -> (Instant, Timestamp) + Send + Sync>;

pub(super) fn system() -> Clock {
    Arc::new(|| (tokio::time::Instant::now().into_std(), Timestamp::now()))
}
