//! Mode words and inks read from the policy's placement.
use crate::{clock::Now, ui::Ui};
use bingo_sdk::{SessionState, slot::Slot};

pub struct Mode {
    pub value: String,
    pub label: String,
}
pub fn current(state: &SessionState) -> Mode {
    bingo_sdk::slot::placed(state)
        .into_iter()
        .find_map(|placed| {
            let Slot::Mode { current, modes } = placed.slot else {
                return None;
            };
            let label = match current.as_str() {
                "bypassPermissions" => "bypass".into(),
                _ => bingo_sdk::slot::mode_label(&current, &modes).to_owned(),
            };
            Some(Mode {
                value: current,
                label,
            })
        })
        .unwrap_or_else(|| Mode {
            value: "default".into(),
            label: "default".into(),
        })
}
impl Mode {
    pub fn ink(&self) -> &'static str {
        match self.value.as_str() {
            "acceptEdits" => "auto",
            "plan" => "plan",
            "bypassPermissions" => "bad",
            _ => "muted",
        }
    }
    pub fn description(&self) -> &'static str {
        match self.value.as_str() {
            "acceptEdits" => "edits inside the working directories run without a prompt",
            "plan" => "nothing that is not read-only runs at all",
            "bypassPermissions" => "everything runs except what only a person may decide",
            "dontAsk" => "tools that need permission are denied without a prompt",
            _ => "trusted read-only tools run; everything else asks",
        }
    }
    pub fn flash_ink(&self) -> &'static str {
        if self.ink() == "muted" {
            "fg"
        } else {
            self.ink()
        }
    }
}

/// A mode acknowledgement already owns its clock; the edge owns no timer.
pub fn changed(state: &SessionState, ui: &Ui, now: Now) -> Option<std::time::Duration> {
    let notice = ui.notice()?;
    let label = notice.text.strip_prefix("permission mode: ")?;
    let mode = current(state);
    let matches =
        label == mode.label || label == "bypass permissions" && mode.value == "bypassPermissions";
    (matches && !label.contains('\n')).then(|| now.since(notice.said()))
}
