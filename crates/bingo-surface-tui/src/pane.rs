//! Pane placement and the editor belonging to the focused element.
#[cfg(test)]
mod tests {
    use crate::test_support::*;
    use bingo_sdk::{slot::{Slot,Shown},View};
    fn pane(asked:bool)->Slot {Slot::Pane{id:"counter".into(),title:"Counter pane".into(),focus:true,close_on_escape:true,hold_toasts:false,asked,rows:20,view:Shown::View(View::text("Count: 0"))}}
    fn with(slot:Slot)->bingo_sdk::SessionState {
        let mut state=state();
        state.signals.entry("counter".into()).or_default().insert("pane".into(),bingo_sdk::slot::with(serde_json::json!({}),[slot]));
        state
    }
    #[test]
    fn asked_panes_are_inline_at_both_widths_and_unasked_panes_wait_for_a_click() {
        for width in [80,200] {
            let (mut ui,now)=scene();
            let tree=solo(&with(pane(true)));
            let screen=draw_tree(width,24,&tree,&mut ui,now);
            assert!(screen.contains("Counter pane") && screen.contains("Count: 0"),"{screen}");
        }
        let (mut ui,now)=scene();
        let tree=solo(&with(pane(false)));
        let screen=draw_tree(200,24,&tree,&mut ui,now);
        assert!(screen.contains("Counter pane (waiting)"),"{screen}");
        assert!(!screen.contains("Count: 0"));
        crate::pointer::on_mouse(&mut ui,&tree,crossterm::event::MouseEvent{kind:crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),column:0,row:23,modifiers:crossterm::event::KeyModifiers::NONE},now);
        assert!(draw_tree(200,24,&tree,&mut ui,now).contains("Count: 0"));
    }
}
