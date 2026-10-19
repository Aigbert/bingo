//! The wake shelf supplies the facts; this pane owns only the selected delay.
use crate::{
    Schedules,
    wake::{self, Wake},
};
use bingo_sdk::*;
use serde_json::{Value, json};
use std::sync::Arc;
const ID: &str = "bingo.schedule";
const REQUEST: &str = "wake.sheet";
const DELAYS: [(&str, i64); 6] = [
    ("10s", 10),
    ("1m", 60),
    ("5m", 300),
    ("10m", 600),
    ("30m", 1800),
    ("1h", 3600),
];
pub fn tree(
    pending: Option<&Wake>,
    selected: Option<&str>,
    width: u16,
    height: u16,
    now: jiff::Timestamp,
    glyphs: bingo_sdk::glyph::GlyphSet,
) -> View {
    use bingo_sdk::{pick, sheet::Sheet};
    let selected = selected
        .filter(|id| {
            DELAYS.iter().any(|(delay, _)| delay == id) || *id == "cancel" && pending.is_some()
        })
        .or(Some("10s"));
    let rows = rows(pending.is_some(), selected);
    let selected = rows
        .get(pick::selected(&rows, selected))
        .map(|r| r.id.clone());
    let mut view = Sheet {
        head: "Wake up later".into(),
        fact: Some(fact(pending, now)),
        top: vec![],
        rows,
        selected: selected.clone(),
        foot: vec![],
        pick: true,
    }
    .view(width, height, glyphs);
    if let View::Box { layout, .. } = &mut view {
        layout.hints = Some(component::SiteHints::new([
            component::KeyPair::new("↑↓", "when"),
            component::KeyPair::new("⏎", "set"),
            component::KeyPair::new("esc", "close"),
        ]));
        layout.on_key = Some(Action {
            name: CommandSpec::ui_name(ID),
            args: json!({"handler":REQUEST,"event":"press","selected":selected}),
        });
    }
    bingo_sdk::sheet::clicks(view)
}
fn fact(pending: Option<&Wake>, now: jiff::Timestamp) -> View {
    pending
        .map(|wake| {
            let text = format!(
                "wakes in {}",
                bingo_sdk::remaining(
                    wake.at
                        .as_millisecond()
                        .saturating_sub(now.as_millisecond())
                )
            );
            let width = DELAYS
                .iter()
                .map(|(delay, _)| pick::width(&pick::text(format!("wakes in {delay}"), "dim")))
                .max()
                .unwrap_or(0)
                .max(pick::width(&pick::text(&text, "dim")));
            let mut fact = pick::text(
                format!(
                    "{}{text}",
                    " ".repeat(width.saturating_sub(pick::width(&pick::text(&text, "dim"))))
                ),
                "dim",
            );
            if let View::Text { style, .. } = &mut fact {
                style.motion = Some(Box::new(view::TextMotion::Countdown {
                    at_ms: wake.at.as_millisecond(),
                    prefix: "wakes in ".into(),
                }));
            }
            fact
        })
        .unwrap_or_else(|| pick::text("bingo comes back to this session", "dim"))
}

fn rows(pending: bool, selected: Option<&str>) -> Vec<pick::Row> {
    let mut rows = DELAYS
        .iter()
        .map(|(id, _)| {
            pick::Row::new(
                *id,
                if selected == Some(id) {
                    pick::bold(format!("in {id}"), "fg")
                } else {
                    pick::text(format!("in {id}"), "muted")
                },
            )
        })
        .collect::<Vec<_>>();
    if pending {
        rows.push(pick::Row::new(
            "cancel",
            pick::text("cancel the wake-up", "bad"),
        ));
    }
    rows
}
pub struct Open;
#[async_trait::async_trait]
impl Interceptor for Open {
    fn id(&self) -> &str {
        "wake.open"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"command":"wake","args":""}),
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
        let run: catalogue::CommandRun = serde_json::from_value(value.clone()).map_err(error)?;
        if !run.surface.is_some_and(render::SurfaceName::shows_sheets)
            || run.name != "wake"
            || !run.args.trim().is_empty()
        {
            return next.run(value).await;
        }
        publish(&cx.host, &cx.session, None).await?;
        serde_json::to_value(CommandOutcome::Surface {
            action: Action {
                name: "Pane".into(),
                args: json!({"plugin":ID,"id":REQUEST}),
            },
            fallback: None,
        })
        .map_err(error)
    }
}
pub struct Drawing(pub Arc<Schedules>);
#[async_trait::async_trait]
impl Interceptor for Drawing {
    fn id(&self) -> &str {
        ID
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"component":"Pane","props":{"owner":ID}}),
            ..LinkMatcher::at(catalogue::UI_RENDER)
        }
    }
    async fn chain(
        &self,
        _: &catalogue::Entry,
        cx: &HookContext,
        value: Value,
        next: Next<'_, Value, Value>,
    ) -> Result<Value, KernelError> {
        let request: render::RenderRequest =
            serde_json::from_value(value.clone()).map_err(error)?;
        if request.props["owner"] != ID {
            return next.run(value).await;
        }
        serde_json::to_value(tree(
            self.0.wakes().pending(&cx.session).as_ref(),
            request.props["selected"].as_str(),
            request.viewport.columns,
            request.viewport.rows,
            jiff::Timestamp::now(),
            request.viewport.glyphs,
        ))
        .map_err(error)
    }
}
pub struct Handler(pub Arc<Schedules>);
#[async_trait::async_trait]
impl Command for Handler {
    fn spec(&self) -> CommandSpec {
        CommandSpec::ui(ID)
    }
    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let args: Value = serde_json::from_str(args).map_err(error)?;
        if args["handler"] != REQUEST || !render::vouched() {
            return Ok(CommandOutcome::Handled { handled: true });
        }
        let press = sheet::pressed(&args);
        let key = press
            .as_ref()
            .and_then(sheet::Press::key)
            .unwrap_or_default();
        let clicked = press.as_ref().and_then(sheet::Press::picked);
        let selected = clicked.or_else(|| args["selected"].as_str());
        let key = if clicked.is_some() { "enter" } else { key };
        let message = if key == "enter" {
            self.apply(selected, cx).await?
        } else {
            None
        };
        if args["event"] == "close" || matches!(key, "esc" | "ctrl+c") || message.is_some() {
            cx.host
                .signal(&cx.session, ID, REQUEST, Value::Null)
                .await?;
        } else if matches!(key, "up" | "down") {
            let rows = rows(self.0.wakes().pending(&cx.session).is_some(), selected);
            publish(
                &cx.host,
                &cx.session,
                pick::moved(&rows, selected, key == "down"),
            )
            .await?;
        }
        Ok(match message {
            Some(said) => said.outcome(said.plain()),
            None => CommandOutcome::Handled { handled: true },
        })
    }
}
impl Handler {
    async fn apply(
        &self,
        selected: Option<&str>,
        cx: &CommandContext,
    ) -> Result<Option<toast::Say>, KernelError> {
        if selected == Some("cancel") {
            if self.0.wakes().take(&cx.session).is_none() {
                return Ok(None);
            }
            wake::publish(&cx.host, &cx.session, None).await;
            return Ok(Some(toast::Say::new(["wake-up cancelled"])));
        }
        let Some((label, seconds)) = DELAYS.iter().find(|(id, _)| Some(*id) == selected) else {
            return Ok(None);
        };
        let pending = wake::set(
            jiff::Timestamp::now(),
            jiff::SignedDuration::from_secs(*seconds),
            "Continue where this session left off.".into(),
        );
        self.0.wakes().set(&cx.session, pending.clone());
        wake::publish(&cx.host, &cx.session, Some(&pending)).await;
        Ok(Some(toast::Say::ok(format!("wakes in {label}"))))
    }
}
async fn publish(
    host: &HostHandle,
    session: &SessionId,
    selected: Option<String>,
) -> Result<(), KernelError> {
    let pane = slot::Slot::Pane {
        hints: None,
        id: REQUEST.into(),
        title: String::new(),
        focus: true,
        close_on_escape: true,
        hold_toasts: false,
        asked: true,
        place: vec![slot::Place::Dock],
        size: slot::PaneSize::from_rows(12),
        view: slot::Shown::Pull {
            request: render::PullRequest {
                component: render::Component::Pane,
                request_id: REQUEST.into(),
                props: json!({"owner":ID,"selected":selected}),
                viewport: render::Viewport {
                    glyphs: Default::default(),
                    columns: 0,
                    rows: 0,
                    place: None,
                },
            },
        },
    };
    host.signal(session, ID, REQUEST, slot::with(json!({}), [pane]))
        .await?;
    host.invalidate(session, vec![REQUEST.into()]).await
}
fn error(error: impl std::fmt::Display) -> KernelError {
    KernelError::new(ErrorCode::Internal, error.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wake_rows_use_the_builder_and_cancel_only_a_pending_wake() {
        let now = jiff::Timestamp::UNIX_EPOCH;
        let pending = wake::set(now, jiff::SignedDuration::from_secs(10), String::new());
        for wake in [None, Some(&pending)] {
            let view = tree(wake, Some("10s"), 100, 18, now, Default::default());
            let directory = tempfile::tempdir().unwrap();
            let drawing = Drawing(Arc::new(Schedules::new(directory.path())));
            view::answer_ok(
                render::Component::Pane,
                &json!({}),
                drawing.id(),
                None,
                &view,
            )
            .unwrap();
            assert_eq!(view.fold().contains("cancel the wake-up"), wake.is_some());
            assert!(
                serde_json::to_string(&view)
                    .unwrap()
                    .contains("\"ground\":\"pick\"")
            );
            bingo_sdk::testing::assert_separate_hints(&view);
        }
    }
}
