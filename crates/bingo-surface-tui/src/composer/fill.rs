//! The one client action that puts text in the composer: a status row's
//! command to finish (M170) and a rewound prompt (M177) both arrive here.

use bingo_sdk::Level;
use serde::Deserialize;

use crate::{clock::Now, focus::Focus, ui::Ui};

/// `{"name":"Composer","args":{"text":…,"say":…}}`: text the person finishes
/// and sends, and what they are told about finishing it (`/seat ` and
/// `seat who?`).
#[derive(Debug, Deserialize)]
pub struct Fill {
    pub text: String,
    #[serde(default)]
    pub say: Option<bingo_sdk::toast::Say>,
}

/// A draft the person is writing is never overwritten: the fill is refused
/// and the notice names what was not put in.
pub fn fill(ui: &mut Ui, fill: Fill, now: Now) {
    if !ui.composer.is_empty() {
        ui.notify(Level::Info, kept(&fill.text), now.instant);
        return;
    }
    ui.composer.set(&fill.text);
    ui.edited();
    match ui.focus {
        Focus::Layer(_) => ui.focus.close(now.instant),
        _ => ui.focus = Focus::Composer,
    }
    if let Some(said) = fill.say {
        ui.tell(said, now.instant);
    }
}

fn kept(text: &str) -> String {
    let mut lines = text.trim().lines();
    let first = lines.next().unwrap_or_default();
    let more = if lines.next().is_some() { " …" } else { "" };
    format!("composer has a draft; clear it to use {first}{more}")
}
