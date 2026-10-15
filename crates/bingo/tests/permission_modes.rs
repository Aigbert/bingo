#![allow(clippy::unwrap_used, clippy::expect_used)]
use bingo_sdk::*;
use futures::StreamExt;
use serde_json::json;

const MODES: [&str; 5] = [
    "bypassPermissions",
    "acceptEdits",
    "default",
    "plan",
    "dontAsk",
];
const TIGHTENS: [[bool; 5]; 5] = [
    [true, true, true, true, true],
    [false, true, true, true, true],
    [false, false, true, true, true],
    [false, false, false, true, false],
    [false, false, false, false, true],
];
async fn change(attachment: &mut Attachment, origin: Origin, mode: &str) -> IntentOutcome {
    let intent = IntentId::mint();
    attachment.handle.submit(
        intent.clone(),
        Input::action(
            Action {
                name: "permission".into(),
                args: json!(mode),
            },
            origin,
        ),
    );
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(frame) = attachment.events.next().await {
            if let Event::IntentAck {
                intent: ack,
                outcome,
            } = frame.event
                && ack == intent
            {
                return outcome;
            }
        }
        panic!("event stream ended");
    })
    .await
    .unwrap()
}
async fn transition(
    attachment: &mut Attachment,
    origin: &Origin,
    before: &str,
    after: &str,
    allowed: bool,
) {
    assert!(matches!(
        change(attachment, Origin::surface("rpc"), before).await,
        IntentOutcome::Applied { .. }
    ));
    let outcome = change(attachment, origin.clone(), after).await;
    assert_eq!(
        matches!(outcome, IntentOutcome::Applied { .. }),
        allowed,
        "{origin:?}: {before} -> {after}: {outcome:?}"
    );
    if !allowed {
        assert!(matches!(
            outcome,
            IntentOutcome::Rejected {
                error: KernelError {
                    code: ErrorCode::PermissionDenied,
                    ..
                }
            }
        ));
    }
    let IntentOutcome::Applied { result } = change(attachment, origin.clone(), "").await else {
        panic!("mode listing");
    };
    let view: View = serde_json::from_value(result["view"].clone()).unwrap();
    let current = if allowed { after } else { before };
    assert!(
        view.fold()
            .starts_with(&format!("permission mode: {current}\n")),
        "{}",
        view.fold()
    );
}
async fn matrix(origin: Origin) {
    let home = tempfile::tempdir().unwrap();
    let host = bingo_core::Host::build(
        vec![Box::new(bingo_permissions::PermissionsPlugin)],
        bingo_core::HostConfig::new(Env::rooted(home.path())),
    )
    .await
    .unwrap();
    let mut attachment = host
        .open(
            SessionSelector::Create {
                spec: SessionSpec {
                    cwd: home.path().into(),
                    driver: Driver::Log,
                    ..Default::default()
                },
            },
            ClientIdentity {
                name: "test".into(),
                surface: "rpc".into(),
            },
            OpenOptions::default(),
        )
        .await
        .unwrap();
    for (before, row) in MODES.into_iter().zip(TIGHTENS) {
        for (after, allowed) in MODES.into_iter().zip(row) {
            transition(&mut attachment, &origin, before, after, allowed).await;
        }
    }
    host.shutdown().await;
}
#[tokio::test]
async fn plugin_actions_only_tighten_permission_modes() {
    matrix(Origin::surface("plugin:probe")).await;
}
#[tokio::test]
async fn mod_rows_only_tighten_permission_modes() {
    matrix(Origin::surface("plugin:bingo.mod.status")).await;
}
#[tokio::test]
async fn stored_modes_only_tighten_permission_modes() {
    matrix(Origin::surface("schedule").voiced(Voice::Quiet)).await;
}
#[tokio::test]
async fn chat_members_only_tighten_permission_modes() {
    let mut origin = Origin::surface("channels");
    origin.principal = Some("member".into());
    origin.conversation = Some("group".into());
    matrix(origin).await;
}
