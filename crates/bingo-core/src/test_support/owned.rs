//! Fakes that announce the exact await an interrupt must reach.

use super::*;
use tokio::sync::Notify;

pub struct Hung {
    pub entered: Notify,
    pub point: HookPoint,
}

impl Hung {
    pub fn at(point: HookPoint) -> Arc<Self> {
        Arc::new(Self {
            entered: Notify::new(),
            point,
        })
    }

    async fn wait<T>(&self) -> T {
        self.entered.notify_one();
        std::future::pending().await
    }
}

#[async_trait]
impl Hook for Hung {
    fn id(&self) -> &str {
        "hung"
    }
    fn matcher(&self) -> HookMatcher {
        HookMatcher {
            points: vec![self.point],
            tool: None,
        }
    }
    async fn before_tool(&self, _: &mut ToolCall, _: &HookContext) -> HookOutcome {
        self.wait().await
    }
    async fn on_submit(&self, input: &mut Input, _: &HookContext) -> HookOutcome {
        if matches!(input, Input::Text { text, .. } if text == "hang") {
            self.wait().await
        } else {
            HookOutcome::Continue
        }
    }
}

#[async_trait]
impl PermissionPolicy for Hung {
    fn id(&self) -> &str {
        "hung"
    }
    async fn decide(&self, _: PolicyInput<'_>) -> Decision {
        self.wait().await
    }
}

#[async_trait]
impl Compactor for Hung {
    async fn compact(
        &self,
        _: CompactContext<'_>,
        _: CompactReason,
    ) -> Result<Compaction, CompactError> {
        self.wait().await
    }
}

#[async_trait]
impl Command for Hung {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            name: "hang".into(),
            aliases: vec![],
            hint: String::new(),
            args: ArgSpec::None,
            instant: false,
            family: "test".into(),
        }
    }
    async fn run(&self, _: &str, _: &CommandContext) -> Result<CommandOutcome, KernelError> {
        self.wait().await
    }
}
