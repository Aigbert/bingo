use super::*;
use bingo_sdk::{Attachment, Delivery, Event, HostApi, SessionSelector, SessionSpec};
use futures::StreamExt;
use std::sync::Arc;

fn input(text: &str, images: Vec<bingo_sdk::Image>) -> Input {
    Input::Text {
        text: text.into(),
        images,
        origin: crate::origin(),
        delivery: Delivery::Hold,
    }
}
fn image(path: &str) -> bingo_sdk::Image {
    bingo_sdk::Image::from_bytes("image/png", b"fixture")
        .unwrap()
        .at(path)
}

#[test]
fn stopping_restores_order_and_images_without_losing_a_late_draft() {
    let mut run = crate::run::tests::idle(Instant::now());
    run.ui.pictures.hold("", "/late.png".into());
    run.ui.composer.set("late [image 1]");
    stopped(
        &mut run,
        vec![
            (
                IntentId::from_raw("first"),
                Ok(input(
                    "first [image 1] @mentioned.png",
                    vec![image("/first.png"), image("/mentioned.png")],
                )),
            ),
            (
                IntentId::from_raw("second"),
                Ok(input("second [image 1]", vec![image("/second.png")])),
            ),
            (
                IntentId::from_raw("delivered"),
                Err(KernelError::new(ErrorCode::NotFound, "already delivered")),
            ),
        ],
    );
    assert_eq!(
        run.ui.composer.text(),
        "first [image 1] @mentioned.png\nsecond [image 2]\nlate [image 3]"
    );
    assert_eq!(
        run.ui.pictures.carried(run.ui.composer.text()),
        vec![
            std::path::PathBuf::from("/first.png"),
            "/second.png".into(),
            "/late.png".into()
        ]
    );
    assert_eq!(
        run.ui.notice().unwrap().plain().unwrap().text,
        "stopped · what you queued is back in the input"
    );
}

async fn until(attached: &mut Attachment, matches: impl Fn(&Event) -> bool) {
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(frame) = attached.events.next().await {
            attached.snapshot.apply(&frame);
            if matches(&frame.event) {
                return;
            }
        }
        panic!("event stream ended");
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn a_fake_provider_delivery_wins_withdrawal_without_duplication() {
    let home = tempfile::tempdir().unwrap();
    let script =
        serde_json::from_value(serde_json::json!({"responses":[{"steps":[{"delay":{"ms":60000}}]}, {"steps":[{"text":"done"}]}]}))
            .unwrap();
    let host = bingo_core::Host::build(
        vec![Box::new(bingo_provider_fake::FakePlugin::new(Arc::new(
            bingo_provider_fake::FakeProvider::new(script),
        )))],
        bingo_core::HostConfig::new(bingo_sdk::Env::rooted(home.path())),
    )
    .await
    .unwrap();
    let mut attached = host
        .open(
            SessionSelector::Create {
                spec: SessionSpec {
                    cwd: home.path().into(),
                    ..Default::default()
                },
            },
            identity(),
            Default::default(),
        )
        .await
        .unwrap();
    attached
        .handle
        .submit(IntentId::mint(), Input::text("first turn", crate::origin()));
    until(&mut attached, |event| {
        matches!(event, Event::TurnStarted { .. })
    })
    .await;
    let intent = IntentId::mint();
    attached
        .handle
        .submit(intent.clone(), input("delivered once", vec![]));
    until(
        &mut attached,
        |event| matches!(event, Event::QueueChanged { entries, .. } if !entries.is_empty()),
    )
    .await;
    attached
        .handle
        .interrupt(IntentId::mint(), bingo_sdk::InterruptScope::Head);
    until(&mut attached, |event| matches!(event, Event::ItemCompleted { item } if matches!(&item.body, bingo_sdk::ItemBody::User { parts, .. } if parts.iter().any(|part| part.as_text() == Some("delivered once"))))).await;
    let taken = host.withdraw(&attached.session, &intent, identity()).await;
    assert_eq!(taken.as_ref().unwrap_err().code, ErrorCode::NotFound);
    let mut run = crate::run::tests::idle(Instant::now());
    took(&mut run, intent, taken);
    assert!(run.ui.composer.is_empty());
    assert_eq!(run.ui.notice().unwrap().plain().unwrap().text, ALREADY_SENT);
    assert_eq!(
        attached
            .snapshot
            .transcript
            .items()
            .iter()
            .filter(|item| matches!(&item.body, bingo_sdk::ItemBody::User { .. }))
            .count(),
        2
    );
    host.shutdown().await;
}
