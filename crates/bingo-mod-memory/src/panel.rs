use crate::{ID, REQUEST, Shared, rows::Group};
use bingo_sdk::*;
use bingo_services::{experience, memory};
use serde_json::{Value, json};

pub struct Open(pub Shared, pub std::path::PathBuf);
#[async_trait::async_trait]
impl Interceptor for Open {
    fn id(&self) -> &str {
        "memory:open"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"command":"memory","args":""}),
            ..LinkMatcher::at(catalogue::COMMAND_RUN)
        }
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        cx: &HookContext,
        value: Value,
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let input: catalogue::CommandRun =
            serde_json::from_value(value.clone()).map_err(crate::error)?;
        if input.name != "memory"
            || !input.args.trim().is_empty()
            || memory::find(&cx.host).is_none()
        {
            return next.run(value).await;
        }
        let groups = groups(&cx.host, &cx.cwd, &self.1).await?;
        let fallback: catalogue::CommandReply =
            serde_json::from_value(next.run(value).await?).map_err(crate::error)?;
        let fallback = match CommandOutcome::from(fallback) {
            CommandOutcome::View { view } => Some(view.fold()),
            CommandOutcome::Applied { message } => message,
            _ => None,
        };
        open(cx).await?;
        self.0.lock().map_err(crate::error)?.insert(
            cx.session.clone(),
            crate::Cursor {
                groups,
                ..Default::default()
            },
        );
        serde_json::to_value(CommandOutcome::Surface {
            action: Action {
                name: "Pane".into(),
                args: json!({"plugin":ID,"id":REQUEST}),
            },
            fallback,
        })
        .map_err(crate::error)
    }
}
pub async fn groups(
    host: &HostHandle,
    cwd: &std::path::Path,
    home: &std::path::Path,
) -> Result<Vec<Group>, KernelError> {
    let listing = memory::find(host)
        .ok_or_else(crate::unavailable)?
        .list(memory::List { cwd: cwd.into() })
        .await?;
    let entries = match experience::find(host) {
        Some(service) => service.list(experience::List { cwd: cwd.into() }).await?,
        None => vec![],
    };
    let mut groups = crate::rows::rows(listing, entries);
    for row in groups.iter_mut().flat_map(|group| &mut group.rows) {
        if let crate::rows::Target::Instruction(path) = &row.target {
            row.label = crate::rows::path_label(path, home, cwd);
        }
    }
    Ok(groups)
}
async fn open(cx: &HookContext) -> Result<(), KernelError> {
    let mut size = slot::PaneSize::from_rows(20);
    size.columns.ideal = slot::Length::Percent(100);
    let slot = slot::Slot::Pane {
        id: REQUEST.into(),
        title: "memory".into(),
        focus: true,
        close_on_escape: false,
        hold_toasts: false,
        asked: true,
        place: vec![slot::Place::Overlay],
        size,
        view: slot::Shown::Pull {
            request: render::PullRequest {
                component: render::Component::Pane,
                request_id: REQUEST.into(),
                props: json!({"owner":ID}),
                viewport: render::Viewport {
                    columns: 0,
                    rows: 0,
                    place: None,
                },
            },
        },
    };
    cx.host
        .signal(&cx.session, ID, REQUEST, slot::with(json!({}), [slot]))
        .await?;
    cx.host.invalidate(&cx.session, vec![REQUEST.into()]).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_the_bare_command_is_wrapped() {
        let matcher = Open(Shared::default(), Default::default()).matcher();
        for (command, args, expected) in [
            ("memory", "", true),
            ("memory", "forget rule", false),
            ("think", "maximum", false),
            ("bash", "", false),
            ("plugins", "disable bingo.mod.memory", false),
        ] {
            assert_eq!(
                matcher.matches(
                    catalogue::COMMAND_RUN,
                    &json!({"command":command,"args":args})
                ),
                expected
            );
        }
    }
}
