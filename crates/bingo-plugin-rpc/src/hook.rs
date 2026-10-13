//! Observation-only hooks; decision points cross as interceptors.
use std::sync::Arc;

use async_trait::async_trait;
use bingo_sdk::{Frame, Hook, HookContext, HookMatcher, Item, Phase, TurnId};

use crate::connection::Connection;
use crate::notice::Notices;
use crate::wire::{HookObservation, HookObserveParams, HookSite, HookSpec, name};

/// The kernel-visible id of a plugin's hook: the plugin's name and the hook's
/// own. Two plugins may both declare a `guard`, and a refusal still says which
/// one refused.
pub fn hook_id(plugin: &str, hook: &str) -> String {
    format!("{plugin}:{hook}")
}

/// A hook a plugin process declared, bound to the pipe that answers it.
pub struct RemoteHook {
    /// The id the kernel sees, the plugin's name in it; the process is asked
    /// by [`HookSpec::id`].
    id: String,
    spec: HookSpec,
    connection: Arc<Connection>,
}

impl RemoteHook {
    pub fn new(
        plugin: &str,
        spec: HookSpec,
        connection: Arc<Connection>,
        _notices: Arc<Notices>,
    ) -> Self {
        Self {
            id: hook_id(plugin, &spec.id),
            spec,
            connection,
        }
    }

    /// Tell the process, and do not wait: nothing here has an answer to read.
    async fn observe(&self, cx: &HookContext, observation: HookObservation) {
        let params = HookObserveParams {
            event: None,
            id: self.spec.id.clone(),
            site: HookSite::from(cx),
            observation,
        };
        let Ok(value) = serde_json::to_value(params) else {
            return;
        };
        self.connection.notify(name::HOOK_OBSERVE, value).await;
    }
}

#[async_trait]
impl Hook for RemoteHook {
    fn id(&self) -> &str {
        &self.id
    }

    /// Handshake data: asked once, when the process said what it claims.
    fn matcher(&self) -> HookMatcher {
        self.spec.matcher.clone()
    }

    async fn on_turn(&self, phase: Phase, turn: &TurnId, items: &[Item], cx: &HookContext) {
        self.observe(
            cx,
            HookObservation::Turn {
                phase,
                turn: turn.clone(),
                items: items.to_vec(),
            },
        )
        .await;
    }

    async fn on_compact(&self, phase: Phase, cx: &HookContext) {
        self.observe(cx, HookObservation::Compact { phase }).await;
    }

    async fn on_session(&self, phase: Phase, cx: &HookContext) {
        self.observe(cx, HookObservation::Session { phase }).await;
    }

    async fn on_event(&self, frame: &Frame, cx: &HookContext) {
        self.observe(
            cx,
            HookObservation::Event {
                frame: Box::new(frame.clone()),
            },
        )
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{hook_context, unanswering};
    use bingo_sdk::HookPoint;

    fn declared(points: Vec<HookPoint>, tool: Option<&str>) -> HookSpec {
        HookSpec {
            id: "guard".into(),
            matcher: HookMatcher {
                points,
                tool: tool.map(str::to_string),
            },
        }
    }

    fn silent(spec: HookSpec) -> RemoteHook {
        RemoteHook::new("notes", spec, unanswering(), Arc::new(Notices::default()))
    }

    #[test]
    fn a_hook_is_named_for_its_plugin_and_itself() {
        assert_eq!(hook_id("notes", "guard"), "notes:guard");
    }

    /// The matcher is the declaration's, asked once and never over the wire:
    /// it is what keeps an unmatched event off the pipe at all.
    #[tokio::test]
    async fn the_matcher_is_the_one_the_handshake_declared() {
        let remote = silent(declared(vec![HookPoint::BeforeTool], Some("Bash")));
        assert_eq!(remote.id(), "notes:guard");
        assert_eq!(
            remote.matcher(),
            HookMatcher {
                points: vec![HookPoint::BeforeTool],
                tool: Some("Bash".into()),
            }
        );
    }

    /// An observation waits for nothing, so a process that answers nothing
    /// costs nothing: this returns on a clock that never advances.
    #[tokio::test(start_paused = true)]
    async fn an_observation_returns_at_once_however_silent_the_process_is() {
        let notices = Arc::new(Notices::default());
        let remote = RemoteHook::new(
            "notes",
            declared(vec![HookPoint::Session], None),
            unanswering(),
            Arc::clone(&notices),
        );
        remote.on_session(Phase::Start, &hook_context()).await;
        assert!(
            notices.drain().is_empty(),
            "nothing was awaited, so nothing was missed"
        );
    }
}
