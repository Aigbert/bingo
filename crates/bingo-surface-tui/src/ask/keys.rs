//! Keys on a permission or a confirmation in the dock: the arrows pick an
//! answer, enter gives it, and any letter starts the no that says why.
use super::spec::{Ask, Does};
use crate::{clock::Now, composer::Composer, effect::Effect, ui::Ui};
use bingo_sdk::{Activation, Answer, Interaction, InteractionKind, Level, pick, view::TextChild};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn on_key(
    ui: &mut Ui,
    interaction: &Interaction,
    ask: &Ask,
    key: KeyEvent,
    now: Now,
) -> Vec<Effect> {
    if ui.dialog.words.is_some() {
        return typing(ui, interaction, ask, key);
    }
    let last = ask.options.len().saturating_sub(1);
    match key.code {
        KeyCode::Up => ui.dialog.focus = ui.dialog.focus.saturating_sub(1),
        KeyCode::Down => ui.dialog.focus = (ui.dialog.focus + 1).min(last),
        KeyCode::Enter => return choose(ui, interaction, ask, now),
        KeyCode::Char(c) if typed(key) => {
            if let Some(no) = ask.options.iter().position(|o| o.does == Does::Deny) {
                let mut words = Composer::default();
                words.insert(&c.to_string());
                ui.dialog.focus = no;
                ui.dialog.words = Some(words);
            }
        }
        _ => {}
    }
    Vec::new()
}

/// Leave the words row and go back to the list, keeping the pick.
pub fn back(ui: &mut Ui) -> bool {
    ui.dialog.changing = None;
    ui.dialog.words.take().is_some()
}

fn choose(ui: &mut Ui, interaction: &Interaction, ask: &Ask, now: Now) -> Vec<Effect> {
    let Some(option) = ask.options.get(ui.dialog.focus) else {
        return Vec::new();
    };
    match &option.does {
        Does::Send(answer) => {
            if let (Answer::AllowSession { .. }, Some(scope)) = (answer, &option.scope) {
                allowed(ui, scope, now);
            }
            send(ui, interaction, answer.clone())
        }
        Does::Deny => {
            ui.dialog.words = Some(Composer::default());
            Vec::new()
        }
        Does::Change => {
            let InteractionKind::Permission { input, .. } = &interaction.kind else {
                return Vec::new();
            };
            let mut words = Composer::default();
            words.set(changed_text(input).unwrap_or_default());
            ui.dialog.words = Some(words);
            ui.dialog.changing = Some(input.clone());
            Vec::new()
        }
    }
}

fn typing(ui: &mut Ui, interaction: &Interaction, ask: &Ask, key: KeyEvent) -> Vec<Effect> {
    let Some(words) = ui.dialog.words.as_mut() else {
        return Vec::new();
    };
    match key.code {
        KeyCode::Enter => {}
        KeyCode::Backspace => words.backspace(),
        KeyCode::Left => words.left(),
        KeyCode::Right => words.right(),
        KeyCode::Char(c) if typed(key) => words.insert(&c.to_string()),
        _ => {}
    }
    if key.code != KeyCode::Enter {
        return Vec::new();
    }
    let text = words.text().trim().to_owned();
    let changing = ask.options.get(ui.dialog.focus).map(|o| &o.does) == Some(&Does::Change);
    match ui.dialog.changing.clone().filter(|_| changing) {
        Some(_) if text.is_empty() => {
            back(ui);
            Vec::new()
        }
        Some(mut input) => {
            if let Some(field) = changed_field(&input) {
                input[field] = serde_json::Value::String(text);
            }
            send(ui, interaction, Answer::AllowChanged { input })
        }
        None => send(
            ui,
            interaction,
            Answer::Deny {
                feedback: (!text.is_empty()).then_some(text),
            },
        ),
    }
}

/// The toast that says what a session answer let through.
fn allowed(ui: &mut Ui, scope: &str, now: Now) {
    let ok = format!("{} ", crate::theme::glyphs::get("ok"));
    let said = pick::line([
        pick::text(ok, "good"),
        pick::text(format!("allowed {scope} for this session"), "muted"),
    ]);
    ui.notify_parts(
        Level::Info,
        vec![TextChild::Node(Box::new(said))],
        now.instant,
    );
}

fn send(ui: &mut Ui, interaction: &Interaction, answer: Answer) -> Vec<Effect> {
    ui.dialog.words = None;
    ui.dialog.changing = None;
    ui.dialog.answered = true;
    vec![Effect::Answer {
        interaction: interaction.id.clone(),
        answer,
        activation: Activation::Keyboard,
    }]
}

/// The field a change replaces: the command, or the new text of an edit.
fn changed_field(input: &serde_json::Value) -> Option<&'static str> {
    ["command", "new_string"]
        .into_iter()
        .find(|field| input[*field].is_string())
}

fn changed_text(input: &serde_json::Value) -> Option<&str> {
    input[changed_field(input)?].as_str()
}

/// A bare letter, not a chord.
fn typed(key: KeyEvent) -> bool {
    (key.modifiers - KeyModifiers::SHIFT).is_empty()
}
