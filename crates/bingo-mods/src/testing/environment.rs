//! Test overrides are instance-owned and never read the process environment.
use std::{collections::BTreeMap, sync::Mutex};
pub struct Environment {
    clock: Mutex<(jiff::Timestamp, tokio::time::Instant)>,
    env: Mutex<BTreeMap<String, String>>,
}
impl Default for Environment {
    fn default() -> Self {
        Self {
            clock: Mutex::new((jiff::Timestamp::UNIX_EPOCH, tokio::time::Instant::now())),
            env: Mutex::default(),
        }
    }
}
impl Environment {
    pub fn now(&self) -> Result<String, String> {
        let (base, start) = *self.clock.lock().unwrap_or_else(|e| e.into_inner());
        base.checked_add(tokio::time::Instant::now().duration_since(start))
            .map(|time| time.to_string())
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
