use super::*;
pub fn result(ui: &mut Ui, tree: &Tree, action: &Action, now: Now) -> Vec<Effect> {
    if action.name != "surface:browse" || action.args["handler"].as_str() != request(ui) {
        return vec![];
    }
    let value = &action.args["value"];
    if value["close"] == true {
        ui.focus.close(now.instant);
    }
    if let Some(line) = value["run"].as_str() {
        ui.focus.close(now.instant);
        return crate::input::command_line(ui, tree, now, line.into());
    }
    for name in ["pager", "copy"] {
        if let Some(value) = value.get(name)
            && let Some(item) = selected_item(ui, tree, value)
        {
            if name == "copy" {
                return vec![Effect::Copy(transcript::text(item))];
            }
            ui.focus.show(
                Open::Pager(crate::pager::Pager::open(item.id.clone())),
                now.instant,
            );
        }
    }
    Vec::new()
}
fn selected_item<'a>(ui: &Ui, tree: &'a Tree, value: &serde_json::Value) -> Option<&'a Item> {
    let state = tree.viewed();
    if let Some(id) = value.as_str() {
        return state.transcript.get(&ItemId::from_raw(id));
    }
    let props = current(ui)?;
    let turn = props
        .turns
        .iter()
        .find(|t| Some(&t.id) == props.selected.as_ref())?;
    state.transcript.items().get(turn.first).map(AsRef::as_ref)
}
pub fn composer(ui: &mut Ui, text: &str, now: Now) {
    if ui.composer.is_empty() {
        ui.composer.set(text);
        ui.edited();
    } else {
        ui.notify(
            Level::Info,
            "Rewound prompt was not inserted; your draft was kept",
            now.instant,
        );
    }
}
