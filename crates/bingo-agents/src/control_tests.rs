use crate::tests::{Fleet, command_context};
use bingo_sdk::*;
use serde_json::json;
use std::sync::Arc;

async fn answer(verb: &str, via_service: bool, peer: bool) -> Result<String, String> {
    let fleet = Fleet::default();
    let root = fleet.root();
    fleet.child(&root, "reviewer");
    let caller = if peer {
        fleet.child(&root, "scout")
    } else {
        root
    };
    let args = if matches!(verb, "message" | "reopen") {
        "reviewer hello"
    } else {
        "reviewer"
    };
    if via_service {
        let service = Arc::new(crate::service::AgentService::new(
            Env::rooted("/tmp"),
            Arc::default(),
            Arc::default(),
        ));
        let host = fleet.handle();
        service.bind(&host);
        let wire = bingo_services::agents_control::wire(service);
        return wire
            .call(
                verb,
                json!({"session":caller,"name":"reviewer","text":"hello"}),
            )
            .await
            .map(|v| v.as_str().unwrap().to_string())
            .map_err(|e| e.0);
    }
    match crate::AgentsCommand
        .run(&format!("{verb} {args}"), &command_context(&caller, &fleet))
        .await
    {
        Ok(CommandOutcome::Applied { message }) => Ok(message.unwrap_or_default()),
        Ok(other) => panic!("unexpected {other:?}"),
        Err(error) => Err(error.message),
    }
}
#[tokio::test]
async fn commands_and_services_share_receipts_and_refusals() {
    for verb in ["stop", "dismiss", "message", "reopen"] {
        for peer in [false, true] {
            let direct = answer(verb, false, peer).await;
            let service = answer(verb, true, peer).await;
            // Dismiss receipts include the fresh session identifier.
            let normalize = |v: Result<String, String>| {
                v.map(|s| {
                    s.split_whitespace()
                        .map(|word| {
                            if word.starts_with("ses_") {
                                "<session>"
                            } else {
                                word
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                })
            };
            assert_eq!(normalize(direct), normalize(service), "{verb} peer={peer}");
        }
    }
}
