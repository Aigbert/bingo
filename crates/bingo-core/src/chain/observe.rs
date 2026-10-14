//! Facts are observed after publication, through one ordered owned queue.
use super::*;
use futures::FutureExt;
use serde_json::Value;

pub(crate) enum Work {
    Snapshot(Box<crate::turn::HookSet>, Box<Work>),
    Calling(Vec<invocation::CallingHook>, Box<Work>),
    Frame(Box<Frame>, HookContext),
    Named(LinkPoint, Value, HookContext),
}

pub(crate) fn observed(event: &Event) -> Option<(LinkPoint, Value)> {
    match event {
        Event::TurnStarted {
            turn,
            inputs,
            origin,
        } => Some((
            catalogue::TURN_START,
            serde_json::to_value(catalogue::TurnStart {
                turn: turn.clone(),
                inputs: inputs.clone(),
                origin: *origin,
            })
            .ok()?,
        )),
        Event::TurnUsage {
            turn,
            usage,
            context,
        } => Some((
            catalogue::SESSION_MEASURE,
            serde_json::to_value(catalogue::Measure {
                turn: turn.clone(),
                usage: *usage,
                context: *context,
            })
            .ok()?,
        )),
        _ => None,
    }
}

pub(crate) fn capture(work: Work) -> Work {
    let calling = invocation::current();
    if calling.is_empty() {
        work
    } else {
        Work::Calling(calling, Box::new(work))
    }
}
pub(crate) async fn work(work: Work, hooks: &crate::turn::HookSet) {
    let (calling, work) = match work {
        Work::Calling(calling, work) => (calling, *work),
        work => (Vec::new(), work),
    };
    match work {
        Work::Snapshot(snapshot, work) => {
            invocation::scope(calling, dispatch(*work, &snapshot)).await
        }
        work => invocation::scope(calling, dispatch(work, hooks)).await,
    }
}
async fn dispatch(work: Work, hooks: &crate::turn::HookSet) {
    let (entry, value, cx) = match work {
        Work::Calling(_, _) | Work::Snapshot(_, _) => return,
        Work::Frame(frame, cx) => {
            for hook in hooks.at(HookPoint::Event, None).await {
                hook.on_event(&frame, &cx).await;
            }
            let Some((entry, value)) = observed(&frame.event) else {
                return;
            };
            (entry, value, cx)
        }
        Work::Named(entry, value, cx) => (entry, value, cx),
    };
    for link in hooks.links().await {
        if !link.interceptor.matcher().matches(entry, &value) {
            continue;
        }
        let future = std::panic::AssertUnwindSafe(link.interceptor.observe(entry, &cx, &value))
            .catch_unwind();
        let failed = if link.external.is_some() && !link.interceptor.owns_deadline() {
            !matches!(
                tokio::time::timeout(HOOK_DEADLINE, future).await,
                Ok(Ok(()))
            )
        } else {
            future.await.is_err()
        };
        if failed {
            let name = link
                .external
                .as_deref()
                .unwrap_or_else(|| link.interceptor.id());
            let _ = cx
                .host
                .notice(
                    Level::Warn,
                    "INTERCEPTOR_SKIPPED",
                    &format!("{name} ({}): observer failed", entry.name),
                )
                .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn additive_spend_is_not_a_context_measurement() {
        assert!(
            observed(&Event::UsageCharged {
                turn: None,
                plugin: "test".into(),
                usage: Usage::default()
            })
            .is_none()
        );
        let event = Event::TurnUsage {
            turn: TurnId::from_raw("t"),
            usage: Usage::default(),
            context: ContextUsage::default(),
        };
        let (entry, value) = observed(&event).unwrap();
        assert_eq!(entry, catalogue::SESSION_MEASURE);
        assert_eq!(value["turn"], "t");
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    struct Observed(std::sync::atomic::AtomicUsize);
    #[async_trait::async_trait]
    impl Interceptor for Observed {
        fn id(&self) -> &str {
            "counter"
        }
        fn matcher(&self) -> LinkMatcher {
            LinkMatcher::at(catalogue::SESSION_MEASURE)
        }
        async fn observe(&self, _: &catalogue::Entry, _: &HookContext, _: &Value) {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    }
    #[tokio::test]
    async fn a_queued_turn_observation_keeps_its_snapshot() {
        let observer = Arc::new(Observed(0.into()));
        let mut hooks = crate::turn::HookSet::default();
        hooks.links.push(Link {
            plugin: "counter".into(),
            external: None,
            placement: plugin::Placement::Outer,
            tier: Tier::Full,
            interceptor: observer.clone(),
        });
        let snapshot = hooks.snapshot().await;
        hooks.running.write().unwrap().insert("counter".into());
        let cx = HookContext {
            item: None,
            session: SessionId::from_raw("test"),
            turn: Some(TurnId::from_raw("turn")),
            cwd: Default::default(),
            provider: None,
            model: None,
            host: testing::NoHost::handle(),
        };
        let queued = Work::Named(catalogue::SESSION_MEASURE, Value::Null, cx.clone());
        work(Work::Snapshot(Box::new(snapshot), Box::new(queued)), &hooks).await;
        work(
            Work::Named(catalogue::SESSION_MEASURE, Value::Null, cx),
            &hooks,
        )
        .await;
        assert_eq!(observer.0.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
