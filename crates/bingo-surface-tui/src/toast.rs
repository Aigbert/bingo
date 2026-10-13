//! Toast deadlines measured in visible time.
#[cfg(test)]
mod tests {
    use crate::test_support::*;
    use bingo_sdk::slot::Slot;
    #[test]
    fn toasts_disappear_at_their_deadline() {
        let (mut ui,now)=scene();
        let mut state=state();
        state.signals.entry("counter".into()).or_default().insert("toast".into(),bingo_sdk::slot::with(serde_json::json!({}),[Slot::Toast{text:"Saved counter".into(),until:now.wall+std::time::Duration::from_secs(4),alert:false}]));
        let tree=solo(&state);
        assert!(draw_tree(80,24,&tree,&mut ui,now).contains("Saved counter"));
        assert!(!draw_tree(80,24,&tree,&mut ui,later(now,4000)).contains("Saved counter"));
    }
}
