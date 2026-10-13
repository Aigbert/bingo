use super::{
    HostCall,
    scripts::{ImmediateScripts, Scripts},
    table::Table,
};
use crate::service::Services;
use crate::{GatewayEvent, HostHandle, SessionSummary};
use futures::channel::mpsc;
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Default)]
pub(super) struct Inner {
    pub calls: Mutex<Vec<HostCall>>,
    pub scripts: Mutex<Scripts>,
    pub immediate: Mutex<ImmediateScripts>,
    pub sessions: Mutex<Option<Table>>,
    pub services: Services,
    pub gateway: Mutex<Option<mpsc::UnboundedReceiver<GatewayEvent>>>,
}

/// A recording, scriptable host. Every HostApi verb is recorded, including
/// gateway and service lookups. Scripts run without holding the recording or
/// script locks, and may await, inspect calls, or re-enter this host.
///
/// Unscripted async verbs answer exactly as NoHost does. `with_sessions`
/// enables a session table for sessions/open/close/delete: close detaches and
/// delete removes. Scripts override the table. Services always share one map;
/// opening an occupied key fails. Gateway streams are empty unless a test
/// supplies one with `gateway` or `on_gateway_events`.
#[derive(Clone, Default)]
pub struct TestHost {
    pub(super) inner: Arc<Inner>,
}

impl TestHost {
    pub fn with_sessions(sessions: Vec<SessionSummary>) -> Self {
        let host = Self::default();
        *locked(&host.inner.sessions) = Some(Table::new(sessions));
        host
    }

    pub fn handle(&self) -> HostHandle {
        HostHandle(Arc::new(self.clone()))
    }

    pub fn calls(&self) -> Vec<HostCall> {
        locked(&self.inner.calls).clone()
    }

    pub fn services(&self) -> &Services {
        &self.inner.services
    }

    /// The next gateway subscription receives this stream, including anything
    /// sent before subscribing. Dropping the sender ends it. For multiple or
    /// paced subscriptions, use `on_gateway_events`.
    pub fn gateway(&self) -> mpsc::UnboundedSender<GatewayEvent> {
        let (sender, receiver) = mpsc::unbounded();
        *locked(&self.inner.gateway) = Some(receiver);
        sender
    }

    pub(super) fn record(&self, call: HostCall) {
        locked(&self.inner.calls).push(call);
    }
}

impl std::fmt::Debug for TestHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestHost")
            .field("calls", &self.calls())
            .finish_non_exhaustive()
    }
}

pub(super) fn locked<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value.lock().unwrap_or_else(|held| held.into_inner())
}
