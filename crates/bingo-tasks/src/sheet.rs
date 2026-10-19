//! The journal owns the tasks; the pane keeps only its picked identity.
use crate::{Status, Task};
use bingo_sdk::*;
use serde_json::{Value, json};
pub const REQUEST: &str = "tasks.sheet";
const ID: &str = "bingo.tasks";

pub fn tree(
    tasks: &[Task],
    selected: Option<&str>,
    width: u16,
    height: u16,
    glyphs: bingo_sdk::glyph::GlyphSet,
    roster: &[SessionSummary],
) -> View {
    use bingo_sdk::{pick, sheet::Sheet};
    let tasks = ordered(tasks);
    let initial = tasks.first().map(|task| task.id.to_string());
    let selected = selected
        .filter(|id| tasks.iter().any(|task| task.id.to_string() == *id))
        .or(initial.as_deref());
    let rows = tasks
        .iter()
        .map(|task| row(task, selected, glyphs, roster))
        .collect::<Vec<_>>();
    let selected = rows
        .get(pick::selected(&rows, selected))
        .map(|r| r.id.clone());
    let mut view = Sheet {
        head: "Tasks".into(),
        fact: Some(pick::text(
            format!(
                "{} of {} done",
                tasks
                    .iter()
                    .filter(|t| t.status == Status::Completed)
                    .count(),
                tasks.len()
            ),
            "dim",
        )),
        top: vec![],
        rows,
        selected: selected.clone(),
        foot: vec![],
        pick: true,
    }
    .view(width, height, glyphs);
    if let View::Box { layout, .. } = &mut view {
        layout.hints = Some(component::SiteHints::new([
            component::KeyPair::new("↑↓", "task"),
            component::KeyPair::new("esc", "close"),
        ]));
        layout.on_key = Some(Action {
            name: CommandSpec::ui_name(ID),
            args: json!({"handler":REQUEST,"event":"press","selected":selected}),
        });
    }
    bingo_sdk::sheet::clicks(view)
}
fn ordered(tasks: &[Task]) -> Vec<&Task> {
    let mut tasks = tasks.iter().collect::<Vec<_>>();
    tasks.sort_by_key(|task| {
        (
            match task.status {
                Status::InProgress => 0,
                Status::Pending => 1,
                Status::Completed => 2,
            },
            std::cmp::Reverse(task.id),
        )
    });
    tasks
}
fn row(
    task: &Task,
    selected: Option<&str>,
    glyphs: glyph::GlyphSet,
    roster: &[SessionSummary],
) -> pick::Row {
    use bingo_sdk::pick::{line, text};
    let owner = task.owner.as_deref().unwrap_or("bingo");
    let ink = bingo_collab::face::who(
        owner,
        roster
            .iter()
            .find(|row| row.parent.is_none())
            .map(|row| row.id.as_str())
            .unwrap_or_default(),
        roster,
    );
    let mut mark = match task.status {
        Status::Completed => text(glyph::Glyph::Ok.spell(glyphs), "good"),
        Status::Pending => text(glyph::Glyph::Still.spell(glyphs), "dim"),
        Status::InProgress => text(glyph::Glyph::Still.spell(glyphs), ink),
    };
    if task.status == Status::InProgress
        && let View::Text { style, .. } = &mut mark
    {
        style.motion = Some(Box::new(view::TextMotion::Spin));
    }
    let title_ink = if task.status == Status::Completed {
        "dim"
    } else if selected == Some(task.id.to_string().as_str()) {
        "fg"
    } else {
        "muted"
    };
    pick::Row::new(
        task.id.to_string(),
        line([mark, text("  ", "fg"), text(&task.subject, title_ink)]),
    )
    .right(text(owner, ink))
}
pub struct Open;
#[async_trait::async_trait]
impl Interceptor for Open {
    fn id(&self) -> &str {
        "tasks.open"
    }
    fn matcher(&self) -> LinkMatcher {
        LinkMatcher {
            prefilter: json!({"command":"tasks","args":""}),
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
            || run.name != "tasks"
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
pub struct Drawing;
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
        let tasks = crate::journal::read(&cx.host, &cx.session).await?;
        let roster = roster(&cx.host, &cx.session).await?;
        serde_json::to_value(tree(
            &tasks,
            request.props["selected"].as_str(),
            request.viewport.columns,
            request.viewport.rows,
            request.viewport.glyphs,
            &roster,
        ))
        .map_err(error)
    }
}
async fn roster(
    host: &HostHandle,
    session: &SessionId,
) -> Result<Vec<SessionSummary>, KernelError> {
    let Some(service) = bingo_services::agents_control::find(host) else {
        return Ok(vec![]);
    };
    let home = home(host, session).await?;
    let rows = service
        .roster(bingo_services::agents_control::Session {
            session: home.id.clone(),
        })
        .await
        .map_err(error)?;
    Ok(std::iter::once(home)
        .chain(rows.into_iter().map(|row| row.summary))
        .collect())
}
async fn home(host: &HostHandle, session: &SessionId) -> Result<SessionSummary, KernelError> {
    let mut id = session.clone();
    loop {
        let attached = host
            .open(
                SessionSelector::ById { id },
                ClientIdentity {
                    name: ID.into(),
                    surface: String::new(),
                },
                OpenOptions::default(),
            )
            .await?;
        let summary = attached.snapshot.summary;
        if let Some(parent) = &summary.parent {
            id = parent.session.clone();
        } else {
            return Ok(summary);
        }
    }
}
pub struct Handler;
#[async_trait::async_trait]
impl Command for Handler {
    fn spec(&self) -> CommandSpec {
        CommandSpec::ui(ID)
    }
    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        let args: Value = serde_json::from_str(args).map_err(error)?;
        if args["handler"] != REQUEST {
            return Ok(CommandOutcome::Handled { handled: false });
        }
        let press = bingo_sdk::sheet::pressed(&args);
        let key = press
            .as_ref()
            .and_then(bingo_sdk::sheet::Press::key)
            .unwrap_or_default();
        let picked = press.as_ref().and_then(bingo_sdk::sheet::Press::picked);
        if args["event"] == "close" || matches!(key, "esc" | "ctrl+c") {
            cx.host
                .signal(&cx.session, ID, REQUEST, Value::Null)
                .await?;
        } else if picked.is_some() || matches!(key, "up" | "down") {
            let tasks = crate::journal::read(&cx.host, &cx.session).await?;
            let tasks = ordered(&tasks);
            let rows = tasks
                .iter()
                .map(|task| row(task, None, Default::default(), &[]))
                .collect::<Vec<_>>();
            publish(
                &cx.host,
                &cx.session,
                picked
                    .filter(|id| rows.iter().any(|row| row.id == *id))
                    .map(str::to_owned)
                    .or_else(|| pick::moved(&rows, args["selected"].as_str(), key == "down")),
            )
            .await?;
        }
        Ok(CommandOutcome::Handled { handled: true })
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
    #[tokio::test]
    async fn opening_from_a_teammate_resolves_the_same_home() {
        let rows: Value =
            serde_json::from_str(include_str!("../../bingo-collab/tests/fixtures/faces.json"))
                .unwrap();
        let rows = rows[2]["rows"].as_array().unwrap().iter().map(|row| serde_json::from_value::<SessionSummary>(json!({
            "id":row[0], "title":row[1], "cwd":"/workspace", "driver":if row[3] == "room" { "log" } else { "model" }, "key":if row[3] == "room" { Some("rooms/home/design") } else { None }, "createdAt":format!("2026-10-19T00:00:{:02}Z",row[2].as_u64().unwrap()), "updatedAt":"2026-10-19T01:00:00Z",
            "parent":if row[3] == "home" { Value::Null } else { json!({"session":row[4].as_str().unwrap_or("home")}) }
        })).unwrap()).collect::<Vec<_>>();
        let host = testing::TestHost::with_sessions(rows.clone());
        for caller in ["home", "ada", "grandchild"] {
            let summary = home(&host.handle(), &SessionId::from_raw(caller))
                .await
                .unwrap();
            assert_eq!(summary.id.as_str(), "home");
            assert_eq!(
                bingo_collab::face::who("ada", summary.id.as_str(), &rows),
                "crew0"
            );
        }
    }
    #[test]
    fn tasks_use_the_shared_builder_in_status_order() {
        let tasks = (1..=12)
            .map(|id| Task {
                id,
                subject: format!("task {id}"),
                status: if id == 1 {
                    Status::InProgress
                } else {
                    Status::Pending
                },
                description: String::new(),
                active_form: None,
                owner: None,
                blocks: vec![],
                blocked_by: vec![],
                metadata: Default::default(),
            })
            .collect::<Vec<_>>();
        let ascii = tree(&tasks, Some("1"), 100, 18, glyph::GlyphSet::Ascii, &[]);
        assert!(ascii.fold().is_ascii(), "{}", ascii.fold());
        let view = tree(&tasks, Some("1"), 100, 18, Default::default(), &[]);
        view::answer_ok(
            render::Component::Pane,
            &json!({}),
            Drawing.id(),
            None,
            &view,
        )
        .unwrap();
        assert!(view.fold().contains("8 of 12"));
        assert!(view.fold().find("task 1").unwrap() < view.fold().find("task 12").unwrap());
        bingo_sdk::testing::assert_separate_hints(&view);
    }
}
