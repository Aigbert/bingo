use crate::tests::{Journals, command_context};
use bingo_sdk::*;
use serde_json::json;
use std::sync::Arc;
async fn answer(verb: &str, wire: bool, id: u64) -> Result<String, String> {
    let journals = Journals::new();
    let session = journals.session();
    crate::create::add(
        &journals.handle(),
        &session,
        crate::Draft {
            subject: "first".into(),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let (method, args, params) = match verb {
        "empty-add" => ("add", "add".into(), json!({"subject":""})),
        "empty-assign" => (
            "reassign",
            format!("assign {id}"),
            json!({"id":id,"owner":""}),
        ),
        "add" => (
            "add",
            "add second @reviewer".into(),
            json!({"subject":"second","owner":"reviewer"}),
        ),
        "assign" => (
            "reassign",
            format!("assign {id} scout"),
            json!({"id":id,"owner":"scout"}),
        ),
        verb => (
            "set_status",
            format!("{verb} {id}"),
            json!({"id":id,"status":if verb=="done" {"completed"} else {"pending"}}),
        ),
    };
    if wire {
        let service = Arc::new(crate::service::Writer::default());
        let host = journals.handle();
        service.bind(&host);
        let mut params = params;
        params["session"] = json!(session);
        return bingo_services::tasks_write::wire(service)
            .call(method, params)
            .await
            .map(|v| v.as_str().unwrap().to_string())
            .map_err(|e| e.0);
    }
    match crate::TasksCommand
        .run(&args, &command_context(&session, &journals))
        .await
    {
        Ok(CommandOutcome::Applied { message }) => Ok(message.unwrap_or_default()),
        Ok(other) => panic!("unexpected {other:?}"),
        Err(error) => Err(error.message),
    }
}
#[tokio::test]
async fn commands_and_services_share_receipts_and_refusals() {
    for verb in ["add", "done", "open", "assign", "empty-add", "empty-assign"] {
        for id in [1, 99] {
            assert_eq!(
                answer(verb, false, id).await,
                answer(verb, true, id).await,
                "{verb}"
            );
        }
    }
}
