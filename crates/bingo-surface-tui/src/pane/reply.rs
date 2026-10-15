//! Client operations returned by the owner of a pressed pane.
use crate::{clock::Now, effect::Effect, tree::Tree, ui::Ui};
use bingo_sdk::{Action, SessionId};
pub fn result(
    ui: &mut Ui,
    tree: &Tree,
    action: &Action,
    plugin: &str,
    id: &str,
    vouched: bool,
    now: Now,
) -> Vec<Effect> {
    if action.name != "surface:pane" || action.args["plugin"] != plugin || action.args["id"] != id {
        return vec![];
    }
    if !vouched {
        return vec![];
    }
    let value = &action.args["value"];
    if let Some(message) = value["message"].as_str() {
        ui.notify(bingo_sdk::Level::Info, message, now.instant);
    }
    if let Some(line) = value["run"].as_str() {
        return crate::input::typed_line(ui, tree, now, line.into());
    }
    if let Some(id) = value["view"].as_str() {
        let id = SessionId::from_raw(id);
        return vec![if tree.state(&id).is_some() {
            Effect::View(id)
        } else {
            Effect::Open(bingo_sdk::SessionSelector::ById { id })
        }];
    }
    vec![]
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    #[test]
    fn only_the_pressed_owner_can_run_a_persons_line() {
        let now = still(scene().1);
        let mut ui = scene().0;
        let tree = solo(&state());
        let action = Action {
            name: "surface:pane".into(),
            args: serde_json::json!({"plugin":"built.in","id":"panel","value":{"run":"/guard enable"}}),
        };
        assert!(result(&mut ui, &tree, &action, "other", "panel", true, now).is_empty());
        assert!(result(&mut ui, &tree, &action, "built.in", "panel", false, now).is_empty());
        assert!(matches!(
            result(&mut ui, &tree, &action, "built.in", "panel", true, now).as_slice(),
            [Effect::Submit(_)]
        ));
    }
    #[test]
    fn a_session_outside_the_current_tree_is_opened() {
        let (mut ui, now) = scene();
        let tree = solo(&state());
        let action = Action {
            name: "surface:pane".into(),
            args: serde_json::json!({"plugin":"built.in","id":"panel","value":{"view":"stored"}}),
        };
        assert!(
            matches!(result(&mut ui,&tree,&action,"built.in","panel",true,now).as_slice(),[Effect::Open(bingo_sdk::SessionSelector::ById {id})] if id.as_str()=="stored")
        );
    }
    #[test]
    fn unvouched_views_and_messages_have_no_effect() {
        let (mut ui, now) = scene();
        let tree = solo(&state());
        for value in [
            serde_json::json!({"view":"stored"}),
            serde_json::json!({"message":"forged"}),
        ] {
            let action = Action {
                name: "surface:pane".into(),
                args: serde_json::json!({"plugin":"built.in","id":"panel","value":value}),
            };
            assert!(result(&mut ui, &tree, &action, "built.in", "panel", false, now).is_empty());
            assert!(result(&mut ui, &tree, &action, "other", "panel", true, now).is_empty());
        }
        assert!(ui.notices.is_empty());
    }
}
