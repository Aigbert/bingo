//! One decision per discovered generation, claimed before it is adjudicated.
use crate::{discovery::Found, grant::grant};
use bingo_sdk::*;
use std::sync::Mutex;

#[derive(Default)]
enum State {
    #[default]
    Pending,
    Checking(PluginGrant),
    Allowed(PluginGrant),
    Refused(Option<PluginGrant>, String),
}
#[derive(Default)]
pub struct Admission(Mutex<State>);
impl Admission {
    pub fn refuse(&self, reason: String) {
        self.finish(Err(reason));
    }
    pub fn claim(&self, found: &Found, runtime: PluginRuntime, trust: Option<&PluginTrust>) -> Option<PluginRegistration> {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if !matches!(*state, State::Pending) { return None; }
        let grant = grant(found.origin, trust, &found.manifest.uses);
        *state = State::Checking(grant.clone());
        Some(PluginRegistration { name: found.name.clone(), version: found.manifest.version.clone(), execution: PluginExecution { origin: found.origin, runtime, grant } })
    }
    pub fn finish(&self, result: Result<PluginGrant, String>) {
        let mut state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let previous = match &*state { State::Checking(grant) | State::Allowed(grant) => Some(grant.clone()), State::Refused(grant, _) => grant.clone(), State::Pending => None };
        *state = match result { Ok(grant) => State::Allowed(grant), Err(reason) => State::Refused(previous, reason) };
    }
    pub fn allowed(&self) -> bool {
        matches!(*self.0.lock().unwrap_or_else(|e| e.into_inner()), State::Allowed(_))
    }
    pub fn status(&self, found: &Found, runtime: PluginRuntime, from: &str) -> PluginStatus {
        let state = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let (enabled, grant, reason) = match &*state {
            State::Pending => (false, None, Some("registration pending".into())),
            State::Checking(grant) => (false, Some(grant.clone()), Some("registration pending".into())),
            State::Allowed(grant) => (true, Some(grant.clone()), None),
            State::Refused(grant, reason) => (false, grant.clone(), Some(reason.clone())),
        };
        PluginStatus { id: found.name.clone(), version: found.manifest.version.clone(), enabled, reason, from: from.into(), execution: grant.map(|grant| PluginExecution { origin: found.origin, runtime, grant }) }
    }
}
