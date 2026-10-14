//! Test overrides are instance-owned and never read the process environment.
#[cfg(test)]
mod tests;
use bingo_sdk::SessionId;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Mutex,
};
use tokio::sync::watch;

pub(crate) type Started = watch::Sender<BTreeSet<SessionId>>;

pub struct Environment {
    clock: Mutex<(jiff::Timestamp, tokio::time::Instant)>,
    env: Mutex<BTreeMap<String, String>>,
    starts: Mutex<BTreeMap<String, watch::Receiver<BTreeSet<SessionId>>>>,
}
impl Default for Environment {
    fn default() -> Self {
        Self {
            clock: Mutex::new((jiff::Timestamp::UNIX_EPOCH, tokio::time::Instant::now())),
            env: Mutex::default(),
            starts: Mutex::default(),
        }
    }
}
impl Environment {
    /// A replacement host keeps the mocks, but owns a new set of module generations.
    pub fn reset_starts(&self) {
        self.starts
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }
    pub(crate) fn track_start(&self, name: &str, observes: bool) -> Option<Started> {
        let mut starts = self.starts.lock().unwrap_or_else(|e| e.into_inner());
        starts.remove(name);
        observes.then(|| {
            let (send, receive) = watch::channel(BTreeSet::new());
            starts.insert(name.into(), receive);
            send
        })
    }
    /// Wait for every admitted observer, including completions before this call.
    pub async fn wait_for_start(&self, session: &SessionId) -> Result<(), String> {
        let starts = self
            .starts
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        for (name, mut started) in starts {
            started
                .wait_for(|sessions| sessions.contains(session))
                .await
                .map_err(|_| format!("{name}: session.start observer closed"))?;
        }
        Ok(())
    }
    pub fn now(&self) -> Result<i64, String> {
        let (base, start) = *self.clock.lock().unwrap_or_else(|e| e.into_inner());
        base.checked_add(tokio::time::Instant::now().duration_since(start))
            .map(|time| time.as_millisecond())
            .map_err(|e| e.to_string())
    }
    pub fn set(&self, time: &str) -> Result<(), String> {
        *self.clock.lock().unwrap_or_else(|e| e.into_inner()) = (
            time.parse::<jiff::Timestamp>().map_err(|e| e.to_string())?,
            tokio::time::Instant::now(),
        );
        Ok(())
    }
    pub fn env(&self, name: &str) -> Option<String> {
        self.env
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(name)
            .cloned()
    }
    pub fn seed_env(&self, values: BTreeMap<String, String>) {
        *self.env.lock().unwrap_or_else(|e| e.into_inner()) = values;
    }
    pub fn set_env(&self, name: String, value: String) {
        self.env
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(name, value);
    }
}
/// Keep the paused executor from advancing time while QuickJS is on its thread.
pub struct Paused(std::sync::mpsc::Sender<()>);
impl Paused {
    pub fn new() -> Self {
        let (send, receive) = std::sync::mpsc::channel();
        tokio::task::spawn_blocking(move || {
            let _ = receive.recv();
        });
        tokio::time::pause();
        Self(send)
    }
}
impl Default for Paused {
    fn default() -> Self {
        Self::new()
    }
}
impl Drop for Paused {
    fn drop(&mut self) {
        tokio::time::resume();
        let _ = self.0.send(());
    }
}
