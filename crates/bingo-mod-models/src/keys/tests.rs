use super::*;
use crate::test_support::*;
#[tokio::test]
async fn letters_leave_the_model_and_scope_alone() {
    let (_, cx) = host();
    let (models, providers) = entries();
    let mut picker = Picker {
        props: component::ModelPickerProps {
            selected: Some("model:p/second".into()),
            ..Default::default()
        },
        ..Default::default()
    };
    for key in ["a", "*", "tab"] {
        act(
            &mut picker,
            key,
            &Row::Model("p/second".into()),
            &models,
            &providers,
            &cx,
        )
        .await
        .unwrap();
    }
    assert_eq!(picker.props.selected.as_deref(), Some("model:p/second"));
    assert!(picker.props.filter.is_empty());
}
#[test]
fn model_hints_are_only_the_prototypes_keys() {
    let (models, providers) = entries();
    let view = crate::draw::tree(
        &Picker::default(),
        &models,
        &providers,
        &Default::default(),
        100,
        18,
        Default::default(),
    );
    bingo_sdk::testing::assert_separate_hints(&view);
    let View::Box { layout, .. } = view else {
        panic!("sheet")
    };
    let hints = layout.hints.unwrap();
    assert_eq!(
        hints
            .pairs
            .iter()
            .map(|p| p.keys.as_str())
            .collect::<Vec<_>>(),
        ["↑↓", "←→", "⏎", "esc"]
    );
}

#[tokio::test]
async fn plus_does_not_open_a_provider_sub_sheet() {
    let (_, cx) = host();
    let mut picker = Picker::default();
    press(&mut picker, &bingo_sdk::sheet::Press::Key("+".into()), &cx)
        .await
        .unwrap();
    assert!(picker.sheet.is_none());
}

#[test]
fn provider_sub_sheets_use_stable_action_ids_and_the_builder() {
    let provider = CatalogEntry {
        id: "p".into(),
        label: "p".into(),
        meta: serde_json::json!({}),
    };
    let mut sheet = Sheet::Manage {
        provider: "p".into(),
        selected: Some("refresh".into()),
        pending: None,
        confirm: false,
    };
    let before = sheet.build(std::slice::from_ref(&provider));
    assert_eq!(
        before.rows[pick::selected(&before.rows, before.selected.as_deref())].id,
        "refresh"
    );
    if let Sheet::Manage { pending, .. } = &mut sheet {
        *pending = Some("p/model".into());
    }
    let after = sheet.build(&[provider]);
    assert_eq!(
        after.rows[pick::selected(&after.rows, after.selected.as_deref())].id,
        "refresh"
    );
    assert!(
        serde_json::to_string(&after.view(100, 18, Default::default()))
            .unwrap()
            .contains("\"ground\":\"pick\"")
    );
    let signin = Sheet::SignIn {
        provider: "p".into(),
        selected: Some("paste".into()),
        pending: None,
    }
    .build(&[]);
    assert_eq!(signin.rows[2].id, "paste");
    assert!(
        !signin
            .view(100, 18, Default::default())
            .fold()
            .contains("model ›")
    );
}
