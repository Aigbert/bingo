//! Durable obligations and their process-local, owned followers.
use crate::{message, names, watch};
use async_trait::async_trait;
use bingo_alarm::{Alarm, Alarms};
use bingo_sdk::{
    Attachment, Delivery, Hook, HookContext, HookMatcher, HookPoint, HostHandle, Input, IntentId,
    Phase, SessionId, ToolError, TurnId,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, MutexGuard};

pub(crate) const PLUGIN: &str = "bingo.agents";
const PREFIX: &str = "report:";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Obligation {
    child: SessionId,
    turn: TurnId,
}
impl Obligation {
    fn kind(&self) -> String {
        format!("{PREFIX}{}", self.child)
    }
}

type Key = (SessionId, SessionId);
#[derive(Debug, Default)]
pub(crate) struct Reports {
    tasks: Mutex<Alarms<Key>>,
}
impl Reports {
    pub(crate) async fn promise(
        &self,
        mut attachment: Attachment,
        host: HostHandle,
        parent: SessionId,
        cancel: &bingo_sdk::CancellationToken,
    ) -> Result<(), ToolError> {
        let turn = watch::started(&host, &mut attachment, cancel).await?;
        let obligation = Obligation {
            child: attachment.session.clone(),
            turn,
        };
        let payload = serde_json::to_value(&obligation)
            .map_err(|error| ToolError::Failed(error.to_string()))?;
        self.tasks()
            .cancel(&(parent.clone(), obligation.child.clone()));
        host.extend(&parent, PLUGIN, &obligation.kind(), payload)
            .await
            .map_err(|error| ToolError::Failed(error.message))?;
        self.arm(host, parent, obligation, Some(attachment));
        Ok(())
    }

    fn arm(
        &self,
        host: HostHandle,
        parent: SessionId,
        obligation: Obligation,
        attachment: Option<Attachment>,
    ) {
        let key = (parent.clone(), obligation.child.clone());
        let mut tasks = self.tasks();
        if tasks.pending(&key) {
            return;
        }
        tasks.arm(
            key,
            Alarm::new(
                tokio::time::Instant::now(),
                deliver(host, parent, obligation, attachment),
            ),
        );
    }

    async fn resume(&self, host: &HostHandle, parent: &SessionId) {
        let Some(state) = watch::snapshot(host, parent).await else {
            return;
        };
        let Some(kinds) = state.extensions.get(PLUGIN) else {
            return;
        };
        for (kind, payload) in kinds {
            if !kind.starts_with(PREFIX) || payload.is_null() {
                continue;
            }
            if let Ok(obligation) = serde_json::from_value::<Obligation>(payload.clone())
                && kind == &obligation.kind()
            {
                self.arm(host.clone(), parent.clone(), obligation, None);
            }
        }
    }

    fn end(&self, parent: &SessionId) {
        self.tasks().retain(|(owner, _)| owner != parent);
    }
    pub(crate) fn stop(&self) {
        self.tasks().clear();
    }
    fn tasks(&self) -> MutexGuard<'_, Alarms<Key>> {
        self.tasks.lock().unwrap_or_else(|held| held.into_inner())
    }
}

async fn deliver(
    host: HostHandle,
    parent: SessionId,
    obligation: Obligation,
    attachment: Option<Attachment>,
) {
    let attachment = match attachment {
        Some(attachment) => Ok(attachment),
        None => watch::follow(&host, &obligation.child).await,
    };
    let (name, text) = match attachment {
        Ok(mut attachment) => {
            let name = names::name_of(&attachment.snapshot.summary).to_owned();
            match watch::awaited_reply(&host, &mut attachment, &obligation.turn).await {
                Ok(reply) => (name, watch::finished(&reply)),
                Err(error) => {
                    tracing::warn!(%error, "a report remains owed");
                    return;
                }
            }
        }
        Err(error) if error.code == bingo_sdk::ErrorCode::SessionNotFound => (
            obligation.child.to_string(),
            "the child is gone; its pending report has been cleared.".into(),
        ),
        Err(error) => {
            tracing::warn!(%error, "a report could not reopen its child");
            return;
        }
    };
    let input = Input::text(text, message::origin(Some(name)));
    if let Err(error) = host
        .deliver(&parent, IntentId::mint(), input, Delivery::Wake)
        .await
    {
        tracing::warn!(%error, "a report reached no parent");
        return;
    }
    if let Err(error) = host
        .extend(&parent, PLUGIN, &obligation.kind(), serde_json::Value::Null)
        .await
    {
        tracing::warn!(%error, "a delivered report could not be cleared");
    }
}

#[derive(Debug)]
pub(crate) struct ReportHook(pub Arc<Reports>);
#[async_trait]
impl Hook for ReportHook {
    fn id(&self) -> &str {
        "agents.reports"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![HookPoint::Session],
            tool: None,
        }
    }
    async fn on_session(&self, phase: Phase, cx: &HookContext) {
        match phase {
            Phase::Start => self.0.resume(&cx.host, &cx.session).await,
            Phase::End => self.0.end(&cx.session),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_obligation_has_one_persisted_shape() {
        let expected: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/report.json")).unwrap();
        let obligation: Obligation = serde_json::from_value(expected.clone()).unwrap();
        assert_eq!(obligation.kind(), "report:ses_scout");
        assert_eq!(serde_json::to_value(obligation).unwrap(), expected);
    }

    use crate::tests::{Fleet, Recorder, assistant, hook_context, turn_completed};

    async fn settle() {
        for _ in 0..32 {
            tokio::task::yield_now().await;
        }
    }

    async fn promised(fleet: &Fleet, parent: &SessionId, child: &SessionId) {
        let first = Reports::default();
        let attachment = watch::follow(&fleet.handle(), child).await.unwrap();
        first
            .promise(
                attachment,
                fleet.handle(),
                parent.clone(),
                &bingo_sdk::CancellationToken::new(),
            )
            .await
            .unwrap();
        drop(first);
    }

    #[tokio::test]
    async fn a_second_owner_delivers_a_finished_child_and_clears_the_record() {
        let fleet = Fleet::default();
        let parent = fleet.root();
        let child = fleet.child(&parent, "scout");
        fleet.set_busy(&child, true);
        promised(&fleet, &parent, &child).await;
        fleet.said(&child, "the build is green");
        let second = ReportHook(Arc::default());
        let cx = hook_context(&parent, fleet.handle());
        second.on_session(Phase::Start, &cx).await;
        settle().await;
        assert_eq!(fleet.delivered().len(), 1);
        let Input::Text { text, .. } = &fleet.delivered()[0].1 else {
            panic!("reply")
        };
        assert!(text.contains("the build is green"));
        let state = watch::snapshot(&fleet.handle(), &parent).await.unwrap();
        assert!(state.extensions.get(PLUGIN).is_none_or(|kinds| {
            kinds
                .get(&format!("report:{child}"))
                .is_none_or(serde_json::Value::is_null)
        }));
        second.on_session(Phase::Start, &cx).await;
        settle().await;
        assert_eq!(fleet.delivered().len(), 1);
    }

    #[tokio::test]
    async fn a_second_owner_follows_a_running_child_to_its_end() {
        let fleet = Fleet::default();
        let parent = fleet.root();
        let child = fleet.child(&parent, "scout");
        fleet.set_busy(&child, true);
        promised(&fleet, &parent, &child).await;
        fleet.script([assistant("done after restart"), turn_completed()]);
        let second = ReportHook(Arc::default());
        second
            .on_session(Phase::Start, &hook_context(&parent, fleet.handle()))
            .await;
        settle().await;
        assert_eq!(fleet.delivered().len(), 1);
        let Input::Text { text, .. } = &fleet.delivered()[0].1 else {
            panic!("reply")
        };
        assert!(text.contains("done after restart"));
    }

    #[tokio::test]
    async fn a_gone_child_is_reported_and_parent_end_cancels_followers() {
        let fleet = Fleet::default();
        let parent = fleet.root();
        let child = fleet.child(&parent, "scout");
        fleet.set_busy(&child, true);
        promised(&fleet, &parent, &child).await;
        let second = ReportHook(Arc::default());
        let cx = hook_context(&parent, fleet.handle());
        second.on_session(Phase::Start, &cx).await;
        settle().await;
        assert!(second.0.tasks().pending(&(parent.clone(), child.clone())));
        second.on_session(Phase::End, &cx).await;
        assert!(!second.0.tasks().contains(&(parent.clone(), child.clone())));
        assert!(fleet.delivered().is_empty());
        fleet.handle().delete(&child).await.unwrap();
        let third = ReportHook(Arc::default());
        third.on_session(Phase::Start, &cx).await;
        settle().await;
        assert_eq!(Recorder::new(&fleet).delivered().len(), 1);
        let Input::Text { text, .. } = &fleet.delivered()[0].1 else {
            panic!("reply")
        };
        assert!(text.contains("child is gone"));
    }
}
