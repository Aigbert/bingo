//! Mode words and inks read from the policy's placement.
pub mod sheet;
use crate::{clock::Now, ui::Ui};
use bingo_sdk::{SessionState, slot::Slot};

pub struct Mode {
    pub value: String,
    pub label: String,
    pub meaning: Option<String>,
}
pub fn current(state: &SessionState) -> Mode {
    bingo_sdk::slot::placed(state)
        .into_iter()
        .find_map(|placed| {
            let Slot::Mode { current, modes } = placed.slot else {
                return None;
            };
            let item = bingo_sdk::slot::mode_at(&current, &modes).and_then(|at| modes.get(at));
            let label = bingo_sdk::slot::mode_label(&current, &modes).to_owned();
            let meaning = item.and_then(|item| item.meaning.clone());
            Some(Mode {
                value: current,
                label,
                meaning,
            })
        })
        .unwrap_or_else(|| Mode {
            value: "default".into(),
            label: "default".into(),
            meaning: None,
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
    pub fn flash_ink(&self) -> &'static str {
        if self.ink() == "muted" {
            "fg"
        } else {
            self.ink()
        }
    }
}

/// A mode acknowledgement already owns its clock; the edge owns no timer.
pub fn changed(ui: &Ui, now: Now) -> Option<std::time::Duration> {
    let notice = ui.notice()?;
    matches!(notice.kind, crate::ui::NoticeKind::Mode).then(|| now.since(notice.said()))
}
