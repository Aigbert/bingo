//! Polling marks a generation stale; a turn boundary replaces it.
use super::*;
use bingo_plugin_host::stamps::{Stamps, stamps};
use serde_json::json;
use std::time::Duration;

pub(super) struct Watch {
    before: Stamps,
    stale: bool,
}
impl Manager {
    pub(crate) async fn prepare(&self) {
        self.poll().await;
        self.reload_stale().await;
    }
    pub(super) fn watch(self: &Arc<Self>) {
        let watches = self
            .modules()
            .iter()
            .filter(|module| module.found.origin == PluginOrigin::Explicit)
            .filter_map(|module| {
                let root = module.found.root.canonicalize().ok()?;
                stamps(&root).ok().map(|before| {
                    (
                        root,
                        Watch {
                            before,
                            stale: false,
                        },
                    )
                })
            })
            .collect();
        *self
            .watches
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = watches;
        let weak = Arc::downgrade(self);
        let stop = self.stop.clone();
        self.tasks.lock().unwrap_or_else(|error| error.into_inner()).push(tokio::spawn(async move {
            let mut clock = tokio::time::interval_at(tokio::time::Instant::now() + Duration::from_secs(1), Duration::from_secs(1));
            clock.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    biased;
                    _ = stop.cancelled() => break,
                    _ = clock.tick() => if let Some(manager) = weak.upgrade() { manager.poll().await; } else { break; },
                }
            }
        }));
    }
    async fn poll(&self) {
        let roots: Vec<_> = self
            .watches
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .keys()
            .cloned()
            .collect();
        for root in roots {
            let scan = root.clone();
            let Ok(Ok(after)) = tokio::task::spawn_blocking(move || stamps(&scan)).await else {
                continue;
            };
            let mut watches = self
                .watches
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if let Some(watch) = watches.get_mut(&root)
                && watch.before != after
            {
                watch.before = after;
                if !watch.stale {
                    watch.stale = true;
                    self.notices.push(Notice::warn(
                        "PLUGIN_STALE",
                        format!(
                            "{}: module changed; reload waits for turn.complete",
                            root.display()
                        ),
                    ));
                }
            }
        }
    }
    async fn reload_stale(&self) {
        let roots: Vec<_> = self
            .watches
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .iter()
            .filter(|(_, watch)| watch.stale)
            .map(|(root, _)| root.clone())
            .collect();
        for root in roots {
            if let Err(error) = self.reload_root(&root).await {
                self.notices
                    .push(Notice::warn("PLUGIN_RELOAD", error.message));
            }
        }
    }
    pub async fn reload_root(&self, root: &Path) -> Result<(), KernelError> {
        let _guard = self.reloading.lock().await;
        let root = root.canonicalize().map_err(failed)?;
        let old = self
            .modules()
            .into_iter()
            .find(|module| module.found.root.canonicalize().ok().as_ref() == Some(&root))
            .ok_or_else(|| failed("module directory is not loaded"))?;
        let mut found = discovery::read(&root, &self.notices)
            .ok_or_else(|| failed("module manifest could not be read"))?;
        if bingo_plugin_host::runtime::runtime_of(&found.manifest, true)
            != Ok(PluginRuntime::Module)
        {
            return Err(failed(format!(
                "{}: runtime changed; restart required",
                found.name
            )));
        }
        found.origin = old.found.origin;
        old.close().await;
        if let Some(dollar) = self.dollar.get() {
            dollar.reload(&old.found.name).await;
        }
        let config = self
            .settings
            .get(&found.name)
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        let module = Module::load(
            found,
            config,
            self.limits.clone(),
            &self.env,
            self.switched_off.contains(&old.found.name),
        )
        .await;
        self.replace(&old, module.clone());
        self.publish_callbacks(module.clone());
        let host = self
            .host
            .get()
            .ok_or_else(|| failed("module host is unavailable"))?;
        host.catalog(CatalogKind::Plugins).await?;
        if let Err(reason) = module.admission.decision() {
            return Err(failed(reason));
        }
        self.restart_sessions(&module, host).await?;
        self.notices.push(Notice::warn(
            "PLUGIN_RELOADED",
            format!("the {} plugin reloaded", module.found.name),
        ));
        self.mark_reloaded(&root)
    }
    fn mark_reloaded(&self, root: &Path) -> Result<(), KernelError> {
        if let Some(watch) = self
            .watches
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .get_mut(root)
        {
            watch.before = stamps(root).map_err(failed)?;
            watch.stale = false;
        }
        Ok(())
    }
    fn replace(&self, old: &Arc<Module>, module: Arc<Module>) {
        if let Some(slot) = self
            .modules
            .write()
            .unwrap_or_else(|error| error.into_inner())
            .iter_mut()
            .find(|slot| Arc::ptr_eq(slot, old))
        {
            *slot = module;
        }
    }
    async fn restart_sessions(
        &self,
        module: &Arc<Module>,
        host: &HostHandle,
    ) -> Result<(), KernelError> {
        if !module.admission.allowed() {
            return Ok(());
        }
        for session in host.sessions(SessionFilter::default()).await? {
            if !self
                .subscriptions
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .contains(&session.id)
            {
                continue;
            }
            let cx = HookContext {
                item: None,
                session: session.id.clone(),
                turn: None,
                cwd: session.cwd.into(),
                provider: None,
                model: None,
                host: host.clone(),
            };
            crate::link::Link(module.clone())
                .observe(
                    catalogue::SESSION_START,
                    &cx,
                    &json!({"session":session.id}),
                )
                .await;
        }
        Ok(())
    }
}
fn failed(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::InvalidInput, error.to_string())
}

pub(super) struct Boundary(pub Weak<Manager>);
#[async_trait]
impl Interceptor for Boundary {
    fn id(&self) -> &str {
        "bingo.mods.reload"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            render: None,
            patterns: [
                "turn.complete",
                "session.start",
                "session.end",
                "session.attach",
                "session.detach",
            ]
            .into_iter()
            .filter_map(|pattern| catalogue::Pattern::parse(pattern).ok())
            .collect(),
            prefilter: Value::Null,
        }
    }
    fn owns_deadline(&self) -> bool {
        true
    }
    async fn observe(&self, entry: &catalogue::Entry, cx: &HookContext, input: &Value) {
        if let Some(manager) = self.0.upgrade() {
            if let Some(dollar) = manager.dollar.get() {
                if entry == catalogue::SESSION_END {
                    dollar.end(&cx.session).await;
                }
                if [catalogue::SESSION_ATTACH, catalogue::SESSION_DETACH].contains(&entry)
                    && let Ok(input) = serde_json::from_value(input.clone())
                    && let Err(reason) = dollar
                        .surface(&cx.host, input, entry == catalogue::SESSION_ATTACH)
                        .await
                {
                    manager.notices.push(Notice::warn("MODS_SURFACE", reason));
                }
            }
            let mut subscriptions = manager
                .subscriptions
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if entry == catalogue::SESSION_START {
                subscriptions.insert(cx.session.clone());
            }
            if entry == catalogue::SESSION_END {
                subscriptions.remove(&cx.session);
            }
        }
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        _: &HookContext,
        input: Value,
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let result = next.run(input).await;
        if let Some(manager) = self.0.upgrade() {
            manager.reload_stale().await;
        }
        result
    }
}
