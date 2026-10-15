//! Catalogue declarations bind a chord to a complete command line.
use super::*;

pub(super) struct NextCommand {
    pub host: Weak<Host>,
    pub model: bool,
}
#[async_trait::async_trait]
impl Command for NextCommand {
    fn spec(&self) -> CommandSpec {
        let mut spec = super::spec(
            if self.model {
                "model next"
            } else {
                "think next"
            },
            if self.model {
                "next pinned model"
            } else {
                "next reasoning effort"
            },
            ArgSpec::None,
            true,
        );
        spec.keys = vec![if self.model { "alt+m" } else { "alt+t" }.into()];
        spec
    }
    async fn is_instant(&self, _: &str, state: &SessionState) -> bool {
        if self.model {
            super::model::ModelCommand {
                host: self.host.clone(),
            }
            .is_instant("next", state)
            .await
        } else {
            true
        }
    }
    async fn run(&self, _: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        if self.model {
            super::model::ModelCommand {
                host: self.host.clone(),
            }
            .run("next", cx)
            .await
        } else {
            super::think::ThinkCommand {
                host: self.host.clone(),
            }
            .run("next", cx)
            .await
        }
    }
}
pub(super) fn after<'a>(values: &'a [String], current: &str) -> Option<&'a str> {
    if values.is_empty() {
        return None;
    }
    let index = values
        .iter()
        .position(|value| value == current)
        .map_or(0, |index| (index + 1) % values.len());
    Some(&values[index])
}
#[cfg(test)]
mod tests {
    #[test]
    fn cycles_in_order_and_starts_at_first_when_absent() {
        let values = vec!["a".into(), "b".into()];
        for (current, expected) in [("a", "b"), ("b", "a"), ("missing", "a")] {
            assert_eq!(super::after(&values, current), Some(expected));
        }
        assert_eq!(super::after(&[], "a"), None);
    }
}
