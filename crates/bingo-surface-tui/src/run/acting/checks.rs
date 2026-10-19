use super::*;
use bingo_sdk::{Delivery, testing::HostCall};

pub(super) fn check(scene: &Scene) {
    let wake = Delivery::Wake;
    let hold = Delivery::Hold;
    let expected = match scene.name.as_str() {
        "live-steered" | "live-stopped" => {
            vec![("测试名字用英文", wake), ("再更新 CHANGELOG", hold)]
        }
        "stop" => vec![(QUEUED, hold)],
        "stop-resent" => vec![(QUEUED, hold), (QUEUED, wake)],
        "steer" | "steer-landed" => vec![("测试名字用英文", wake), (QUEUED, hold)],
        "shell" => vec![("!git status --short", wake)],
        "shell-ls" => vec![("!git status --short", wake), ("!ls", wake)],
        "image" => vec![],
        "image-sent" => vec![("[image 1] 这个报错什么意思", wake)],
        _ => unreachable!(),
    };
    let inputs = scene.session.submitted();
    let actual: Vec<_> = inputs
        .iter()
        .map(|input| match input {
            Input::Text { text, delivery, .. } => (text.as_str(), *delivery),
            _ => panic!("text"),
        })
        .collect();
    assert_eq!(actual, expected, "{} sent", scene.name);
    let expected_withdrawals = match scene.name.as_str() {
        "stop" | "stop-resent" | "live-stopped" => 1,
        _ => 0,
    };
    let submissions = scene.session.submissions();
    let withdrawals: Vec<_> = scene
        .host
        .calls()
        .into_iter()
        .filter_map(|call| match call {
            HostCall::Withdraw {
                intent,
                session,
                who,
            } => Some((session, intent, who)),
            _ => None,
        })
        .collect();
    let queued_intents: Vec<_> = submissions
        .iter()
        .filter(|(_, input)| input.is_held())
        .map(|(intent, _)| {
            (
                scene.run.session.tree.root_id().clone(),
                intent.clone(),
                identity(),
            )
        })
        .take(expected_withdrawals)
        .collect();
    assert_eq!(withdrawals, queued_intents);
    let interrupts: Vec<_> = scene
        .session
        .interruptions()
        .into_iter()
        .map(|(_, scope)| scope)
        .collect();
    assert_eq!(interrupts, vec![InterruptScope::Head; expected_withdrawals]);
    if scene.name == "image-sent" {
        assert!(
            matches!(&inputs[0], Input::Text { images, .. } if images.len() == 1 && images[0].media_type == "image/png")
        );
    }
}
