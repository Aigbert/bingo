//! Picture refusal at the run boundary.
use std::sync::Arc;

#[test]
fn image_refusal_is_checked_once_at_the_run_boundary() {
    let mut state = crate::test_support::state();
    state.summary.model = Some("text-only".into());
    state.summary.provider = Some("fake".into());
    let mut run = super::tests::idle_in(
        state,
        Arc::new(crate::doubles::TestSession::default()),
        std::time::Instant::now(),
    );
    run.ui.catalogs.values.insert(
        "models".into(),
        vec![bingo_sdk::CatalogEntry {
            id: "fake/text-only".into(),
            label: "text-only".into(),
            meta: serde_json::json!({"images":false}),
        }],
    );
    run.clipboard_image = Arc::new(|| panic!("a refused model never reads the clipboard"));
    run.terminal_event(crossterm::event::Event::Key(crate::test_support::ctrl('v')));
    assert!(run.ui.composer.is_empty());
    assert!(
        run.ui
            .notice()
            .unwrap()
            .plain()
            .unwrap()
            .text
            .contains("alt+m switches model")
    );
}
