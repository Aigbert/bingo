use super::*;
use crate::test_support::*;
use serde_json::json;

#[tokio::test]
async fn unpin_keeps_the_model_row_and_reports_the_change() {
    let (host, cx) = host();
    let (models, providers) = entries();
    let mut picker = Picker {
        props: component::ModelPickerProps {
            selected: Some("pin:p/second".into()),
            ..Default::default()
        },
        ..Default::default()
    };
    act(
        &mut picker,
        "*",
        &Row::Pinned("p/second".into()),
        &models,
        &providers,
        &cx,
    )
    .await
    .unwrap();
    assert_eq!(picker.props.selected.as_deref(), Some("model:p/second"));
    assert!(host.calls().iter().any(|call| matches!(call,
        testing::HostCall::Signal { payload, .. } if payload["slots"][0]["text"] == "unpinned second"
    )));
    let mut updated = models;
    updated[1].meta["pinned"] = json!(false);
    let rows = crate::rows::rows(&updated, &providers, "", false);
    let at = crate::rows::cursor(&rows, picker.props.selected.as_deref()).unwrap();
    assert_eq!(rows[at], Row::Model("p/second".into()));
}

#[test]
fn a_pinned_models_hint_offers_unpin() {
    let (models, providers) = entries();
    let picker = Picker {
        props: component::ModelPickerProps {
            selected: Some("pin:p/second".into()),
            ..Default::default()
        },
        ..Default::default()
    };
    let view = crate::draw::tree(
        &picker,
        &models,
        &providers,
        &crate::card::Chat::default(),
        108,
        18,
    );
    assert!(view.fold().contains("* unpin"), "{}", view.fold());
}
