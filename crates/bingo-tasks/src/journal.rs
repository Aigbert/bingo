//! Where the list lives: the session's own journal, as the extension
//! `bingo.tasks`/`tasks` (ADR-0011 §2). Every reader opens the session and
//! folds the frames the kernel already folded; every writer publishes the
//! whole list again. Nothing is kept between calls — a file or a map here
//! would be a second copy of a fact the journal already holds, and the one
//! that survives `--continue` is the journal's.

use bingo_sdk::{
    ClientIdentity, ErrorCode, HostHandle, KernelError, OpenOptions, SessionId, SessionSelector,
    SessionState,
};

use crate::task::Task;

/// The plugin the payload belongs to, and the kind within it.
pub const PLUGIN: &str = "bingo.tasks";
pub const KIND: &str = "tasks";

/// The list as the session's journal has it. A session that never wrote one,
/// or wrote something this crate cannot read, has no tasks.
pub async fn read(host: &HostHandle, session: &SessionId) -> Result<Vec<Task>, KernelError> {
    let attachment = host
        .open(
            SessionSelector::ById {
                id: session.clone(),
            },
            ClientIdentity {
                name: "tasks".into(),
                surface: "tasks".into(),
            },
            OpenOptions::default(),
        )
        .await?;
    Ok(tasks_of(&attachment.snapshot))
}

fn tasks_of(snapshot: &SessionState) -> Vec<Task> {
    let Some(payload) = snapshot
        .extensions
        .get(PLUGIN)
        .and_then(|kinds| kinds.get(KIND))
    else {
        return Vec::new();
    };
    match serde_json::from_value(payload.get("tasks").unwrap_or(payload).clone()) {
        Ok(tasks) => tasks,
        Err(error) => {
            tracing::warn!(%error, "the tasks in this journal are not a task list; starting empty");
            Vec::new()
        }
    }
}

/// Publishes the whole list, which is what the kind means: the next snapshot
/// carries exactly this, and so does the next run that continues the session.
pub async fn write(
    host: &HostHandle,
    session: &SessionId,
    tasks: &[Task],
) -> Result<(), KernelError> {
    let payload = serde_json::to_value(tasks)
        .map_err(|e| KernelError::new(ErrorCode::Internal, format!("a task list is json: {e}")))?;
    let slots = activity(tasks).into_iter();
    let payload = bingo_sdk::slot::with(serde_json::json!({"tasks": payload}), slots);
    host.extend(session, PLUGIN, KIND, payload).await
}

fn activity(tasks: &[Task]) -> Option<bingo_sdk::slot::Slot> {
    use crate::task::Status;
    use bingo_sdk::View;
    if tasks.is_empty() {
        return None;
    }
    let count = |status| tasks.iter().filter(|task| task.status == status).count();
    let title = format!(
        "{} {} ({} done, {} in progress, {} open)",
        tasks.len(),
        if tasks.len() == 1 { "task" } else { "tasks" },
        count(Status::Completed),
        count(Status::InProgress),
        count(Status::Pending)
    );
    let shown: Vec<_> = tasks
        .iter()
        .filter(|task| tasks.len() <= 5 || task.status != Status::Completed)
        .take(if tasks.len() <= 5 { tasks.len() } else { 5 })
        .collect();
    let tree = View::Tree {
        nodes: shown.iter().map(|task| node(task)).collect(),
    };
    let child = match overflow(tasks, shown.len()) {
        Some(text) => View::Stack {
            children: vec![tree, View::text(text)],
        },
        None => tree,
    };
    Some(bingo_sdk::slot::Slot::Activity {
        verb: tasks
            .iter()
            .find(|task| task.status == Status::InProgress)
            .map(|task| task.active_form.as_ref().unwrap_or(&task.subject).clone()),
        view: View::Panel {
            title,
            child: Box::new(child),
        },
    })
}

fn node(task: &Task) -> bingo_sdk::TreeNode {
    use crate::task::Status;
    use bingo_sdk::Tone;
    bingo_sdk::TreeNode {
        label: task.subject.clone(),
        badge: task.owner.clone(),
        children: vec![],
        tone: match task.status {
            Status::Pending => Tone::Neutral,
            Status::InProgress => Tone::Attention,
            Status::Completed => Tone::Good,
        },
    }
}

fn overflow(tasks: &[Task], shown: usize) -> Option<String> {
    let hidden = tasks.len() - shown;
    if hidden == 0 {
        return None;
    }
    let done = tasks
        .iter()
        .filter(|task| task.status == crate::task::Status::Completed)
        .count();
    let mut parts = Vec::new();
    if hidden > done {
        parts.push(format!("+{} pending", hidden - done));
    }
    if done > 0 {
        parts.push(format!("{done} completed"));
    }
    Some(format!("… {}", parts.join(", ")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{self, Draft};
    use crate::tests::Journals;
    use serde_json::json;

    #[tokio::test]
    async fn slots_upgrade_an_old_array_with_its_activity() {
        let journals = Journals::new();
        let session = journals.session();
        let host = journals.handle();
        let old: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/pre-m121.json")).unwrap();
        host.extend(&session, PLUGIN, KIND, old.clone())
            .await
            .unwrap();
        let tasks = read(&host, &session).await.unwrap();
        assert_eq!(tasks.len(), 1);
        write(&host, &session, &tasks).await.unwrap();
        let state = host
            .open(
                SessionSelector::ById { id: session },
                ClientIdentity {
                    name: "test".into(),
                    surface: "test".into(),
                },
                OpenOptions::default(),
            )
            .await
            .unwrap()
            .snapshot;
        assert_eq!(state.extensions[PLUGIN][KIND]["tasks"], old);
        assert_eq!(
            bingo_sdk::slot::placed(&state)[0].slot,
            bingo_sdk::slot::Slot::Activity {
                verb: Some("Shipping".into()),
                view: bingo_sdk::View::Panel {
                    title: "1 task (0 done, 1 in progress, 0 open)".into(),
                    child: Box::new(bingo_sdk::View::Tree {
                        nodes: vec![bingo_sdk::TreeNode {
                            label: "Ship it".into(),
                            badge: Some("Scout".into()),
                            tone: bingo_sdk::Tone::Attention,
                            children: vec![],
                        }]
                    }),
                },
            }
        );
        assert_eq!(read(&host, &state.summary.id).await.unwrap(), tasks);
    }

    #[tokio::test]
    async fn a_session_that_wrote_nothing_has_no_tasks() {
        let journals = Journals::new();
        let session = journals.session();
        assert!(
            read(&journals.handle(), &session)
                .await
                .expect("an empty list")
                .is_empty()
        );
    }

    #[tokio::test]
    async fn what_was_written_is_what_the_next_read_folds() {
        let journals = Journals::new();
        let session = journals.session();
        let host = journals.handle();
        let mut tasks = Vec::new();
        task::create(
            &mut tasks,
            Draft {
                subject: "write the plan".into(),
                ..Draft::default()
            },
        );
        write(&host, &session, &tasks).await.expect("published");
        assert_eq!(read(&host, &session).await.expect("read back"), tasks);
    }

    #[tokio::test]
    async fn a_payload_that_is_not_a_task_list_reads_as_none() {
        let journals = Journals::new();
        let session = journals.session();
        let host = journals.handle();
        host.extend(&session, PLUGIN, KIND, json!({"tasks": "later"}))
            .await
            .expect("the kernel takes any payload");
        assert!(read(&host, &session).await.expect("no tasks").is_empty());
    }

    /// The four tools keep nothing between them: what one wrote, the next
    /// reads back out of the journal, and a fresh attachment sees the same.
    #[tokio::test]
    async fn the_four_tools_share_one_list_and_nothing_else() {
        use crate::create::TaskCreateTool;
        use crate::get::TaskGetTool;
        use crate::list::TaskListTool;
        use crate::tests::{Journals, text, tool_context};
        use crate::update::TaskUpdateTool;
        use bingo_sdk::Tool;

        let journals = Journals::new();
        let session = journals.session();
        let cx = tool_context(&session, &journals);

        let created = TaskCreateTool
            .call(json!({"subject": "write the plan"}), &cx)
            .await
            .expect("a task");
        assert_eq!(text(&created), "Created #1: write the plan");

        let listed = TaskListTool.call(json!({}), &cx).await.expect("a listing");
        assert_eq!(text(&listed), "#1 [pending] write the plan");

        let updated = TaskUpdateTool
            .call(json!({"id": 1, "status": "in_progress"}), &cx)
            .await
            .expect("an update");
        assert_eq!(text(&updated), "Updated #1 (in_progress): write the plan");

        let got = TaskGetTool
            .call(json!({"id": 1}), &cx)
            .await
            .expect("the task");
        let value: serde_json::Value = serde_json::from_str(&text(&got)).expect("pretty json");
        assert_eq!(value["status"], json!("in_progress"));

        assert_eq!(
            read(&journals.handle(), &session).await.expect("read back"),
            vec![Task {
                id: 1,
                subject: "write the plan".into(),
                description: String::new(),
                active_form: None,
                status: crate::task::Status::InProgress,
                owner: None,
                blocks: Vec::new(),
                blocked_by: Vec::new(),
                metadata: Default::default(),
            }],
            "a second attachment sees the same list"
        );
    }

    /// Another plugin's state in the same journal is not this one's.
    #[tokio::test]
    async fn only_this_plugin_s_kind_is_the_list() {
        let journals = Journals::new();
        let session = journals.session();
        let host = journals.handle();
        host.extend(&session, "bingo.rooms", "members", json!([]))
            .await
            .expect("published");
        assert!(read(&host, &session).await.expect("no tasks").is_empty());
    }
}
