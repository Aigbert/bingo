//! Facts are observed after publication, through one ordered owned queue.
use super::*;
use futures::FutureExt;
use serde_json::Value;

pub(crate) enum Work {
    Task(BoxFuture<'static, ()>),
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

pub(crate) async fn work(work: Work, hooks: &crate::turn::HookSet) {
    let (entry, value, cx) = match work {
        Work::Task(task) => return task.await,
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
        let failed = if link.external.is_some() {
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
