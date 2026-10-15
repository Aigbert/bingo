//! `/tasks`: the session's list as a person reads it, in the columns of the
//! table every surface draws — or, with `in #room`, the shared board that
//! room holds.

use async_trait::async_trait;
use bingo_sdk::{ArgSpec, Command, CommandContext, CommandOutcome, CommandSpec, KernelError, View};

use crate::board::{self, In};
use crate::render::Present;
use crate::{journal, render};

/// What a person is told when there is nothing on the list. The model's copy
/// names the tool that adds one; a person has this command and no tool.
const NONE: &str = "no tasks in this session";

#[derive(Debug, Default, Clone, Copy)]
pub struct TasksCommand;

#[async_trait]
impl Command for TasksCommand {
    fn spec(&self) -> CommandSpec {
        CommandSpec {
            opens: false,
            keys: vec![],
            hidden: false,
            name: "tasks".into(),
            aliases: Vec::new(),
            hint: "the tasks of this session, or of a room's board".into(),
            args: ArgSpec::Free {
                hint: "[in #room]".into(),
            },
            // Reading the journal touches nothing a turn is using.
            instant: true,
            family: "tasks".into(),
        }
    }

    async fn run(&self, args: &str, cx: &CommandContext) -> Result<CommandOutcome, KernelError> {
        if let Some(message) = write(args, cx).await? {
            return Ok(CommandOutcome::Applied {
                message: Some(message),
            });
        }
        let asked = In::spoken(args)?;
        let board = board::of(&cx.host, &cx.session, &asked).await?;
        let tasks = journal::read(&cx.host, &board.session).await?;
        if tasks.is_empty() {
            return Ok(CommandOutcome::View {
                view: View::text(empty(&asked)),
            });
        }
        let here = board.present();
        Ok(CommandOutcome::View {
            view: View::Table {
                headers: render::HEADERS.map(str::to_string).to_vec(),
                rows: render::rows(&tasks, Present::among(here.as_deref())),
            },
        })
    }
}

async fn write(args: &str, cx: &CommandContext) -> Result<Option<String>, KernelError> {
    let (verb, rest) = args
        .trim()
        .split_once(char::is_whitespace)
        .unwrap_or((args.trim(), ""));
    let message = match verb {
        "add" => {
            let (subject, owner) = rest
                .rsplit_once(" @")
                .map_or((rest, None), |(subject, owner)| {
                    (subject, Some(owner.into()))
                });
            crate::create::add(
                &cx.host,
                &cx.session,
                crate::Draft {
                    subject: subject.into(),
                    owner,
                    ..Default::default()
                },
            )
            .await?
        }
        "done" | "open" | "assign" => {
            let (id, owner) = rest
                .trim()
                .split_once(char::is_whitespace)
                .unwrap_or((rest.trim(), ""));
            let id = id.parse().map_err(|_| invalid("name a numeric task id"))?;
            if verb == "assign" {
                crate::update::assign(&cx.host, &cx.session, id, owner).await?
            } else {
                let change = crate::Change {
                    id,
                    status: Some(if verb == "done" {
                        crate::Status::Completed
                    } else {
                        crate::Status::Pending
                    }),
                    ..Default::default()
                };
                crate::update::change(&cx.host, &cx.session, change, false).await?
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(message))
}
fn invalid(text: &str) -> KernelError {
    KernelError::new(bingo_sdk::ErrorCode::InvalidInput, text)
}

/// Which empty list it was: a person who asked about a room is told about that
/// room, not about the session they are sitting in.
fn empty(board: &In) -> String {
    match board.name() {
        None => NONE.to_string(),
        Some(name) => format!("no tasks on {name}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::create::TaskCreateTool;
    use crate::tests::{Journals, command_context, tool_context};
    use crate::update::TaskUpdateTool;
    use bingo_sdk::Tool;
    use serde_json::json;

    #[tokio::test]
    async fn the_table_has_a_row_per_task_in_the_columns_of_the_headers() {
        let journals = Journals::new();
        let session = journals.session();
        let cx = tool_context(&session, &journals);
        TaskCreateTool
            .call(
                json!({"subject": "write the plan", "owner": "reviewer"}),
                &cx,
            )
            .await
            .expect("a task");
        TaskCreateTool
            .call(json!({"subject": "ship it"}), &cx)
            .await
            .expect("a task");
        TaskUpdateTool
            .call(json!({"id": 1, "status": "in_progress"}), &cx)
            .await
            .expect("an update");

        let outcome = TasksCommand
            .run("", &command_context(&session, &journals))
            .await
            .expect("a table");
        let CommandOutcome::View {
            view: View::Table { headers, rows },
        } = outcome
        else {
            panic!("a task list is a table");
        };
        assert_eq!(headers, render::HEADERS);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], ["1", "in_progress", "write the plan", "reviewer"]);
        assert_eq!(rows[1], ["2", "pending", "ship it", ""]);
    }

    #[tokio::test]
    async fn a_session_with_no_tasks_gets_one_line() {
        let journals = Journals::new();
        let session = journals.session();
        let outcome = TasksCommand
            .run("", &command_context(&session, &journals))
            .await
            .expect("a line");
        assert_eq!(
            outcome,
            CommandOutcome::View {
                view: View::text(NONE)
            }
        );
    }

    /// The same table, for a room's board, with the one thing a board says
    /// that a session's own list does not.
    #[tokio::test]
    async fn a_room_s_board_is_the_same_table_and_names_who_is_gone() {
        let journals = Journals::new();
        let root = journals.session();
        journals.room(&root, "#design");
        journals.child(&root, "reviewer");
        let cx = tool_context(&root, &journals);
        for owner in ["reviewer", "scout"] {
            TaskCreateTool
                .call(
                    json!({"subject": "write the plan", "owner": owner, "in": "#design"}),
                    &cx,
                )
                .await
                .expect("a task on the board");
        }

        let outcome = TasksCommand
            .run("in #design", &command_context(&root, &journals))
            .await
            .expect("a table");
        let CommandOutcome::View {
            view: View::Table { rows, .. },
        } = outcome
        else {
            panic!("a board is a table");
        };
        assert_eq!(rows[0][3], "reviewer");
        assert_eq!(rows[1][3], "scout (gone)");
    }

    #[tokio::test]
    async fn an_empty_board_names_the_room_and_an_unreachable_one_is_refused() {
        let journals = Journals::new();
        let root = journals.session();
        journals.room(&root, "#design");
        let cx = command_context(&root, &journals);

        assert_eq!(
            TasksCommand.run("in #design", &cx).await.expect("a line"),
            CommandOutcome::View {
                view: View::text("no tasks on #design")
            }
        );
        let error = TasksCommand
            .run("in #nowhere", &cx)
            .await
            .expect_err("no such room");
        assert!(error.message.contains("#design"), "{error}");
        let misspoken = TasksCommand
            .run("#design", &cx)
            .await
            .expect_err("a board is two words");
        assert!(misspoken.message.contains("in #room"), "{misspoken}");
    }

    #[test]
    fn the_spec_runs_now_and_takes_a_board_or_nothing() {
        let spec = TasksCommand.spec();
        assert_eq!(spec.name, "tasks");
        assert!(spec.instant, "reading the journal never waits for a turn");
        assert_eq!(
            spec.args,
            ArgSpec::Free {
                hint: "[in #room]".into()
            }
        );
        assert_eq!(spec.family, "tasks");
    }
}
