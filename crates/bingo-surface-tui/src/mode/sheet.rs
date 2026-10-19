//! Modes and their meanings come from the policy's own description.
use crate::{
    clock::Now,
    effect::Effect,
    tree::Tree,
    ui::{Open, Ui},
};
use bingo_sdk::{pick, sheet::Sheet, slot::Slot, *};
use crossterm::event::{KeyCode, KeyEvent};
const LABEL_COLUMNS: usize = 15;

fn policy(state: &SessionState) -> Option<(String, String, Vec<ActionItem>)> {
    slot::described(state)
        .into_iter()
        .find_map(|placed| match placed.slot {
            Slot::Mode { current, modes } => Some((placed.plugin, current, modes)),
            _ => None,
        })
}
fn identity(item: &ActionItem) -> String {
    item.action.args.as_str().unwrap_or(&item.label).into()
}
pub fn open(ui: &mut Ui, state: &SessionState, now: Now) {
    let selected = policy(state).map(|(_, current, _)| current);
    ui.focus.show(Open::Mode(selected), now.instant);
}
pub fn sheet(state: &SessionState, selected: Option<&str>, width: u16, height: u16) -> View {
    let (_, current, modes) = policy(state).unwrap_or_default();
    let selected = selected.unwrap_or(&current);
    let rows = rows(&modes, selected, &current);
    let foot = foot(state);
    Sheet {
        head: "Permission mode".into(),
        fact: Some(pick::text(
            format!("{} cycles", crate::theme::glyphs::key_text("⇧tab")),
            "dim",
        )),
        top: vec![],
        rows,
        selected: Some(selected.into()),
        foot,
        pick: true,
    }
    .with_limit(width, height, 8, crate::theme::glyphs::set())
}
fn rows(modes: &[ActionItem], selected: &str, current: &str) -> Vec<pick::Row> {
    modes
        .iter()
        .map(|item| {
            let id = identity(item);
            let ink = super::Mode {
                value: id.clone(),
                label: item.label.clone(),
                meaning: item.meaning.clone(),
            }
            .ink();
            let label = &item.label;
            let left = pick::line([
                if id == selected {
                    pick::pad(pick::bold(label, ink), LABEL_COLUMNS)
                } else {
                    pick::pad(pick::text(label, "muted"), LABEL_COLUMNS)
                },
                pick::text(item.meaning.as_deref().unwrap_or_default(), "dim"),
            ]);
            let mut row = pick::Row::new(&id, left);
            if id == current {
                row.right = Some(pick::text("current", "muted"));
            }
            row
        })
        .collect()
}
fn foot(state: &SessionState) -> Vec<View> {
    let mut foot = vec![pick::text("allowed for this session", "dim")];
    let rules = state
        .config
        .policy()
        .and_then(|(_, payload)| payload["rules"].as_array());
    if let Some(rules) = rules.filter(|rules| !rules.is_empty()) {
        foot.extend(
            rules
                .iter()
                .filter_map(|rule| rule.as_str())
                .map(|rule| pick::text(format!("  {rule}"), "code")),
        );
    } else {
        foot.push(pick::text(
            format!(
                "  none yet; \"Yes, allow {} for this session\" adds one",
                crate::theme::glyphs::get("ell")
            ),
            "dim",
        ));
    }
    foot
}
pub fn keys(ui: &mut Ui, tree: &Tree, key: KeyEvent, now: Now) -> Vec<Effect> {
    let Some((_, current, modes)) = policy(tree.viewed()) else {
        ui.focus.close(now.instant);
        return vec![];
    };
    let Some(Open::Mode(selected)) = ui.focus.open_mut() else {
        return vec![];
    };
    match key.code {
        KeyCode::Up | KeyCode::Down => {
            let rows = rows(&modes, selected.as_deref().unwrap_or(&current), &current);
            *selected = pick::moved(
                &rows,
                selected.as_deref().or(Some(&current)),
                key.code == KeyCode::Down,
            );
        }
        KeyCode::Enter => return select(ui, tree, now),
        KeyCode::Esc => ui.focus.close(now.instant),
        _ => {}
    }
    vec![]
}

pub fn select(ui: &mut Ui, tree: &Tree, now: Now) -> Vec<Effect> {
    let Some((plugin, current, modes)) = policy(tree.viewed()) else {
        return vec![];
    };
    let Open::Mode(selected) = ui.focus.open() else {
        return vec![];
    };
    let input = modes
        .iter()
        .find(|item| identity(item) == selected.as_deref().unwrap_or(&current))
        .or_else(|| modes.first())
        .map(|item| crate::input::mode::mode_step(&plugin, item.action.clone()));
    ui.focus.close(now.instant);
    input
        .map(|input| vec![Effect::CycleMode(input)])
        .unwrap_or_default()
}
